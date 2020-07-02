// Copyright (c) Facebook, Inc. and its affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(backtrace)]
#![feature(never_type)]
#![recursion_limit = "256"]

use std::backtrace::Backtrace;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Error, Result};
use slog::{self, debug, error, warn};
use structopt::StructOpt;

// Shim between facebook types and open source types.
//
// The type interfaces and module hierarchy should be identical on
// both "branches". And since we glob import, all the submodules in
// this crate will inherit our name bindings and can use generic paths,
// eg `crate::logging::setup(..)`.
#[cfg(fbcode_build)]
mod facebook;
#[cfg(fbcode_build)]
use crate::facebook::*;
#[cfg(not(fbcode_build))]
mod open_source;
#[cfg(not(fbcode_build))]
use crate::open_source::*;

mod advance;
mod below_config;
mod bpf;
mod dateutil;
mod dump;
mod logutil;
mod model;
mod store;
#[cfg(test)]
mod test;
mod util;
mod view;

use crate::model::collect_sample;
use crate::view::ViewState;
use advance::Advance;
use below_config::BelowConfig;
use below_thrift::DataFrame;

#[derive(Debug, StructOpt)]
#[structopt(no_version)]
struct Opt {
    #[structopt(long, parse(from_os_str), default_value = below_config::BELOW_DEFAULT_CONF)]
    config: PathBuf,
    #[structopt(short, long)]
    debug: bool,
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Display live system data (interactive) (default)
    Live {
        #[structopt(short, long, default_value = "5")]
        interval_s: u64,
    },
    /// Record local system data (daemon mode)
    Record {
        #[structopt(short, long, default_value = "5")]
        interval_s: u64,
        #[structopt(long)]
        retain_for_s: Option<u64>,
        /// Whether or not to collect io.stat for cgroups which could
        /// be expensive
        #[structopt(long)]
        collect_io_stat: bool,
        /// Override default port for remote viewing server
        #[structopt(long)]
        port: Option<u16>,
        /// Threshold for hold long data collection takes to trigger warnings.
        #[structopt(long, default_value = "500")]
        skew_detection_threshold_ms: u64,
    },
    /// Replay historical data (interactive)
    Replay {
        /// Time string specifying the replay starting point, e.g. "1 day ago"
        ///
        /// Keywords: now, today, yesterday
        ///
        /// Relative: {humantime} ago, e.g. "2 days 3 hr 15m 10sec ago"
        ///
        /// Relative short: Mixed {time_digit}{time_unit_char} E.g. 10m, 3d2h, 5h30s. Case insensitive.
        ///
        /// Absolute: "Jan 01 23:59", "01/01/1970 11:59PM", "1970-01-01 23:59:59"
        ///
        /// Unix Epoch: 1589808367
        #[structopt(short, long)]
        time: String,
        /// Supply hostname to activate remote viewing
        #[structopt(long)]
        host: Option<String>,
        /// Override default port to connect remote viewing to
        #[structopt(long)]
        port: Option<u16>,
    },
    /// Debugging facilities (for development use)
    Debug {
        #[structopt(subcommand)]
        cmd: DebugCommand,
    },
    /// Dump historical data into parseable text format
    Dump {
        /// Supply hostname to activate remote dumping
        #[structopt(long)]
        host: Option<String>,
        /// Override default port to connect remote dumping to
        #[structopt(long)]
        port: Option<u16>,
        #[structopt(subcommand)]
        cmd: dump::DumpCommand,
    },
}

#[derive(Debug, StructOpt)]
enum DebugCommand {
    DumpStore {
        /// Time string to dump data for (same format as Replay mode)
        #[structopt(short, long)]
        time: String,
        /// Pretty print in JSON
        #[structopt(short, long)]
        json: bool,
    },
}

// Whether or not to start a service to respond to network request
// (e.g. for stats collection or otherwise)
pub enum Service {
    On(Option<u16>),
    Off,
}

fn bump_memlock_rlimit() -> Result<()> {
    let rlimit = libc::rlimit {
        rlim_cur: 128 << 20,
        rlim_max: 128 << 20,
    };

    if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
        bail!("Failed to increase rlimit");
    }

    Ok(())
}

