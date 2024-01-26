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

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use regex::Regex;
use slog::error;

use super::*;
use crate::collector_plugin;

pub struct CollectorOptions {
    pub cgroup_root: PathBuf,
    pub exit_data: Arc<Mutex<procfs::PidMap>>,
    pub collect_io_stat: bool,
    pub disable_disk_stat: bool,
    pub enable_btrfs_stats: bool,
    pub enable_ethtool_stats: bool,
    pub enable_resctrl_stats: bool,
    pub btrfs_samples: u64,
    pub btrfs_min_pct: f64,
    pub cgroup_re: Option<Regex>,
    pub gpu_stats_receiver:
        Option<collector_plugin::Consumer<crate::gpu_stats_collector_plugin::SampleType>>,
}

impl Default for CollectorOptions {
    fn default() -> Self {
        Self {
            cgroup_root: Path::new(cgroupfs::DEFAULT_CG_ROOT).to_path_buf(),
            exit_data: Default::default(),
            collect_io_stat: true,
            disable_disk_stat: false,
            enable_btrfs_stats: false,
            enable_ethtool_stats: false,
            enable_resctrl_stats: false,
            btrfs_samples: btrfs::DEFAULT_SAMPLES,
            btrfs_min_pct: btrfs::DEFAULT_MIN_PCT,
            cgroup_re: None,
            gpu_stats_receiver: None,
        }
    }
}

/// Collects data samples and maintains the latest data
pub struct Collector {
    logger: slog::Logger,
    proc_reader: procfs::ProcReader,
    prev_sample: Option<(Sample, Instant)>,
    collector_options: CollectorOptions,
}

impl Collector {
    pub fn new(logger: slog::Logger, collector_options: CollectorOptions) -> Self {
        Self {
            logger,
            proc_reader: procfs::ProcReader::new(),
            prev_sample: None,
            collector_options,
        }
    }

    pub fn collect_sample(&mut self) -> Result<Sample> {
        collect_sample(&self.logger, &mut self.proc_reader, &self.collector_options)
    }

    /// Collect a new `Sample`, returning an updated Model
    pub fn collect_and_update_model(&mut self) -> Result<Model> {
        let now = Instant::now();
        let sample = self.collect_sample()?;
        let model = Model::new(
            SystemTime::now(),
            &sample,
            self.prev_sample
                .as_ref()
                .map(|(s, i)| (s, now.duration_since(*i))),
        );
        self.prev_sample = Some((sample, now));
        Ok(model)
    }
}

pub fn opt_add<T: std::ops::Add<T, Output = T>>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        _ => None,
    }
}

pub fn opt_multiply<S: Sized + std::ops::Mul<T, Output = S>, T: Sized>(
    a: Option<S>,
    b: Option<T>,
) -> Option<S> {
    a.and_then(|x| b.map(|y| x * y))
}

pub fn get_hostname() -> Result<String> {
    if let Ok(h) = hostname::get() {
        if let Ok(s) = h.into_string() {
            return Ok(s);
        }
    }
    Err(anyhow!("Could not get hostname"))
}

#[cfg(fbcode_build)]
pub fn get_os_release() -> Result<String> {
    std::fs::read_to_string("/etc/centos-release")
        .context("Fail to get centos release")
        .map(|o| o.trim_matches('\n').trim().into())
}

use os_info as _; // So RUSTFIXDEPS doesn't complain.
#[cfg(not(fbcode_build))]
pub fn get_os_release() -> Result<String> {
    let info = os_info::get();
    Ok(format!(
        "{} {} {}",
        info.os_type(),
        info.version(),
        info.bitness()
    ))
}

fn merge_procfs_and_exit_data(
    mut procfs_data: procfs::PidMap,
    exit_data: procfs::PidMap,
) -> procfs::PidMap {
    exit_data
        .iter()
        // If `procfs_data` already has the pid, then we use the procfs data because the time delta
        // between the two collection points is negligible and procfs collected data is more
        // complete.
        .for_each(|entry| {
            if !procfs_data.contains_key(entry.0) {
                procfs_data.insert(*entry.0, entry.1.clone());
            }
        });

    procfs_data
}

