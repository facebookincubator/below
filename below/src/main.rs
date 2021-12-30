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

#![cfg_attr(feature = "enable_backtrace", feature(backtrace))]
#![recursion_limit = "256"]

use std::cell::RefCell;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::exit;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, bail, Context, Error, Result};
use cursive::Cursive;
use indicatif::ProgressBar;
use regex::Regex;
use signal_hook::iterator::Signals;
use slog::{self, debug, error, warn};
use structopt::{
    clap::{AppSettings, Shell},
    StructOpt,
};
use users::{get_current_uid, get_user_by_uid};

mod exitstat;
#[cfg(test)]
mod test;

use common::{cliutil, logutil, open_source_shim};
use config::BelowConfig;
use dump::DumpCommand;
use model;
use store::advance::{new_advance_local, new_advance_remote};
use store::{self, DataFrame, Store};
use view::ViewState;

open_source_shim!();

static LIVE_REMOTE_MAX_LATENCY_SEC: u64 = 10;

#[derive(Debug, StructOpt)]
#[structopt(no_version)]
struct Opt {
    #[structopt(long, parse(from_os_str), default_value = config::BELOW_DEFAULT_CONF)]
    config: PathBuf,
    #[structopt(short, long)]
    debug: bool,
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(flatten)]
    External(commands::Command),
    /// Display live system data (interactive) (default)
    Live {
        #[structopt(short, long, default_value = "5")]
        interval_s: u64,
        /// Supply hostname to activate remote viewing
        #[structopt(long)]
        host: Option<String>,
        /// Override default port to connect remote viewing to
        #[structopt(long)]
        port: Option<u16>,
    },
    /// Record local system data (daemon mode)
    Record {
        #[structopt(short, long, default_value = "5")]
        interval_s: u64,
        /// Store retention in seconds. Data is stored in 24 hour shards.
        /// Whever an entire shard of data is outside the retention period it
        /// is discarded. That is, any data older than retention + 24 hours
        /// is guaranteed to be discarded.
        ///
        /// N.B. If --store-size-limit is set, data may be discarded earlier
        ///      than the specified retention.
        #[structopt(long)]
        retain_for_s: Option<u64>,
        /// Store size limit in bytes. Data is stored in 24 hour shards.
        /// Shards before the active shard are deleted, oldest first,
        /// according to the size limit. Enforcement is only triggered on new
        /// shard creation.
        ///
        /// N.B. Since the active shard cannot be deleted, the size limit may
        ///      be exceeded by a single active shard.
        #[structopt(long)]
        store_size_limit: Option<u64>,
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
        /// Flag to disable disk_stat collection.
        #[structopt(long)]
        disable_disk_stat: bool,
        /// Flag to disable eBPF-based exitstats
        #[structopt(long)]
        disable_exitstats: bool,
        /// Enable data file compression
        ///
        /// You can expect up to ~4.5x smaller data files
        #[structopt(long)]
        compress: bool,
    },
    /// Replay historical data (interactive)
    Replay {
        /// Time string specifying the replay starting point, e.g. "1 day ago"{n}
        /// Keywords: now, today, yesterday{n}
        /// Relative: {humantime} ago, e.g. "2 days 3 hr 15m 10sec ago"{n}
        /// Relative short: Mixed {time_digit}{time_unit_char} E.g. 10m, 3d2h, 5h30s. Case insensitive.{n}
        /// Absolute: "Jan 01 23:59", "01/01/1970 11:59PM", "1970-01-01 23:59:59"{n}
        /// Unix Epoch: 1589808367
        /// _
        #[structopt(short, long, verbatim_doc_comment)]
        time: String,
        /// Supply hostname to activate remote viewing
        #[structopt(long)]
        host: Option<String>,
        /// Override default port to connect remote viewing to
        #[structopt(long)]
        port: Option<u16>,
        /// Days adjuster: y[y...] for yesterday (repeated).
        /// Each "y" will deduct 1 day from the input of "--time/-t"{n}
        /// Examples:
        /// * Yesterday at 2 pm: below replay -r y -t 2:00pm
        /// * 09/01/2020 17:00: below replay -r yy -t "09/03/2020 17:00"
        #[structopt(short = "r", verbatim_doc_comment)]
        yesterdays: Option<String>,
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
        cmd: DumpCommand,
    },
    /// Generate a shell completions file
    #[structopt(setting = AppSettings::Hidden)]
    GenerateCompletions {
        /// The shell type
        #[structopt(short, long, default_value = "bash")]
        shell: Shell,
        /// Output file, stdout if not present
        #[structopt(short, long, parse(from_os_str))]
        output: Option<PathBuf>,
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
    /// Convert frames from an existing store and write them to a new store.
    /// This can be used to test compression/serialization formats.
    ConvertStore {
        #[structopt(short, long, verbatim_doc_comment)]
        start_time: String,
        #[structopt(short, long, verbatim_doc_comment)]
        end_time: String,
        #[structopt(long)]
        from_store_dir: Option<PathBuf>,
        #[structopt(long)]
        to_store_dir: PathBuf,
        #[structopt(long)]
        host: Option<String>,
        #[structopt(long)]
        port: Option<u16>,
        #[structopt(long)]
        compress: bool,
    },
}

