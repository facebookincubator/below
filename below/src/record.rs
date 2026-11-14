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

//! The `below record` daemon: collects a sample every interval and writes it to
//! the store.

use std::num::NonZeroU64;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::channel;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Args;
use clap::Parser;
use common::logutil;
use config::BelowConfig;
use model::gpu_stats_collector_plugin::GpuStatsCollectorPlugin;
use nix::sys::mman::MlockAllFlags;
use nix::sys::mman::mlockall;
use regex::Regex;
use scopeguard::guard;
use sd_notify::NotifyState;
use signal_hook::iterator::Signals;
use slog::debug;
use slog::error;
use slog::warn;
use store::ChunkSizePo2;
use store::CompressionMode;
use store::DataFrame;
use tokio::runtime::Builder as TB;

use crate::exitstat;
use crate::watchdog;

#[derive(Debug, Parser)]
pub struct CompressOpts {
    /// Enable zstd data file compression
    ///
    /// Depending on typical data, you can expect around 10x
    /// smaller data files, and an even higher compression ratio if
    /// used with --dict-compress-chunk-size.
    #[clap(long)]
    pub compress: bool,
    /// Only valid when used with --compress. Must be at least 2, a
    /// power of 2, and at most 32768.
    ///
    /// If specified, zstd dictionary compression is used in aligned
    /// chunks of size --dict-compress-chunk-size. The first frame of
    /// each is used as a zstd dictionary for the frames in the rest
    /// of the chunk.
    ///
    /// With --dict-compress-chunk-size 16, you can expect around
    /// 20-30x smaller data files.
    #[clap(long, requires("compress"), value_parser = parse_chunk_size)]
    pub dict_compress_chunk_size: Option<u32>,
}

const MAX_WATCHDOG_TIMEOUT_S: u64 = 86_400;

fn parse_watchdog_timeout_s(value: &str) -> Result<NonZeroU64> {
    let timeout = value
        .parse::<NonZeroU64>()
        .with_context(|| format!("{value} is not a positive integer"))?;
    if timeout.get() > MAX_WATCHDOG_TIMEOUT_S {
        bail!(
            "{} is greater than the maximum watchdog timeout of {} seconds",
            timeout,
            MAX_WATCHDOG_TIMEOUT_S
        );
    }
    Ok(timeout)
}

impl CompressOpts {
    pub fn to_compression_mode(&self) -> Result<CompressionMode> {
        Ok(match (self.compress, self.dict_compress_chunk_size) {
            (true, Some(chunk_size)) => {
                assert_eq!(chunk_size.count_ones(), 1, "chunk size not a power of 2");
                let chunk_size_po2 = chunk_size.trailing_zeros();
                CompressionMode::ZstdDictionary(ChunkSizePo2(chunk_size_po2))
            }
            (true, None) => CompressionMode::Zstd,
            (false, Some(_)) => {
                bail!("bug: --dict-compress-chunk-size can only be used with --compress");
            }
            (false, None) => CompressionMode::None,
        })
    }
}

fn parse_chunk_size(s: &str) -> Result<u32> {
    let x = s
        .parse::<u32>()
        .with_context(|| format!("{} cannot be parsed as a u32", s))?;
    if x <= 1 {
        bail!("{} is less than the minimum chunk size of 2", x);
    }
    if x.count_ones() != 1 {
        bail!("{} is not a power of two", x);
    }
    if x > store::MAX_CHUNK_COMPRESS_SIZE {
        bail!(
            "{} is greater than the maximimum supported chunk size of {}",
            x,
            store::MAX_CHUNK_COMPRESS_SIZE
        );
    }
    Ok(x)
}