/// This function will test if all field of DiskStat are zero, if so we will need to skip
/// this sample inside collector.
fn is_all_zero_disk_stats(disk_stats: &procfs::DiskStat) -> bool {
    disk_stats.read_completed == Some(0)
        && disk_stats.write_completed == Some(0)
        && disk_stats.discard_completed == Some(0)
        && disk_stats.read_merged == Some(0)
        && disk_stats.read_sectors == Some(0)
        && disk_stats.time_spend_read_ms == Some(0)
        && disk_stats.write_merged == Some(0)
        && disk_stats.write_sectors == Some(0)
        && disk_stats.time_spend_write_ms == Some(0)
        && disk_stats.discard_merged == Some(0)
        && disk_stats.discard_sectors == Some(0)
        && disk_stats.time_spend_discard_ms == Some(0)
}

fn collect_sample(
    logger: &slog::Logger,
    reader: &mut procfs::ProcReader,
    options: &CollectorOptions,
) -> Result<Sample> {
    let btrfs_reader =
        btrfs::BtrfsReader::new(options.btrfs_samples, options.btrfs_min_pct, logger.clone());
    let ethtool_reader = ethtool::EthtoolReader::new();

    // Take mutex, then take all values out of shared map and replace with default map
    //
    // NB: unconditionally drain the exit buffer otherwise we can leak the entries
    let exit_pidmap = std::mem::take(
        &mut *options
            .exit_data
            .lock()
            .expect("tried to acquire poisoned lock"),
    );

    Ok(Sample {
        cgroup: collect_cgroup_sample(
            &cgroupfs::CgroupReader::new(options.cgroup_root.to_owned())?,
            options.collect_io_stat,
            logger,
            &options.cgroup_re,
        )?,
        processes: merge_procfs_and_exit_data(reader.read_all_pids()?, exit_pidmap),
        netstats: match procfs::NetReader::new(logger.clone()).and_then(|v| v.read_netstat()) {
            Ok(ns) => ns,
            Err(e) => {
                error!(logger, "{:#}", e);
                Default::default()
            }
        },
        system: SystemSample {
            stat: reader.read_stat()?,
            meminfo: reader.read_meminfo()?,
            vmstat: reader.read_vmstat()?,
            slabinfo: reader.read_slabinfo().unwrap_or_default(),
            hostname: get_hostname()?,
            kernel_version: match reader.read_kernel_version() {
                Ok(k) => Some(k),
                Err(e) => {
                    error!(logger, "{:#}", e);
                    None
                }
            },
            os_release: match get_os_release() {
                Ok(o) => Some(o),
                Err(e) => {
                    error!(logger, "{:#}", e);
                    None
                }
            },
            disks: if options.disable_disk_stat {
                Default::default()
            } else {
                match reader.read_disk_stats_and_fsinfo() {
                    Ok(disks) => disks
                        .into_iter()
                        .filter(|(disk_name, disk_stat)| {
                            if disk_name.starts_with("ram") || disk_name.starts_with("loop") {
                                return false;
                            }
                            !is_all_zero_disk_stats(disk_stat)
                        })
                        .collect(),
                    Err(e) => {
                        error!(logger, "{:#}", e);
                        Default::default()
                    }
                }
            },
            btrfs: if !options.enable_btrfs_stats {
                Default::default()
            } else {
                match btrfs_reader.sample() {
                    Ok(btrfs) => Some(btrfs),
                    Err(e) => {
                        error!(logger, "{:#}", e);
                        Default::default()
                    }
                }
            },
        },
        gpus: {
            if let Some(gpu_stats_receiver) = &options.gpu_stats_receiver {
                // It is possible to receive no sample if the
                // collector has not updated since the previous take
                // or the collector encountered a recoverable error
                // (e.g. timeout). The behavior for now is to store an
                // empty map. Alternatively we could store the latest
                // sample and read that, but then we have to decide how
                // stale the data can be.
                Some(
                    gpu_stats_receiver
                        .try_take()
                        .context("GPU stats collector had an error")?
                        .unwrap_or_default(),
                )
            } else {
                None
            }
        },
        ethtool: if !options.enable_ethtool_stats {
            Default::default()
        } else {
            match ethtool_reader.read_stats::<ethtool::Ethtool>() {
                Ok(ethtool_stats) => Some(ethtool_stats),
                Err(e) => {
                    error!(logger, "{:#}", e);
                    Default::default()
                }
            }
        },
        resctrl: if !options.enable_resctrl_stats {
            None
        } else {
            match resctrlfs::ResctrlReader::root() {
                Ok(resctrl_reader) => match resctrl_reader.read_all() {
                    Ok(resctrl) => Some(resctrl),
                    Err(e) => {
                        error!(logger, "{:#}", e);
                        None
                    }
                },
                Err(_e) => {
                    // ResctrlReader only fails to initialize if resctrlfs is
                    // not mounted. In this case we ignore.
                    None
                }
            }
        },
    })
}