fn create_log_dir(path: &PathBuf) -> Result<()> {
    if path.exists() && !path.is_dir() {
        bail!("{} exists and is not a directory", path.to_string_lossy());
    }

    if !path.is_dir() {
        match fs::create_dir_all(path) {
            Ok(()) => (),
            Err(e) => {
                bail!(
                    "Failed to create dir {}: {}\nTry sudo.",
                    path.to_string_lossy(),
                    e
                );
            }
        }
    }

    let dir = fs::File::open(path).unwrap();
    let mut perm = dir.metadata().unwrap().permissions();

    if perm.mode() & 0o777 != 0o777 {
        perm.set_mode(0o777);
        match dir.set_permissions(perm) {
            Ok(()) => (),
            Err(e) => {
                bail!(
                    "Failed to set permissions on {}: {}",
                    path.to_string_lossy(),
                    e
                );
            }
        }
    }

    Ok(())
}

// Exitstat runs a bpf program that hooks into process exit events. This allows below to keep
// track of processes whose lifetimes are shorter than polling interval.
fn start_exitstat(
    logger: slog::Logger,
    debug: bool,
) -> (Arc<Mutex<procfs::PidMap>>, Receiver<Error>) {
    let mut exit_driver = bpf::exitstat::ExitstatDriver::new(logger, debug);
    let exit_buffer = exit_driver.get_buffer();
    let (bpf_err_send, bpf_err_recv) = channel();
    thread::spawn(move || {
        match exit_driver.drive() {
            Ok(_) => (),
            Err(e) => bpf_err_send.send(e).unwrap(),
        };
    });

    (exit_buffer, bpf_err_recv)
}

fn check_for_exitstat_errors(logger: &slog::Logger, receiver: &Receiver<Error>) {
    // Print an error but don't exit on bpf issues. Do this b/c we can't always
    // be sure what kind of kernel we're running on and if it's new enough.
    match receiver.try_recv() {
        Ok(e) => error!(logger, "{}", e),
        Err(TryRecvError::Empty) => (),
        Err(TryRecvError::Disconnected) => error!(logger, "bpf error channel disconnected"),
    };
}

fn run<F>(
    init: init::InitToken,
    debug: bool,
    below_config: BelowConfig,
    service: Service,
    command: F,
) -> i32
where
    F: FnOnce(slog::Logger, Receiver<Error>) -> Result<()>,
{
    let (err_sender, err_receiver) = channel();
    let mut log_dir = below_config.log_dir.clone();
    log_dir.push("error.log");

    if let Err(e) = create_log_dir(&below_config.log_dir) {
        eprintln!("{}", e);
        return 1;
    }

    let logger = logging::setup(init, log_dir, debug);
    setup_log_on_panic(logger.clone());
    #[cfg(fbcode_build)]
    facebook::init(
        init,
        logger.clone(),
        service,
        below_config.store_dir.clone(),
        err_sender,
    );
    let res = command(logger.clone(), err_receiver);

    match res {
        Ok(_) => 0,
        Err(e) => {
            if logutil::get_current_log_target() == logutil::TargetLog::File {
                logutil::set_current_log_target(logutil::TargetLog::All);
            }
            error!(
                logger,
                "\n----------------- Detected unclean exit -------------------\n\
                Error Message: {}\n\
                -------------------------------------------------------------",
                e
            );
            1
        }
    }
}

#[cfg(fbcode_build)]
#[fbinit::main]
fn main(fb: FacebookInit) {
    real_main(init::InitToken { fb })
}

#[cfg(not(fbcode_build))]
fn main() {
    real_main(init::InitToken {})
}