// Whether or not to start a service to respond to network request
// (e.g. for stats collection or otherwise)
pub enum Service {
    On(Option<u16>),
    Off,
}

// Whether or not to redirect log to stderr on fs failure
#[derive(PartialEq)]
pub enum RedirectLogOnFail {
    On,
    Off,
}

fn bump_memlock_rlimit() -> Result<()> {
    // TODO(T78976996) remove the fbcode_gate once we can exit stats is
    // enabled for opensource
    if cfg!(fbcode_build) {
        let rlimit = libc::rlimit {
            rlim_cur: 128 << 20,
            rlim_max: 128 << 20,
        };

        if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
            bail!("Failed to increase rlimit");
        }
    }

    Ok(())
}

fn create_log_dir(path: &PathBuf) -> Result<()> {
    if path.exists() && !path.is_dir() {
        bail!("{} exists and is not a directory", path.to_string_lossy());
    }

    if !path.is_dir() {
        match fs::create_dir_all(path) {
            Ok(()) => {}
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
            Ok(()) => {}
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
) -> (Arc<Mutex<procfs::PidMap>>, Option<Receiver<Error>>) {
    let mut exit_driver = exitstat::ExitstatDriver::new(logger, debug);
    let exit_buffer = exit_driver.get_buffer();
    let (bpf_err_send, bpf_err_recv) = channel();
    thread::spawn(move || {
        match exit_driver.drive() {
            Ok(_) => {}
            Err(e) => bpf_err_send.send(e).unwrap(),
        };
    });

    (exit_buffer, Some(bpf_err_recv))
}

/// Returns true if other end disconnected, false otherwise
fn check_for_exitstat_errors(logger: &slog::Logger, receiver: &Receiver<Error>) -> bool {
    // Print an error but don't exit on bpf issues. Do this b/c we can't always
    // be sure what kind of kernel we're running on and if it's new enough.
    match receiver.try_recv() {
        Ok(e) => error!(logger, "{:#}", e),
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            warn!(logger, "bpf error channel disconnected");
            return true;
        }
    };

    false
}

/// Discard old data shards in store according to store size limit and retention
fn cleanup_store(
    store: &store::StoreWriter,
    logger: &slog::Logger,
    store_size_limit: Option<u64>,
    retention: Option<Duration>,
) -> Result<()> {
    if let Some(limit) = store_size_limit {
        if !store
            .try_discard_until_size(limit)
            .context("Failed to discard earlier data")?
        {
            warn!(
                logger,
                "Failed to limit store size since the current shard is \
                greater than the limit"
            );
        }
    }
    if let Some(retention) = retention {
        store
            .discard_earlier(SystemTime::now() - retention)
            .context("Failed to discard earlier data")?;
    }
    Ok(())
}

/// Special Error that indicates the program should stop now. It represents an
/// actual signal, e.g. SIGINT, SIGTERM, that is handled by below and thus below
/// can shutdown gracefully.
#[derive(Clone, Debug)]
struct StopSignal {
    signal: i32,
}

impl std::error::Error for StopSignal {}

impl std::fmt::Display for StopSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stopped by signal: {}", self.signal)
    }
}