/// Options of `below record`
#[derive(Debug, Args)]
pub struct RecordArgs {
    #[clap(short, long, default_value = "5")]
    pub interval_s: u64,
    /// Store retention in seconds. Data is stored in 24 hour shards.
    /// Whever an entire shard of data is outside the retention period it
    /// is discarded. That is, any data older than retention + 24 hours
    /// is guaranteed to be discarded.
    ///
    /// N.B. If --store-size-limit is set, data may be discarded earlier
    ///      than the specified retention.
    #[clap(long)]
    pub retain_for_s: Option<u64>,
    /// Store size limit in bytes. Data is stored in 24 hour shards.
    /// Shards before the active shard are deleted, oldest first,
    /// according to the size limit. Enforcement is only triggered on new
    /// shard creation.
    ///
    /// N.B. Since the active shard cannot be deleted, the size limit may
    ///      be exceeded by a single active shard.
    #[clap(long)]
    pub store_size_limit: Option<u64>,
    /// Whether or not to collect io.stat for cgroups which could
    /// be expensive
    #[clap(long)]
    pub collect_io_stat: bool,
    /// Threshold for hold long data collection takes to trigger warnings.
    #[clap(long, default_value = "500")]
    pub skew_detection_threshold_ms: u64,
    /// Flag to disable disk_stat collection.
    #[clap(long)]
    pub disable_disk_stat: bool,
    /// Flag to disable eBPF-based exitstats
    #[clap(long)]
    pub disable_exitstats: bool,
    /// Options for compression
    #[clap(flatten)]
    pub compress_opts: CompressOpts,
    /// Max number of samples we will buffer before writing becomes blocking
    #[clap(short, default_value = "10")]
    pub writer_buffer_size: usize,
    /// Report to /dev/kmsg after this many seconds without record-loop progress,
    /// then at that interval while stalled. The integer must exceed
    /// max(--interval-s, 1) and be at most 86400. Omit to disable.
    #[clap(long, value_parser = parse_watchdog_timeout_s)]
    pub watchdog_timeout_s: Option<NonZeroU64>,
}

/// Where `record` reports on itself, e.g. as fb303 counters.
pub trait RecordStats: Send + Sync {
    fn report_store_size(&self, _store_dir: &Path) {}
    fn report_nr_accelerators(&self, _sample: &model::Sample) {}
    fn report_collection_skew(&self) {}
    fn report_collection_time_ms(&self, _duration: Duration) {}
    fn report_writer_time_ms(&self, _duration: Duration) {}
}

