#![feature(backtrace)]
#![feature(never_type)]
#![recursion_limit = "256"]

use std::backtrace::Backtrace;
use std::path::PathBuf;
use std::process::exit;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Error, Result};
use slog::{self, error, info};
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
    Live {
        #[structopt(short, long, default_value = "5")]
        interval_s: u64,
    },
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

// Where to log
pub enum LogTo {
    File,
    Stderr,
}

fn run<F>(init: init::InitToken, opts: Opt, log_to: LogTo, service: Service, command: F) -> i32
where
    F: FnOnce(slog::Logger, Receiver<Error>) -> Result<()>,
{
    let (err_sender, err_receiver) = channel();

    let log_to = match log_to {
        LogTo::File => {
            let mut log_dir = opts.log_dir.clone();
            log_dir.push("error.log");
            logging::LogTo::LogFile(log_dir)
        }
        LogTo::Stderr => logging::LogTo::Stderr,
    };

    let logger = logging::setup(init, &log_to, opts.debug);
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
            error!(logger, "{}", e);

            // Print a helpful message about where to start troubleshooting
            match &log_to {
                logging::LogTo::LogFile(path) => {
                    eprintln!("-----------------------");
                    eprintln!("Detected unclean exit\n");
                    eprintln!(
                        "Logs may contain more information: {}",
                        path.to_string_lossy()
                    );
                    eprintln!("-----------------------");
                }
                logging::LogTo::Stderr => (),
            }

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
        Command::Live { interval_s } => {
            run(init, opts, LogTo::File, Service::Off, |logger, _errs| {
                live(logger, Duration::from_secs(interval_s as u64))
            })
        }
        Command::Record {
            interval_s,
            retain_for_s,
            collect_io_stat,
            port,
        } => {
            let log_dir = opts.log_dir.clone();
            run(
                init,
                opts,
                LogTo::Stderr,
                Service::On(port),
                |logger, errs| {
                    record(
                        logger,
                        errs,
                        Duration::from_secs(interval_s as u64),
                        log_dir,
                        retain_for_s.map(|r| Duration::from_secs(r as u64)),
                        collect_io_stat,
                    )
                },
            )
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
            run(init, opts, LogTo::File, Service::Off, |logger, _errs| {
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
    info!(logger, "Starting up!");

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