pub fn run<F>(
    init: init::InitToken,
    debug: bool,
    below_config: &BelowConfig,
    _service: Service,
    redirect: RedirectLogOnFail,
    command: F,
) -> i32
where
    F: FnOnce(init::InitToken, &BelowConfig, slog::Logger, Receiver<Error>) -> Result<()>,
{
    let (err_sender, err_receiver) = channel();
    let mut log_dir = below_config.log_dir.clone();
    let user = get_user_by_uid(get_current_uid()).expect("Failed to get current user for logging");

    log_dir.push(format!("error_{}.log", user.name().to_string_lossy()));

    if let Err(e) = create_log_dir(&below_config.log_dir) {
        eprintln!("{:#}", e);
        return 1;
    }

    let logger = logging::setup(init, log_dir, debug, redirect);
    setup_log_on_panic(logger.clone());

    match Signals::new(&[signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM]) {
        Ok(mut signals) => {
            let logger = logger.clone();
            let err_sender = err_sender.clone();
            thread::spawn(move || {
                let mut term_now = false;
                for signal in signals.forever() {
                    if term_now {
                        error!(logger, "Below didn't stop in time. Terminate now!");
                        std::process::exit(1);
                    }
                    term_now = true;
                    error!(logger, "Stop signal received: {}, exiting.", signal);
                    err_sender.send(anyhow!(StopSignal { signal })).unwrap();
                }
            });
        }
        Err(e) => {
            error!(logger, "{:#}", e);
            return 1;
        }
    }

    #[cfg(fbcode_build)]
    facebook::init(
        init,
        logger.clone(),
        _service,
        below_config.store_dir.clone(),
        err_sender,
    );
    let res = command(init, below_config, logger.clone(), err_receiver);

    match res {
        Ok(_) => 0,
        Err(e) if e.is::<StopSignal>() => {
            error!(logger, "{:#}", e);
            0
        }
        Err(e) => {
            if logutil::get_current_log_target() == logutil::TargetLog::File {
                logutil::set_current_log_target(logutil::TargetLog::All);
            }
            error!(
                logger,
                "\n\
                ----------------- Detected unclean exit ---------------------\n\
                Error Message: {:#}\n\
                -------------------------------------------------------------",
                e
            );
            1
        }
    }
}