pub fn bump_memlock_rlimit() -> Result<()> {
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

// Exitstat runs a bpf program that hooks into process exit events. This allows below to keep
// track of processes whose lifetimes are shorter than polling interval.
pub fn start_exitstat(
    logger: slog::Logger,
    debug: bool,
) -> (Arc<Mutex<procfs::PidMap>>, Option<Receiver<Error>>) {
    let mut exit_driver = exitstat::ExitstatDriver::new(logger, debug);
    let exit_buffer = exit_driver.get_buffer();
    let (bpf_err_send, bpf_err_recv) = channel();
    thread::Builder::new()
        .name("exit_driver".to_owned())
        .spawn(move || {
            match exit_driver.drive() {
                Ok(_) => {}
                Err(e) => bpf_err_send.send(e).expect("Failed to send error"),
            };
        })
        .expect("Failed to spawn thread");

    (exit_buffer, Some(bpf_err_recv))
}

// rgpu refreshes its cache when a read finds it stale, so reading it every
// sample adds NVML load on the GPU. Keep this under 2x rgpu's hot-group cache
// TTL (currently 25s), or reads block on a synchronous refresh.
pub const RGPU_INTERVAL: Duration = Duration::from_secs(30);

pub fn start_gpu_stats_thread_and_get_stats_receiver(
    gpu_collector: GpuStatsCollectorPlugin,
    logger: slog::Logger,
    interval: Duration,
) -> Result<model::collector_plugin::Consumer<model::gpu_stats_collector_plugin::SampleType>> {
    let target_interval = interval;
    let (mut collector, receiver) = model::collector_plugin::collector_consumer(gpu_collector);
    thread::Builder::new()
        .name("gpu_stats_collector".to_owned())
        .spawn(move || {
            // Exponential backoff on unrecoverable errors
            const EXP_BACKOFF_FACTOR: u32 = 2;
            const MAX_BACKOFF_SECS: u64 = 900;
            let max_backoff = Duration::from_secs(MAX_BACKOFF_SECS);
            let mut interval = target_interval;
            loop {
                let collect_instant = Instant::now();
                let rt = TB::new_current_thread()
                    .thread_name("create_fburl")
                    .build()
                    .expect("Failed to build tokio runtime.");
                match rt.block_on(collector.collect_and_update()) {
                    Ok(_) => {
                        interval = target_interval;
                    }
                    Err(e) => {
                        interval = std::cmp::min(
                            interval
                                .saturating_mul(EXP_BACKOFF_FACTOR),
                            max_backoff,
                        );
                        error!(
                            logger,
                            "GPU stats collection backing off {:?} because of unrecoverable error: {:#}",
                            interval,
                            e
                        );
                    }
                }
                let collect_duration = Instant::now().duration_since(collect_instant);

                const COLLECT_DURATION_WARN_THRESHOLD: u64 = 2;
                if collect_duration > Duration::from_secs(COLLECT_DURATION_WARN_THRESHOLD) {
                    warn!(
                        logger,
                        "GPU collection took {} > {}",
                        collect_duration.as_secs_f64(),
                        COLLECT_DURATION_WARN_THRESHOLD
                    );
                }
                if interval > collect_duration {
                    let sleep_duration = interval - collect_duration;
                    std::thread::sleep(sleep_duration);
                }
            }
        })
        .expect("Failed to spawn thread");
    Ok(receiver)
}

fn start_tc_stats_thread_and_get_stats_receiver(
    logger: slog::Logger,
    interval: Duration,
) -> Result<model::collector_plugin::Consumer<model::tc_collector_plugin::SampleType>> {
    let tc_collector = model::tc_collector_plugin::TcStatsCollectorPlugin::new(logger.clone())
        .context("Failed to initialize TC stats collector")?;
    let (mut collector, receiver) = model::collector_plugin::collector_consumer(tc_collector);

    // Start a thread to collect TC stats
    let target_interval = interval;
    thread::Builder::new()
        .name("tc_stats_collector".to_owned())
        .spawn(move || {
            // Exponential backoff on unrecoverable errors
            const EXP_BACKOFF_FACTOR: u32 = 2;
            const MAX_BACKOFF_SECS: u64 = 900;
            let max_backoff = Duration::from_secs(MAX_BACKOFF_SECS);
            let mut interval = target_interval;
            loop {
                let collect_instant = Instant::now();
                let rt = TB::new_current_thread()
                    .thread_name("create_fburl")
                    .build()
                    .expect("Failed to build tokio runtime.");
                match rt.block_on(collector.collect_and_update()) {
                    Ok(_) => {
                        interval = target_interval;
                    }
                    Err(e) => {
                        interval = std::cmp::min(
                            interval
                                .saturating_mul(EXP_BACKOFF_FACTOR),
                            max_backoff,
                        );
                        error!(
                            logger,
                            "TC stats collection backing off {:?} because of unrecoverable error: {:#}",
                            interval,
                            e
                        );
                    }
                }
                let collect_duration = Instant::now().duration_since(collect_instant);

                const COLLECT_DURATION_WARN_THRESHOLD: u64 = 2;
                if collect_duration > Duration::from_secs(COLLECT_DURATION_WARN_THRESHOLD) {
                    warn!(
                        logger,
                        "TC collection took {} > {}",
                        collect_duration.as_secs_f64(),
                        COLLECT_DURATION_WARN_THRESHOLD
                    );
                }
                if interval > collect_duration {
                    let sleep_duration = interval - collect_duration;
                    std::thread::sleep(sleep_duration);
                }
            }
        })
        .expect("Failed to spawn thread");

    Ok(receiver)
}

/// Start the cgroup-stat BPF driver, or fall back to reading the cgroupfs files.
///
/// BPF is an optimization here, so a kernel that cannot run it is not an error
/// worth exiting for: say why once and carry on reading files, which is what
/// `None` makes the collector do.
pub fn start_cgroup_bpf_or_read_files(
    logger: &slog::Logger,
    below_config: &BelowConfig,
    debug: bool,
) -> Option<cgroupfs::CgroupBpfHandle> {
    if !below_config.enable_cgroup_bpf {
        return None;
    }
    match cgroupfs::start_cgroup_bpf(debug, below_config.cgroup_root.clone()) {
        Ok(handle) => handle,
        Err(e) => {
            error!(
                logger,
                "Failed to start cgroup BPF collection, reading cgroup files instead: {:#}", e
            );
            None
        }
    }
}

/// Returns true if other end disconnected, false otherwise
pub fn check_for_exitstat_errors(logger: &slog::Logger, receiver: &Receiver<Error>) -> bool {
    // Print an error but don't exit on bpf issues. Do this b/c we can't always
    // be sure what kind of kernel we're running on and if it's new enough.
    match receiver.try_recv() {
        Ok(e) => {
            // When running as non-root for live, ignore EACCESS
            if let Some(e) = e.downcast_ref::<libbpf_rs::Error>()
                && e.kind() == libbpf_rs::ErrorKind::PermissionDenied
            {
                return false;
            }
            error!(logger, "{:#}", e);
        }
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
    if let Some(limit) = store_size_limit
        && !store
            .try_discard_until_size(limit)
            .context("Failed to discard earlier data")?
    {
        warn!(
            logger,
            "Failed to limit store size since the current shard is \
                greater than the limit"
        );
    }
    if let Some(retention) = retention {
        store
            .discard_earlier(SystemTime::now() - retention)
            .context("Failed to discard earlier data")?;
    }
    Ok(())
}

struct WorkerTask {
    post_collect_sys_time: SystemTime,
    data: DataFrame,
}

fn start_store_writer_thread(
    logger: slog::Logger,
    mut store: store::StoreWriter,
    store_size_limit: Option<u64>,
    retention: Option<Duration>,
    writer_buffer_size: usize,
    stats: Arc<dyn RecordStats>,
) -> Result<(JoinHandle<()>, SyncSender<WorkerTask>, libc::pid_t)> {
    let (send_task, recv_task) = sync_channel::<WorkerTask>(writer_buffer_size);
    let (send_started, recv_started) = sync_channel(0);
    let handle = thread::Builder::new()
        .name("store_writer".to_owned())
        .spawn(move || {
            if send_started.send(watchdog::current_tid()).is_err() {
                return;
            }
            loop {
                let loop_start_time = Instant::now();
                match recv_task.recv() {
                    Ok(write_task) => {
                        match store.put(write_task.post_collect_sys_time, &write_task.data) {
                            Ok(/* new shard */ true) => {
                                cleanup_store(
                                    &store,
                                    &logger,
                                    store_size_limit,
                                    /* retention */ None,
                                )
                                .expect("cleanup_store failed");
                            }
                            Ok(/* new shard */ false) => {}
                            Err(e) => {
                                error!(logger, "{:#}", e);
                                // no need to report/cleanup
                                continue;
                            }
                        }
                        stats.report_writer_time_ms(Instant::now().duration_since(loop_start_time));
                    }
                    Err(_) => {
                        warn!(
                            logger,
                            "store_write input channel disconnected, exiting thread"
                        );
                        // end loop
                        break;
                    }
                }

                // Only check against retention and not size limit. Size limit is only
                // checked on creation of successful write to a new shard.
                cleanup_store(&store, &logger, /* store_size_limit */ None, retention)
                    .expect("cleanup_store failed");
            }
        })?;
    let writer_tid = recv_started
        .recv()
        .context("store_writer exited before publishing its thread ID")?;
    Ok((handle, send_task, writer_tid))
}

/// Special Error that indicates the program should stop now. It represents an
/// actual signal, e.g. SIGINT, SIGTERM, that is handled by below and thus below
/// can shutdown gracefully.
#[derive(Clone, Debug)]
pub struct StopSignal {
    signal: i32,
}

impl std::error::Error for StopSignal {}

impl std::fmt::Display for StopSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stopped by signal: {}", self.signal)
    }
}