fn real_main(init: init::InitToken) {
    let opts = Opt::from_args();
    let debug = opts.debug;
    let below_config = match BelowConfig::load(&opts.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    // Use live mode as default
    let cmd = opts
        .cmd
        .as_ref()
        .unwrap_or(&Command::Live { interval_s: 5 });
    let rc = match cmd {
        Command::Live { ref interval_s } => {
            run(init, debug, below_config, Service::Off, |logger, _errs| {
                live(logger, Duration::from_secs(*interval_s as u64), debug)
            })
        }
        Command::Record {
            ref interval_s,
            ref retain_for_s,
            ref collect_io_stat,
            ref port,
            ref skew_detection_threshold_ms,
        } => {
            logutil::set_current_log_target(logutil::TargetLog::Term);
            let store_dir = below_config.store_dir.clone();
            run(
                init,
                debug,
                below_config,
                Service::On(*port),
                |logger, errs| {
                    record(
                        logger,
                        errs,
                        Duration::from_secs(*interval_s as u64),
                        store_dir,
                        retain_for_s.map(|r| Duration::from_secs(r as u64)),
                        *collect_io_stat,
                        Duration::from_millis(*skew_detection_threshold_ms),
                        debug,
                    )
                },
            )
        }
        Command::Replay {
            ref time,
            ref host,
            ref port,
        } => {
            let store_dir = below_config.store_dir.clone();
            let time = time.clone();
            let host = host.clone();
            let port = port.clone();
            run(init, debug, below_config, Service::Off, |logger, _errs| {
                replay(logger, time, store_dir, host, port)
            })
        }
        Command::Debug { ref cmd } => match cmd {
            DebugCommand::DumpStore { ref time, ref json } => {
                let time = time.clone();
                let json = json.clone();
                let store_dir = below_config.store_dir.clone();
                run(init, debug, below_config, Service::Off, |logger, _errs| {
                    dump_store(logger, time, store_dir, json)
                })
            }
        },
        Command::Dump {
            ref host,
            ref port,
            ref cmd,
        } => {
            let store_dir = below_config.store_dir.clone();
            let host = host.clone();
            let port = port.clone();
            let cmd = cmd.clone();
            run(init, debug, below_config, Service::Off, |logger, _errs| {
                dump::run(logger, store_dir, host, port, cmd)
            })
        }
    };
    exit(rc);
}

fn replay(
    logger: slog::Logger,
    time: String,
    dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let timestamp = UNIX_EPOCH
        + Duration::from_secs(
            dateutil::HgTime::parse(&time)
                .ok_or_else(|| {
                    anyhow!(
                        "Unrecognized timestamp format\n\
                    Input: {}.\n\
                    Examples:\n\t\
                    Keywords: now, today, yesterday\n\t\
                    Relative: \"{{humantime}} ago\", e.g. 2 days 3 hr 15m 10sec ago\n\t\
                    Relative short: Mixed {{time_digit}}{{time_unit_char}}. E.g. 10m, 3d2H, 5h30s, 10m5h\n\t\
                    Absolute: \"Jan 01 23:59\", \"01/01/1970 11:59PM\", \"1970-01-01 23:59:59\"\n\t\
                    Unix Epoch: 1589808367",
                        &time
                    )
                })?
                .unixtime,
        );

    let mut advance = if let Some(host) = host {
        Advance::new_with_remote(logger.clone(), host, port, timestamp)?
    } else {
        Advance::new(logger.clone(), dir, timestamp)
    };

    // Fill the last_sample for forward iteration. If no previous sample exists,
    // this should have no effect.
    advance.initialize();
    let mut view = match advance.advance(store::Direction::Forward) {
        Some(model) => view::View::new(model),
        None => return Err(anyhow!(
            "No initial model could be found!\n\
            You may have provided a time in the future or no data was recorded during the provided time.\n\
            Please check your input and timezone."
        )),
    };
    view.register_advance(advance);
    logutil::set_current_log_target(logutil::TargetLog::File);
    view.run()
}

fn record(
    logger: slog::Logger,
    errs: Receiver<Error>,
    interval: Duration,
    dir: PathBuf,
    retain: Option<Duration>,
    collect_io_stat: bool,
    skew_detection_threshold: Duration,
    debug: bool,
) -> Result<()> {
    debug!(logger, "Starting up!");

    bump_memlock_rlimit()?;

    let mut store = store::StoreWriter::new(&dir)?;
    let mut stats = statistics::Statistics::new();

    let (exit_buffer, bpf_errs) = start_exitstat(logger.clone(), debug);

    loop {
        // Anything that comes over the error channel is an error
        match errs.try_recv() {
            Ok(e) => bail!(e),
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => bail!("error channel disconnected"),
        };

        check_for_exitstat_errors(&logger, &bpf_errs);

        let collect_instant = Instant::now();
        let collected_sample = collect_sample(&exit_buffer, collect_io_stat, &logger);
        let post_collect_sys_time = SystemTime::now();
        let post_collect_instant = Instant::now();

        let collection_skew = post_collect_instant.duration_since(collect_instant);
        if collection_skew >= skew_detection_threshold {
            warn!(
                logger,
                "data collection took {} ms (>= {} ms)",
                collection_skew.as_millis(),
                skew_detection_threshold.as_millis()
            );

            stats.report_collection_skew();
        }

        match collected_sample {
            Ok(s) => {
                if let Err(e) = store.put(post_collect_sys_time, &DataFrame { sample: s }) {
                    error!(logger, "{}", e);
                }
            }
            Err(e) => error!(logger, "{}", e),
        };

        if let Some(retention) = retain {
            store
                .discard_earlier(SystemTime::now() - retention, logger.clone())
                .context("Failed to discard earlier data")?;
        }

        stats.report_store_size(dir.as_path());

        let collect_duration = Instant::now().duration_since(collect_instant);
        if collect_duration < interval {
            std::thread::sleep(interval - collect_duration);
        }
    }
}

fn live(logger: slog::Logger, interval: Duration, debug: bool) -> Result<()> {
    // TODO Raise warning popup on error here: T69437919
    match bump_memlock_rlimit() {
        Err(e) => {
            error!(logger, "{}", e);
        }
        _ => (),
    };

    let (exit_buffer, bpf_errs) = start_exitstat(logger.clone(), debug);

    let mut collector = model::Collector::new(exit_buffer);
    let mut view = view::View::new(collector.update_model(&logger)?);

    let sink = view.cb_sink().clone();

    thread::spawn(move || {
        loop {
            check_for_exitstat_errors(&logger, &bpf_errs);

            thread::sleep(interval);
            let res = collector.update_model(&logger);
            match res {
                Ok(model) => {
                    // Error only happens if the other side disconnected - just terminate the thread
                    if let Err(_) = sink.send(Box::new(move |s| {
                        s.user_data::<ViewState>().expect("user data not set").model = model;
                    })) {
                        return;
                    }
                }
                Err(e) => {
                    error!(logger, "{}", e);
                }
            }
        }
    });

    logutil::set_current_log_target(logutil::TargetLog::File);
    view.run()
}

fn dump_store(logger: slog::Logger, time: String, path: PathBuf, json: bool) -> Result<()> {
    let timestamp = UNIX_EPOCH
        + Duration::from_secs(
            dateutil::HgTime::parse(&time)
                .ok_or(anyhow!("Unrecognized timestamp format"))?
                .unixtime,
        );

    let (ts, df) = match store::read_next_sample(
        &path,
        timestamp,
        store::Direction::Forward,
        logger.clone(),
    ) {
        Ok(Some((ts, df))) => (ts, df),
        Ok(None) => bail!("Data not found for requested timestamp"),
        Err(e) => bail!(e),
    };

    if ts != timestamp {
        bail!("Exact requested timestamp not found (would have used next datapoint)");
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&df)?);
    } else {
        println!("{:#?}", df);
    }

    Ok(())
}

fn setup_log_on_panic(logger: slog::Logger) {
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::force_capture();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Unknown panic object",
            },
        };

        match info.location() {
            Some(location) => {
                error!(
                    logger,
                    "panic '{}': {}:{}\n{}",
                    msg,
                    location.file(),
                    location.line(),
                    backtrace
                );
            }
            None => {
                error!(logger, "panic '{}'\n{}", msg, backtrace);
            }
        }
    }));
}