/// cgroupfs can give us a NotFound error if the cgroup doesn't have
/// the relevant stat file (e.g. if it is the root cgroup). We
/// translate that into `None` so that other errors are propagated,
/// but omitted data is allowed.
///
/// This method just does that translation for us.
fn wrap<S: Sized>(
    v: std::result::Result<S, cgroupfs::Error>,
) -> std::result::Result<Option<S>, cgroupfs::Error> {
    if let Err(cgroupfs::Error::IoError(_, ref e)) = v {
        if e.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        if e.kind() == std::io::ErrorKind::Other {
            if let Some(errno) = e.raw_os_error() {
                if errno == /* ENODEV */ 19 {
                    // If the cgroup is removed after a control file is opened,
                    // ENODEV is returned. Ignore it.
                    return Ok(None);
                }
            }
        }
    }
    v.map(Some)
}

/// As above, but in addition, io.stat can have broken formatting due
/// to a kernel bug which will not output more than one page. In such
/// cases we should not fail all data collection, but just omit the io
/// data.
fn io_stat_wrap<S: Sized>(
    v: std::result::Result<S, cgroupfs::Error>,
) -> std::result::Result<Option<S>, cgroupfs::Error> {
    match wrap(v) {
        Err(cgroupfs::Error::InvalidFileFormat(_)) => Ok(None),
        Err(cgroupfs::Error::UnexpectedLine(_, _)) => Ok(None),
        wrapped => wrapped,
    }
}

/// Pressure metrics may not be supported, in which case cgroupfs will
/// return a specific error. We don't fail all data collection, just
/// omit pressure metrics.
fn pressure_wrap<S: Sized>(
    v: std::result::Result<S, cgroupfs::Error>,
) -> std::result::Result<Option<S>, cgroupfs::Error> {
    match wrap(v) {
        Err(cgroupfs::Error::PressureNotSupported(_)) => Ok(None),
        wrapped => wrapped,
    }
}