#[cfg(fbcode_build)]
#[fbinit::main(disable_fatal_signals = all)]
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
    config::BELOW_CONFIG
        .set(match BelowConfig::load(&opts.config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{:#}", e);
                exit(1);
            }
        })
        .expect("BELOW_CONFIG singleton set twice");
    let below_config = config::BELOW_CONFIG
        .get()
        .expect("BELOW_CONFIG empty after set");

    // Use live mode as default
    let cmd = opts.cmd.as_ref().unwrap_or(&Command::Live {
        interval_s: 5,
        host: None,
        port: None,
    });
    let rc = match cmd {
        Command::External(command) => commands::run_command(init, debug, below_config, command),
        Command::Live {
            ref interval_s,
            ref host,
            ref port,
        } => {
            let host = host.clone();
            let port = port.clone();
            run(
                init,
                debug,
                below_config,
                Service::Off,
                RedirectLogOnFail::On,
                |_, below_config, logger, errs| {
                    live(
                        logger,
                        errs,
                        Duration::from_secs(*interval_s as u64),
                        debug,
                        below_config,
                        host,
                        port,
                    )
                },
            )
        }
        Command::Record {
            ref interval_s,
            ref retain_for_s,
            ref store_size_limit,
            ref collect_io_stat,
            ref port,
            ref skew_detection_threshold_ms,
            ref disable_disk_stat,
            ref disable_exitstats,
            ref compress,
        } => {
            logutil::set_current_log_target(logutil::TargetLog::Term);
            run(
                init,
                debug,
                below_config,
                Service::On(*port),
                RedirectLogOnFail::Off,
                |_, below_config, logger, errs| {
                    record(
                        logger,
                        errs,
                        Duration::from_secs(*interval_s as u64),
                        below_config,
                        retain_for_s.map(|r| Duration::from_secs(r as u64)),
                        *store_size_limit,
                        *collect_io_stat,
                        Duration::from_millis(*skew_detection_threshold_ms),
                        debug,
                        *disable_disk_stat,
                        *disable_exitstats,
                        *compress,
                    )
                },
            )
        }
        Command::Replay {
            ref time,
            ref host,
            ref port,
            ref yesterdays,
        } => {
            let time = time.clone();
            let host = host.clone();
            let port = port.clone();
            let days_adjuster = yesterdays.clone();
            run(
                init,
                debug,
                below_config,
                Service::Off,
                RedirectLogOnFail::Off,
                |_, below_config, logger, errs| {
                    replay(logger, errs, time, below_config, host, port, days_adjuster)
                },
            )
        }
        Command::Debug { ref cmd } => match cmd {
            DebugCommand::DumpStore { ref time, ref json } => {
                let time = time.clone();
                let json = json.clone();
                run(
                    init,
                    debug,
                    below_config,
                    Service::Off,
                    RedirectLogOnFail::Off,
                    |_, below_config, logger, _errs| dump_store(logger, time, below_config, json),
                )
            }
            DebugCommand::ConvertStore {
                ref start_time,
                ref end_time,
                ref from_store_dir,
                ref to_store_dir,
                ref host,
                ref port,
                ref compress,
            } => {
                let start_time = start_time.clone();
                let end_time = end_time.clone();
                let from_store_dir = from_store_dir.clone();
                let to_store_dir = to_store_dir.clone();
                let host = host.clone();
                let port = port.clone();
                run(
                    init,
                    debug,
                    below_config,
                    Service::Off,
                    RedirectLogOnFail::Off,
                    |_, below_config, logger, _errs| {
                        convert_store(
                            logger,
                            below_config,
                            start_time,
                            end_time,
                            from_store_dir,
                            to_store_dir,
                            host,
                            port,
                            *compress,
                        )
                    },
                )
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
            run(
                init,
                debug,
                below_config,
                Service::Off,
                RedirectLogOnFail::Off,
                |_, _below_config, logger, errs| {
                    dump::run(logger, errs, store_dir, host, port, cmd)
                },
            )
        }
        Command::GenerateCompletions {
            ref shell,
            ref output,
        } => {
            generate_completions(shell.clone(), output.clone())
                .unwrap_or_else(|_| panic!("Failed to generate completions for {:?}", shell));
            0
        }
    };
    exit(rc);
}

fn replay(
    logger: slog::Logger,
    errs: Receiver<Error>,
    time: String,
    below_config: &BelowConfig,
    host: Option<String>,
    port: Option<u16>,
    days_adjuster: Option<String>,
) -> Result<()> {
    let timestamp =
        cliutil::system_time_from_date_and_adjuster(time.as_str(), days_adjuster.as_deref())?;

    let mut advance = if let Some(host) = host {
        new_advance_remote(logger.clone(), host, port, timestamp)?
    } else {
        new_advance_local(logger.clone(), below_config.store_dir.clone(), timestamp)
    };

    // Fill the last_sample for forward iteration. If no previous sample exists,
    // this should have no effect.
    advance.initialize();

    let model = match advance.jump_sample_to(timestamp) {
        Some(m) => m,
        None => bail!(
            "No initial sample could be found!\n\
            You may have provided a time in the future or no data was recorded during the provided time. \
            Please check your input and timezone.\n\
            If you are using remote, please make sure the below service on target host is running."
        ),
    };

    cliutil::check_initial_sample_time_with_requested_time(model.timestamp, timestamp);

    let mut view = view::View::new_with_advance(
        model,
        view::ViewMode::Replay(Rc::new(RefCell::new(advance))),
    );
    logutil::set_current_log_target(logutil::TargetLog::File);

    let sink = view.cb_sink().clone();
    thread::spawn(move || {
        match errs.recv() {
            Ok(e) => {
                error!(logger, "{:#}", e);
            }
            Err(_) => {
                error!(logger, "error channel disconnected");
            }
        };
        sink.send(Box::new(|c| c.quit()))
            .expect("Failed to stop view");
    });

    view.run()
}

