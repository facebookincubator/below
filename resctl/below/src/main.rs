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
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Error, Result};
use slog::{self, debug, error};
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
mod dateutil;
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
use below_thrift::DataFrame;

#[derive(Debug, StructOpt)]
#[structopt(no_version)]
struct Opt {
    #[structopt(long, parse(from_os_str), default_value = "/var/log/below")]
    log_dir: PathBuf,
    #[structopt(short, long)]
    debug: bool,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Display live system data (interactive)
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
    },
    /// Replay historical data (interactive)
    Replay {
        /// Time string specifying the replay starting point, e.g. "1 day ago"
        ///
        /// Keywords: now, today, yesterday
        ///
        /// Relative: {humantime} ago, e.g. "2 days 3 hr 15m 10sec ago"
        ///
        /// Absolute: "Jan 01 23:59", "01/01/1970 11:59PM", "1970-01-01 23:59:59"
        #[structopt(short, long)]
        time: String,
        /// Supply hostname to activate remote viewing
        #[structopt(long)]
        host: Option<String>,
        /// Override default port to connect remote viewing to
        #[structopt(long)]
        port: Option<u16>,
    },
}

// Whether or not to start a service to respond to network request
// (e.g. for stats collection or otherwise)
pub enum Service {
    On(Option<u16>),
    Off,
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

fn run<F>(init: init::InitToken, opts: Opt, service: Service, command: F) -> i32
where
    F: FnOnce(slog::Logger, Receiver<Error>) -> Result<()>,
{
    let (err_sender, err_receiver) = channel();
    let mut log_dir = opts.log_dir.clone();
    log_dir.push("error.log");

    if let Err(e) = create_log_dir(&opts.log_dir) {
        eprintln!("{}", e);
        return 1;
    }

    let logger = logging::setup(init, log_dir, opts.debug);
    setup_log_on_panic(logger.clone());
    #[cfg(fbcode_build)]
    facebook::init(
        init,
        logger.clone(),
        service,
        opts.log_dir.clone().join("store"),
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

    let rc = match opts.cmd {
        Command::Live { interval_s } => run(init, opts, Service::Off, |logger, _errs| {
            live(logger, Duration::from_secs(interval_s as u64))
        }),
        Command::Record {
            interval_s,
            retain_for_s,
            collect_io_stat,
            port,
        } => {
            logutil::set_current_log_target(logutil::TargetLog::Term);
            let log_dir = opts.log_dir.clone();
            run(init, opts, Service::On(port), |logger, errs| {
                record(
                    logger,
                    errs,
                    Duration::from_secs(interval_s as u64),
                    log_dir,
                    retain_for_s.map(|r| Duration::from_secs(r as u64)),
                    collect_io_stat,
                )
            })
        }
        Command::Replay {
            ref time,
            ref host,
            ref port,
        } => {
            let log_dir = opts.log_dir.clone();
            let time = time.clone();
            let host = host.clone();
            let port = port.clone();
            run(init, opts, Service::Off, |logger, _errs| {
                replay(logger, time, log_dir, host, port)
            })
        }
    };
    exit(rc);
}

fn replay(
    logger: slog::Logger,
    time: String,
    mut dir: PathBuf,
    host: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let timestamp = UNIX_EPOCH
        + Duration::from_secs(
            dateutil::HgTime::parse(&time)
                .ok_or(anyhow!("Unrecognized timestamp format"))?
                .unixtime,
        );

    let mut advance = if let Some(host) = host {
        Advance::new_with_remote(logger.clone(), host, port, timestamp)?
    } else {
        dir.push("store");
        Advance::new(logger.clone(), dir, timestamp)
    };

    // Fill the last_sample for forward iteration. If no previous sample exists,
    // this should have no effect.
    advance.initialize();
    let mut view = match advance.advance(store::Direction::Forward) {
        Some(model) => view::View::new(model),
        None => return Err(anyhow!("No initial model could be found!")),
    };
    view.register_advance(advance);
    logutil::set_current_log_target(logutil::TargetLog::File);
    view.run()
}

fn record(
    logger: slog::Logger,
    errs: Receiver<Error>,
    interval: Duration,
    mut dir: PathBuf,
    retain: Option<Duration>,
    collect_io_stat: bool,
) -> Result<()> {
    debug!(logger, "Starting up!");

    dir.push("store");
    let mut store = store::StoreWriter::new(&dir)?;
    let mut stats = statistics::Statistics::new();

    loop {
        // Anything that comes over the error channel is an error
        match errs.try_recv() {
            Ok(e) => bail!(e),
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => bail!("error channel disconnected"),
        };

        let collect_instant = Instant::now();
        match collect_sample(collect_io_stat) {
            Ok(s) => {
                if let Err(e) = store.put(SystemTime::now(), &DataFrame { sample: s }) {
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

/// Live mode - gather data and display but do not record
fn live(logger: slog::Logger, interval: Duration) -> Result<()> {
    let mut collector = model::Collector::new();
    let mut view = view::View::new(collector.update_model()?);

    let sink = view.cb_sink().clone();

    thread::spawn(move || {
        loop {
            thread::sleep(interval);
            let res = collector.update_model();
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