fn collect_cgroup_sample(
    reader: &cgroupfs::CgroupReader,
    collect_io_stat: bool,
    logger: &slog::Logger,
    cgroup_re: &Option<Regex>,
) -> Result<CgroupSample> {
    let io_stat = if collect_io_stat {
        io_stat_wrap(reader.read_io_stat())?
    } else {
        None
    };
    Ok(CgroupSample {
        cpu_stat: wrap(reader.read_cpu_stat())?.map(Into::into),
        io_stat,
        tids_current: wrap(reader.read_pids_current())?,
        tids_max: wrap(reader.read_pids_max())?,
        memory_current: wrap(reader.read_memory_current().map(|v| v as i64))?,
        memory_stat: wrap(reader.read_memory_stat())?.map(Into::into),
        pressure: pressure_wrap(reader.read_pressure())?.map(Into::into),
        // We transpose at the end here to convert the
        // Option<Result<BTreeMap... into Result<Option<BTreeMap and
        // then bail any errors with `?` - leaving us with the
        // Option<BTreeMap...
        //
        // The only case this can be None is if the cgroup no longer
        // exists - this is consistent with the above members
        children: wrap(reader.child_cgroup_iter())
            .context("Failed to get iterator over cgroup children")?
            .map(|child_iter| {
                child_iter
                    .filter(|child| {
                        if let Some(cgroup_re) = cgroup_re.as_ref() {
                            !cgroup_re.is_match(&child.name().to_string_lossy())
                        } else {
                            true
                        }
                    })
                    .map(|child| {
                        collect_cgroup_sample(&child, collect_io_stat, logger, cgroup_re).map(
                            |child_sample| {
                                (
                                    child
                                        .name()
                                        .file_name()
                                        .expect("Unexpected .. in cgroup path")
                                        .to_string_lossy()
                                        .to_string(),
                                    child_sample,
                                )
                            },
                        )
                    })
                    .collect::<Result<BTreeMap<String, CgroupSample>>>()
            })
            .transpose()?,
        memory_swap_current: wrap(reader.read_memory_swap_current().map(|v| v as i64))?,
        memory_zswap_current: None, // Use the one from memory.stat
        memory_min: wrap(reader.read_memory_min())?,
        memory_low: wrap(reader.read_memory_low())?,
        memory_high: wrap(reader.read_memory_high())?,
        memory_max: wrap(reader.read_memory_max())?,
        memory_swap_max: wrap(reader.read_memory_swap_max())?,
        memory_zswap_max: wrap(reader.read_memory_zswap_max())?,
        memory_events: wrap(reader.read_memory_events())?.map(Into::into),
        inode_number: match reader.read_inode_number() {
            Ok(st_ino) => Some(st_ino as i64),
            Err(e) => {
                error!(logger, "Fail to collect inode number: {:#}", e);
                None
            }
        },
        cgroup_stat: wrap(reader.read_cgroup_stat())?.map(Into::into),
        memory_numa_stat: wrap(reader.read_memory_numa_stat())?.map(Into::into),
        cpuset_cpus: wrap(reader.read_cpuset_cpus())?,
        cpuset_cpus_effective: wrap(reader.read_cpuset_cpus_effective())?,
        cpuset_mems: wrap(reader.read_cpuset_mems())?,
        cpuset_mems_effective: wrap(reader.read_cpuset_mems_effective())?,
        cpu_weight: wrap(reader.read_cpu_weight())?,
        cpu_max: wrap(reader.read_cpu_max())?,
        cgroup_controllers: wrap(reader.read_cgroup_controllers())?,
        cgroup_subtree_control: wrap(reader.read_cgroup_subtree_control())?,
    })
}

macro_rules! usec_pct {
    ($a_opt:expr, $b_opt:expr, $delta:expr) => {{
        let mut ret = None;
        if let (Some(a), Some(b)) = ($a_opt, $b_opt) {
            if a <= b {
                ret = Some((b - a) as f64 * 100.0 / $delta.as_micros() as f64);
            }
        }
        ret
    }};
}

macro_rules! count_per_sec {
    ($a_opt:expr, $b_opt:expr, $delta:expr) => {{
        let mut ret = None;
        if let (Some(a), Some(b)) = ($a_opt, $b_opt) {
            if a <= b {
                ret = Some((b - a) as f64 / $delta.as_secs_f64());
            }
        }
        ret
    }};
    ($a_opt:expr, $b_opt:expr, $delta:expr, $target_type:ty) => {{
        let mut ret = None;
        if let (Some(a), Some(b)) = ($a_opt, $b_opt) {
            if a <= b {
                ret = Some(((b - a) as f64 / $delta.as_secs_f64()).ceil() as $target_type);
            }
        }
        ret
    }};
}

#[allow(unused)]
macro_rules! get_option_rate {
    ($key:ident, $sample:ident, $last:ident) => {
        $last
            .map(|(l, d)| {
                count_per_sec!(l.$key.map(|s| s as u64), $sample.$key.map(|s| s as u64), d)
            })
            .unwrap_or_default()
            .map(|s| s as u64)
    };
}