fn record(
    logger: slog::Logger,
    errs: Receiver<Error>,
    interval: Duration,
    below_config: &BelowConfig,
    retention: Option<Duration>,
    store_size_limit: Option<u64>,
    collect_io_stat: bool,
    skew_detection_threshold: Duration,
    debug: bool,
    disable_disk_stat: bool,
    disable_exitstats: bool,
    compress: bool,
) -> Result<()> {
    debug!(logger, "Starting up!");

    if !disable_exitstats {
        bump_memlock_rlimit()?;
    }

    let mut store = store::StoreWriter::new(
        logger.clone(),
        &below_config.store_dir,
        compress,
        store::Format::Cbor,
    )?;
    let mut stats = statistics::Statistics::new();

    let (exit_buffer, bpf_errs) = if disable_exitstats {
        (Arc::new(Mutex::new(procfs::PidMap::new())), None)
    } else {
        start_exitstat(logger.clone(), debug)
    };
    let mut bpf_err_warned = false;

    // Handle cgroup filter from conf and generate Regex
    let cgroup_re = if !below_config.cgroup_filter_out.is_empty() {
        Some(
            Regex::new(&below_config.cgroup_filter_out)
                .expect("Failed to generate regex from cgroup_filter_out in below.conf"),
        )
    } else {
        None
    };

    loop {
        if !disable_exitstats {
            // Anything that comes over the error channel is an error
            match errs.try_recv() {
                Ok(e) => bail!(e),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => bail!("error channel disconnected"),
            };

            if !bpf_err_warned {
                bpf_err_warned = check_for_exitstat_errors(
                    &logger,
                    bpf_errs
                        .as_ref()
                        .expect("Failed to unwrap bpf_errs receiver"),
                );
            }
        }

        let collect_instant = Instant::now();

        let collected_sample = model::collect_sample(
            &below_config.cgroup_root,
            &exit_buffer,
            collect_io_stat,
            &logger,
            disable_disk_stat,
            &cgroup_re,
        );
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
                match store.put(post_collect_sys_time, &DataFrame { sample: s }) {
                    Ok(/* new shard */ true) => {
                        cleanup_store(&store, &logger, store_size_limit, /* retention */ None)?
                    }
                    Ok(/* new shard */ false) => {}
                    Err(e) => error!(logger, "{:#}", e),
                }
            }
            Err(e) => {
                // Handle cgroupfs errors
                match e.downcast_ref::<cgroupfs::Error>() {
                    // Unrecoverable error -- below only supports cgroup2
                    Some(cgroupfs::Error::NotCgroup2(_)) => bail!(e),
                    _ => {}
                };

                error!(logger, "{:#}", e);
            }
        };

        // Only check against retention and not size limit. Size limit is only
        // checked on creation of successful write to a new shard.
        cleanup_store(&store, &logger, /* store_size_limit */ None, retention)?;

        stats.report_store_size(below_config.store_dir.as_path());

        let collect_duration = Instant::now().duration_since(collect_instant);
        // Sleep for at least 1s to avoid sample collision
        let sleep_duration = if interval > collect_duration {
            std::cmp::max(Duration::from_secs(1), interval - collect_duration)
        } else {
            Duration::from_secs(1)
        };
        std::thread::sleep(sleep_duration);
    }
}