/// On SIGINT or SIGTERM, send a [`StopSignal`] to `errs`. Exit right away on
/// a second one.
pub fn forward_stop_signals(logger: slog::Logger, errs: Sender<Error>) -> Result<()> {
    let mut signals = Signals::new([signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM])?;
    thread::Builder::new()
        .name("sighandler".to_owned())
        .spawn(move || {
            let mut term_now = false;
            for signal in signals.forever() {
                if term_now {
                    error!(logger, "Below didn't stop in time. Terminate now!");
                    std::process::exit(1);
                }
                term_now = true;
                error!(logger, "Stop signal received: {}, exiting.", signal);
                errs.send(anyhow!(StopSignal { signal }))
                    .expect("Failed to send stop signal");
            }
        })
        .expect("Failed to spawn thread");
    Ok(())
}

/// The exit code for what a command returned, logging any error.
pub fn exit_code(logger: &slog::Logger, res: Result<()>) -> i32 {
    match res {
        Ok(()) => 0,
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

/// Collect a sample every interval and write it to the store, until `errs`
/// receives an error such as a [`StopSignal`].
///
/// `gpu_stats_collector` is only called when GPU stats are enabled.
pub fn record(
    logger: slog::Logger,
    errs: Receiver<Error>,
    below_config: &BelowConfig,
    args: &RecordArgs,
    debug: bool,
    stats: Arc<dyn RecordStats>,
    gpu_stats_collector: impl FnOnce() -> Result<GpuStatsCollectorPlugin>,
) -> Result<()> {
    let RecordArgs {
        interval_s,
        retain_for_s,
        store_size_limit,
        collect_io_stat,
        skew_detection_threshold_ms,
        disable_disk_stat,
        disable_exitstats,
        ref compress_opts,
        writer_buffer_size,
        watchdog_timeout_s,
    } = *args;
    let interval = Duration::from_secs(interval_s);
    let retention = retain_for_s.map(Duration::from_secs);
    let skew_detection_threshold = Duration::from_millis(skew_detection_threshold_ms);
    let watchdog_timeout = watchdog_timeout_s.map(|timeout| Duration::from_secs(timeout.get()));

    validate_watchdog_timeout(interval, watchdog_timeout)?;
    debug!(logger, "Starting up!");

    if !disable_exitstats {
        bump_memlock_rlimit()?;
    } else if below_config.enable_cgroup_bpf {
        // Only warn here: cgroup BPF collection reads the cgroup files when it
        // cannot start, so a limit we fail to raise costs data, not startup.
        // exitstat above has no such fallback, which is why it still bails.
        if let Err(e) = bump_memlock_rlimit() {
            warn!(
                logger,
                "Failed to raise memlock limit, cgroup BPF collection may read cgroup files instead: {:#}",
                e
            );
        }
    }

    let (exit_buffer, bpf_errs) = if disable_exitstats {
        (Arc::new(Mutex::new(procfs::PidMap::new())), None)
    } else {
        start_exitstat(logger.clone(), debug)
    };
    let mut bpf_err_warned = false;

    let cgroup_bpf = start_cgroup_bpf_or_read_files(&logger, below_config, debug);

    // Handle cgroup filter from conf and generate Regex
    let cgroup_re = if !below_config.cgroup_filter_out.is_empty() {
        Some(
            Regex::new(&below_config.cgroup_filter_out)
                .expect("Failed to generate regex from cgroup_filter_out in below.conf"),
        )
    } else {
        None
    };

    let gpu_stats_receiver = if below_config.enable_gpu_stats {
        Some(start_gpu_stats_thread_and_get_stats_receiver(
            gpu_stats_collector().context("Failed to initialize GPU stats collector")?,
            logger.clone(),
            interval,
        )?)
    } else {
        None
    };

    let tc_stats_receiver = if below_config.enable_tc_stats {
        Some(start_tc_stats_thread_and_get_stats_receiver(
            logger.clone(),
            interval,
        )?)
    } else {
        None
    };

    let mut collector = model::Collector::new(
        logger.clone(),
        model::CollectorOptions {
            cgroup_root: below_config.cgroup_root.clone(),
            exit_data: exit_buffer,
            collect_io_stat,
            disable_disk_stat,
            enable_btrfs_stats: below_config.enable_btrfs_stats,
            enable_ethtool_stats: below_config.enable_ethtool_stats,
            enable_ksm_stats: below_config.enable_ksm_stats,
            enable_resctrl_stats: below_config.enable_resctrl_stats,
            enable_tc_stats: below_config.enable_tc_stats,
            process_stack_trace_filter: below_config.process_stack_trace_filter,
            btrfs_samples: below_config.btrfs_samples,
            btrfs_min_pct: below_config.btrfs_min_pct,
            btrfs_commit_root: below_config.btrfs_commit_root,
            cgroup_re,
            gpu_stats_receiver,
            tc_stats_receiver,
            cgroup_bpf,
        },
    );

    let store = store::StoreWriter::new(
        logger.clone(),
        &below_config.store_dir,
        compress_opts.to_compression_mode()?,
        store::Format::Cbor,
    )?;

    let (writer_thread, send_task, writer_tid) = start_store_writer_thread(
        logger.clone(),
        store,
        store_size_limit,
        retention,
        writer_buffer_size,
        stats.clone(),
    )?;

    let send_task = guard(send_task, |s| {
        drop(s);
        let _ = writer_thread.join();
    });

    if below_config.mlock_record {
        let ret = mlockall(MlockAllFlags::MCL_CURRENT);
        if ret.is_err() {
            warn!(
                logger,
                "mlockall failed: {}. Continue without mlock",
                std::io::Error::last_os_error()
            );
        }
    }

    let watchdog =
        watchdog_timeout.and_then(
            |timeout| match watchdog::Watchdog::start(timeout, writer_tid) {
                Ok(watchdog) => Some(watchdog),
                Err(e) => {
                    warn!(
                        logger,
                        "Failed to start record watchdog; continuing without stall detection: {}",
                        e
                    );
                    None
                }
            },
        );

    let mut notified_ready = false;
    loop {
        if let Some(watchdog) = watchdog.as_ref() {
            watchdog.beat();
        }

        if !disable_exitstats && !bpf_err_warned {
            bpf_err_warned = check_for_exitstat_errors(
                &logger,
                bpf_errs
                    .as_ref()
                    .expect("Failed to unwrap bpf_errs receiver"),
            );
        }

        let collect_instant = Instant::now();

        let collected_sample = collector.collect_sample();
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
                if below_config.enable_gpu_stats {
                    stats.report_nr_accelerators(&s);
                }
                send_task
                    .send(WorkerTask {
                        post_collect_sys_time,
                        data: DataFrame { sample: s },
                    })
                    .expect("Failed to send write task");
                // Readiness means a sample was collected, not merely that the
                // process started. A no-op when not run by systemd.
                if !notified_ready {
                    if let Err(e) = sd_notify::notify(&[NotifyState::Ready]) {
                        warn!(logger, "Failed to notify systemd of readiness: {:#}", e);
                    }
                    notified_ready = true;
                }
            }
            Err(e) => {
                // Handle cgroupfs errors
                if let Some(cgroupfs::Error::NotCgroup2(_)) = e.downcast_ref::<cgroupfs::Error>() {
                    // Unrecoverable error -- below only supports cgroup2
                    bail!(e)
                }
                error!(logger, "{:#}", e);
            }
        };

        stats.report_store_size(below_config.store_dir.as_path());

        let collect_duration = Instant::now().duration_since(collect_instant);
        stats.report_collection_time_ms(collect_duration);

        // Sleep for at least 1s to avoid sample collision
        let sleep_duration = if interval > collect_duration {
            std::cmp::max(Duration::from_secs(1), interval - collect_duration)
        } else {
            Duration::from_secs(1)
        };

        if let Some(watchdog) = watchdog.as_ref() {
            watchdog.beat();
        }

        // Use recv timeout as loop interval
        match errs.recv_timeout(sleep_duration) {
            // SIGTERM/SIGINT or any genral error. Stop loop immediately.
            Ok(e) => bail!(e),
            // No error. Continue to collect next sample.
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => bail!("error channel disconnected"),
        };
    }
}

pub fn validate_watchdog_timeout(
    interval: Duration,
    watchdog_timeout: Option<Duration>,
) -> Result<()> {
    let normal_sleep = interval.max(Duration::from_secs(1));
    if let Some(timeout) = watchdog_timeout.filter(|timeout| *timeout <= normal_sleep) {
        bail!(
            "watchdog timeout ({timeout:?}) must exceed the maximum normal sleep ({normal_sleep:?})"
        );
    }
    Ok(())
}