fn live_local(
    logger: slog::Logger,
    errs: Receiver<Error>,
    interval: Duration,
    debug: bool,
    below_config: &BelowConfig,
) -> Result<()> {
    match bump_memlock_rlimit() {
        Err(e) => {
            warn!(
                logger,
                #"V",
                "Failed to initialize BPF: {}. Data collection will be degraded. \
                You can ignore this warning or try to run with sudo.",
                &e
            );
        }
        _ => {}
    };

    let (exit_buffer, bpf_errs) = start_exitstat(logger.clone(), debug);
    let mut bpf_err_warned = false;

    let mut collector =
        model::Collector::new_with_cgroup_root(below_config.cgroup_root.clone(), exit_buffer);
    logutil::set_current_log_target(logutil::TargetLog::File);
    // Prepare advance obj for pause mode
    let mut adv = new_advance_local(
        logger.clone(),
        below_config.store_dir.clone(),
        SystemTime::now(),
    );
    adv.initialize();
    let mut view = view::View::new_with_advance(
        collector.update_model(&logger)?,
        view::ViewMode::Live(Rc::new(RefCell::new(adv))),
    );

    let sink = view.cb_sink().clone();

    thread::spawn(move || {
        loop {
            if !bpf_err_warned {
                bpf_err_warned = check_for_exitstat_errors(
                    &logger,
                    bpf_errs
                        .as_ref()
                        .expect("Failed to unwrap bpf_errs receiver"),
                );
            }

            // Rely on timeout to guarantee interval between samples
            match errs.recv_timeout(interval) {
                Ok(e) => {
                    error!(logger, "{:#}", e);
                    sink.send(Box::new(|c| c.quit()))
                        .expect("Failed to stop view");
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    error!(logger, "error channel disconnected");
                    sink.send(Box::new(|c| c.quit()))
                        .expect("Failed to stop view");
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {}
            };

            let res = collector.update_model(&logger);
            match res {
                Ok(model) => {
                    // Error only happens if the other side disconnected - just terminate the thread
                    let data_plane = Box::new(move |s: &mut Cursive| {
                        let view_state = s.user_data::<ViewState>().expect("user data not set");

                        // When paused, no need to update model
                        if !view_state.is_paused() {
                            view_state.update(model);
                        }
                    });
                    if sink.send(data_plane).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    error!(logger, "{:#}", e);
                }
            }
        }
    });

    view.run()
}

fn live_remote(
    logger: slog::Logger,
    errs: Receiver<Error>,
    interval: Duration,
    host: String,
    port: Option<u16>,
) -> Result<()> {
    let timestamp = SystemTime::now()
        .checked_sub(Duration::from_secs(LIVE_REMOTE_MAX_LATENCY_SEC))
        .expect("Fail to construct timestamp with latency allowance in live remote.");
    let mut advance = new_advance_remote(logger.clone(), host, port, timestamp)?;

    advance.initialize();
    let mut view = match advance.get_latest_sample() {
        Some(model) => view::View::new_with_advance(
            model,
            view::ViewMode::Live(Rc::new(RefCell::new(advance))),
        ),
        None => return Err(anyhow!("No data could be found!")),
    };

    let sink = view.cb_sink().clone();

    thread::spawn(move || {
        loop {
            // Rely on timeout to guarantee interval between samples
            match errs.recv_timeout(interval) {
                Ok(e) => {
                    error!(logger, "{:#}", e);
                    sink.send(Box::new(|c| c.quit()))
                        .expect("Failed to stop view");
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    error!(logger, "error channel disconnected");
                    sink.send(Box::new(|c| c.quit()))
                        .expect("Failed to stop view");
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {}
            };

            let data_plane = Box::new(move |s: &mut Cursive| {
                let view_state = s.user_data::<ViewState>().expect("user data not set");

                if let view::ViewMode::Live(adv) = view_state.mode.clone() {
                    match adv.borrow_mut().advance(store::Direction::Forward) {
                        Some(data) => view_state.update(data),
                        None => {}
                    }
                }
            });
            if sink.send(data_plane).is_err() {
                return;
            }
        }
    });

    logutil::set_current_log_target(logutil::TargetLog::File);
    view.run()
}

fn live(
    logger: slog::Logger,
    errs: Receiver<Error>,
    interval: Duration,
    debug: bool,
    below_config: &BelowConfig,
    host: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    if let Some(host) = host {
        live_remote(logger, errs, interval, host, port)
    } else {
        live_local(logger, errs, interval, debug, below_config)
    }
}

fn dump_store(
    logger: slog::Logger,
    time: String,
    below_config: &BelowConfig,
    json: bool,
) -> Result<()> {
    let timestamp = cliutil::system_time_from_date(time.as_str())?;

    let (ts, df) = match store::read_next_sample(
        &below_config.store_dir,
        timestamp,
        store::Direction::Forward,
        logger,
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

fn generate_completions(shell: Shell, output: Option<PathBuf>) -> Result<()> {
    let mut file: Box<dyn io::Write> = match output {
        Some(path) => Box::new(fs::File::create(path)?),
        None => Box::new(io::stdout()),
    };
    Opt::clap().gen_completions_to("below", shell, &mut file);
    Ok(())
}

fn convert_store(
    logger: slog::Logger,
    below_config: &BelowConfig,
    start_time: String,
    end_time: String,
    from_store_dir: Option<PathBuf>,
    to_store_dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    compress: bool,
) -> Result<()> {
    let (time_begin, time_end) = cliutil::system_time_range_from_date_and_adjuster(
        start_time.as_str(),
        Some(end_time.as_str()),
        /* days_adjuster */ None,
    )?;
    let (timestamp_begin, timestamp_end) = (
        common::util::get_unix_timestamp(time_begin),
        common::util::get_unix_timestamp(time_end),
    );
    let pb = ProgressBar::new(timestamp_end - timestamp_begin);

    let mut store: Box<dyn Store<SampleType = DataFrame>> = match (from_store_dir, host) {
        (Some(_from_store_dir), Some(_host)) => {
            bail!("Only one of --from-store-dir and --host should be specified");
        }
        (Some(from_store_dir), None) => {
            pb.set_message(&format!("Using local store at {:?}", from_store_dir));
            Box::new(store::LocalStore::new(logger.clone(), from_store_dir))
        }
        (None, Some(host)) => {
            pb.set_message(&format!("Using remote store for {}", host));
            Box::new(store::RemoteStore::new(host, port)?)
        }
        (None, None) => {
            pb.set_message(&format!(
                "Using local store at {:?}",
                &below_config.store_dir
            ));
            Box::new(store::LocalStore::new(
                logger.clone(),
                below_config.store_dir.clone(),
            ))
        }
    };

    let mut dest_store =
        store::StoreWriter::new(logger.clone(), &to_store_dir, compress, store::Format::Cbor)?;

    pb.set_message(&format!("Writing to local store at {:?}", to_store_dir));

    let mut nr_samples = 0;
    let mut cur_time = time_begin;
    while cur_time < time_end {
        match store.get_sample_at_timestamp(cur_time, store::Direction::Forward)? {
            Some((frame_time, frame)) => {
                cur_time = frame_time;
                pb.set_message(&format!("Storing frame at t = {:?}", frame_time));
                dest_store.put(frame_time, &frame)?;
                nr_samples += 1;
            }
            None => {
                pb.set_message(&format!(
                    "Error: Breaking early. Couldn't find any frames after t = {:?}",
                    cur_time
                ));
                break;
            }
        }
        pb.set_position(common::util::get_unix_timestamp(cur_time) - timestamp_begin);
        cur_time += Duration::from_secs(1); // To actually move forward
    }
    pb.set_message(&format!("Done. Logged {} samples.", nr_samples));
    Ok(())
}

#[cfg(feature = "enable_backtrace")]
pub fn get_backtrace() -> impl std::fmt::Display {
    std::backtrace::Backtrace::force_capture()
}

#[cfg(not(feature = "enable_backtrace"))]
pub fn get_backtrace() -> impl std::fmt::Display {
    "Backtrace is not available."
}

fn setup_log_on_panic(logger: slog::Logger) {
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = get_backtrace();

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
