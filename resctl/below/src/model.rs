use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, Context, Result};

use below_thrift::types::{CgroupSample, Sample, SystemSample};

/// Collects data samples and maintains the latest data
pub struct Collector {
    last: Option<(Sample, Instant)>,
}

impl Collector {
    pub fn new() -> Collector {
        Collector { last: None }
    }

    /// Collect a new `Sample`, returning an updated Model
    pub fn update_model(&mut self) -> Result<Model> {
        let sample = collect_sample(true)?;
        let now = Instant::now();
        let last = self.last.replace((sample, now));
        let model = Model::new(
            SystemTime::now(),
            &self.last.as_ref().unwrap().0,
            last.as_ref().map(|(s, i)| (s, now.duration_since(*i))),
        );
        Ok(model)
    }
}

pub struct Model {
    pub timestamp: SystemTime,
    pub system: SystemModel,
    pub cgroup: CgroupModel,
    pub process: ProcessModel,
}

impl Model {
    /// Construct a `Model` from a Sample and optionally, the last
    /// `CumulativeSample` as well as the `Duration` since it was
    /// collected.
    pub fn new(timestamp: SystemTime, sample: &Sample, last: Option<(&Sample, Duration)>) -> Self {
        Model {
            timestamp,
            system: SystemModel::new(
                &sample.system,
                last.map(|(s, d)| (&s.system, d)),
                &sample.processes,
                last.map(|(s, d)| (&s.processes, d)),
            ),
            cgroup: CgroupModel::new(
                "<root>".to_string(),
                String::new(),
                0,
                &sample.cgroup,
                last.map(|(s, d)| (&s.cgroup, d)),
            ),
            process: ProcessModel::new(&sample.processes, last.map(|(s, d)| (&s.processes, d))),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemModel {
    pub hostname: String,
    pub cpu: Option<CpuModel>,
    pub mem: Option<MemoryModel>,
    pub io: Option<IoModel>,
}

impl SystemModel {
    fn new(
        sample: &SystemSample,
        last: Option<(&SystemSample, Duration)>,
        process_sample: &procfs::PidMap,
        process_last: Option<(&procfs::PidMap, Duration)>,
    ) -> SystemModel {
        let cpu = if let Some((last, delta)) = last {
            if let (Some(begin), Some(end), Some(cpus)) = (
                last.stat.total_cpu.as_ref(),
                sample.stat.total_cpu.as_ref(),
                sample.stat.cpus.as_ref(),
            ) {
                Some(CpuModel::new(&begin, &end, cpus.len(), delta))
            } else {
                None
            }
        } else {
            None
        };

        let mem = Some(MemoryModel::new(&sample.meminfo));

        let io = if let Some((last, delta)) = process_last {
            Some(IoModel::new(process_sample, Some((last, delta))))
        } else {
            None
        };

        SystemModel {
            hostname: sample.hostname.clone(),
            cpu,
            mem,
            io,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuModel {
    pub usage_pct: Option<f64>,
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
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
}

fn opt_merge<S: Sized, T: Sized>(a: Option<S>, b: Option<T>) -> Option<(S, T)> {
    a.and_then(|x| b.map(|y| (x, y)))
}

impl CpuModel {
    fn new(
        begin: &procfs::CpuStat,
        end: &procfs::CpuStat,
        ncpus: usize,
        delta: Duration,
    ) -> CpuModel {
        let idle_opt = usec_pct!(begin.idle_usec, end.idle_usec, delta);
        let iowait_opt = usec_pct!(begin.iowait_usec, end.iowait_usec, delta);
        let usage_opt = opt_merge(idle_opt, iowait_opt)
            .map(|(idle, iowait)| (ncpus as f64 * 100.0) - (idle + iowait));
        CpuModel {
            usage_pct: usage_opt,
            user_pct: usec_pct!(begin.user_usec, end.user_usec, delta),
            system_pct: usec_pct!(begin.system_usec, end.system_usec, delta),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryModel {
    pub total: Option<u64>,
    pub free: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
}

impl MemoryModel {
    fn new(meminfo: &procfs::MemInfo) -> MemoryModel {
        MemoryModel {
            total: meminfo.total.map(|v| v as u64),
            free: meminfo.free.map(|v| v as u64),
            anon: opt_merge(meminfo.active_anon, meminfo.inactive_anon)
                .map(|(a, b)| (a + b) as u64),
            file: opt_merge(meminfo.active_file, meminfo.inactive_file)
                .map(|(a, b)| (a + b) as u64),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IoModel {
    pub rbytes_per_sec: Option<f64>,
    pub wbytes_per_sec: Option<f64>,
}

impl IoModel {
    fn new(sample: &procfs::PidMap, last: Option<(&procfs::PidMap, Duration)>) -> IoModel {
        let mut rbytes = 0.0;
        let mut wbytes = 0.0;

        let process_model = ProcessModel::new(sample, last);
        for (_, spm) in process_model.processes.iter() {
            rbytes += spm
                .io
                .as_ref()
                .map_or(0.0, |io| io.rbytes_per_sec.map_or(0.0, |n| n));

            wbytes += spm
                .io
                .as_ref()
                .map_or(0.0, |io| io.wbytes_per_sec.map_or(0.0, |n| n));
        }

        IoModel {
            rbytes_per_sec: Some(rbytes),
            wbytes_per_sec: Some(wbytes),
        }
    }
}

pub struct ProcessModel {
    pub processes: BTreeMap<i32, SingleProcessModel>,
}

impl ProcessModel {
    fn new(sample: &procfs::PidMap, last: Option<(&procfs::PidMap, Duration)>) -> ProcessModel {
        let mut processes: BTreeMap<i32, SingleProcessModel> = BTreeMap::new();

        for (pid, pidinfo) in sample.iter() {
            processes.insert(
                *pid,
                SingleProcessModel::new(
                    &pidinfo,
                    last.and_then(|(p, d)| p.get(pid).map(|p| (p, d))),
                ),
            );
        }

        ProcessModel { processes }
    }
}

pub struct SingleProcessModel {
    pub pid: Option<i32>,
    pub comm: Option<String>,
    pub state: Option<procfs::PidState>,
    pub uptime_secs: Option<u64>,
    pub cgroup: Option<String>,
    pub io: Option<ProcessIoModel>,
    pub mem: Option<ProcessMemoryModel>,
    pub cpu: Option<ProcessCpuModel>,
}

impl SingleProcessModel {
    fn new(
        sample: &procfs::PidInfo,
        last: Option<(&procfs::PidInfo, Duration)>,
    ) -> SingleProcessModel {
        SingleProcessModel {
            pid: sample.stat.pid,
            comm: sample.stat.comm.clone(),
            state: sample.stat.state.clone(),
            uptime_secs: sample.stat.running_secs.map(|s| s as u64),
            cgroup: Some(sample.cgroup.clone()),
            io: last.map(|(l, d)| ProcessIoModel::new(&l.io, &sample.io, d)),
            mem: last.map(|(l, d)| ProcessMemoryModel::new(&l.stat, &sample.stat, d)),
            cpu: last.map(|(l, d)| ProcessCpuModel::new(&l.stat, &sample.stat, d)),
        }
    }
}

#[derive(Clone)]
pub struct ProcessIoModel {
    pub rbytes_per_sec: Option<f64>,
    pub wbytes_per_sec: Option<f64>,
}

impl ProcessIoModel {
    fn new(begin: &procfs::PidIo, end: &procfs::PidIo, delta: Duration) -> ProcessIoModel {
        ProcessIoModel {
            rbytes_per_sec: count_per_sec!(begin.rbytes, end.rbytes, delta),
            wbytes_per_sec: count_per_sec!(begin.wbytes, end.wbytes, delta),
        }
    }
}

#[derive(Clone)]
pub struct ProcessCpuModel {
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
    pub num_threads: Option<u64>,
}

impl ProcessCpuModel {
    fn new(begin: &procfs::PidStat, end: &procfs::PidStat, delta: Duration) -> ProcessCpuModel {
        ProcessCpuModel {
            user_pct: usec_pct!(begin.user_usecs, end.user_usecs, delta),
            system_pct: usec_pct!(begin.system_usecs, end.system_usecs, delta),
            num_threads: end.num_threads.map(|t| t as u64),
        }
    }
}

#[derive(Clone)]
pub struct ProcessMemoryModel {
    pub minorfaults_per_sec: Option<f64>,
    pub majorfaults_per_sec: Option<f64>,
    pub rss_bytes: Option<u64>,
}

impl ProcessMemoryModel {
    fn new(begin: &procfs::PidStat, end: &procfs::PidStat, delta: Duration) -> ProcessMemoryModel {
        ProcessMemoryModel {
            minorfaults_per_sec: count_per_sec!(begin.minflt, end.minflt, delta),
            majorfaults_per_sec: count_per_sec!(begin.majflt, end.majflt, delta),
            rss_bytes: end.rss_bytes.map(|i| i as u64),
        }
    }
}

fn get_hostname() -> Result<String> {
    if let Ok(h) = hostname::get() {
        if let Ok(s) = h.into_string() {
            return Ok(s);
        }
    }
    return Err(anyhow!("Could not get hostname"));
}

pub fn collect_sample(collect_io_stat: bool) -> Result<Sample> {
    let reader = procfs::ProcReader::new();
    Ok(Sample {
        cgroup: collect_cgroup_sample(&cgroupfs::CgroupReader::root(), collect_io_stat)?,
        processes: reader.read_all_pids()?,
        system: SystemSample {
            stat: reader.read_stat()?,
            meminfo: reader.read_meminfo()?,
            vmstat: reader.read_vmstat()?,
            hostname: get_hostname()?,
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
    v.map(|s| Some(s))
}

/// As above, but in addition, io.stat can have broken formatting due
/// to a kernel bug which will not output more than one page. In such
/// cases we should not fail all data collection, but just omit the io
/// data.
fn io_stat_wrap<S: Sized>(
    v: std::result::Result<S, cgroupfs::Error>,
) -> std::result::Result<Option<S>, cgroupfs::Error> {
    let wrapped = wrap(v);
    if let Err(cgroupfs::Error::InvalidFileFormat(_)) = wrapped {
        return Ok(None);
    } else if let Err(cgroupfs::Error::UnexpectedLine(_, _)) = wrapped {
        return Ok(None);
    }
    wrapped
}

fn collect_cgroup_sample(
    reader: &cgroupfs::CgroupReader,
    collect_io_stat: bool,
) -> Result<CgroupSample> {
    let io_stat = if collect_io_stat {
        io_stat_wrap(reader.read_io_stat())?
    } else {
        None
    };
    Ok(CgroupSample {
        cpu_stat: wrap(reader.read_cpu_stat())?,
        io_stat,
        memory_current: wrap(reader.read_memory_current().map(|v| v as i64))?,
        memory_stat: wrap(reader.read_memory_stat())?,
        pressure: wrap(reader.read_pressure())?,
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
                    .map(|child| {
                        collect_cgroup_sample(&child, collect_io_stat).map(|child_sample| {
                            (
                                child
                                    .name()
                                    .file_name()
                                    .expect("Unexpected .. in cgroup path")
                                    .to_string_lossy()
                                    .to_string(),
                                child_sample,
                            )
                        })
                    })
                    .collect::<Result<BTreeMap<String, CgroupSample>>>()
            })
            .transpose()?,
    })
}

#[derive(Clone, Debug, Default)]
pub struct CgroupModel {
    pub name: String,
    pub full_path: String,
    pub depth: u32,
    pub cpu: Option<CgroupCpuModel>,
    pub memory: Option<CgroupMemoryModel>,
    pub io: Option<BTreeMap<String, CgroupIoModel>>,
    pub io_total: Option<CgroupIoModel>,
    pub pressure: Option<CgroupPressureModel>,
    pub children: BTreeSet<CgroupModel>,
    pub count: u32,
}

// We implement equality and ordering based on the cgroup name only so
// CgroupModel can be stored in a BTreeSet
impl Ord for CgroupModel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for CgroupModel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CgroupModel {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CgroupModel {}

impl CgroupModel {
    fn new(
        name: String,
        full_path: String,
        depth: u32,
        sample: &CgroupSample,
        last: Option<(&CgroupSample, Duration)>,
    ) -> CgroupModel {
        let (cpu, io, io_total, pressure) = if let Some((last, delta)) = last {
            // We have cumulative data, create cpu, io, and pressure models
            let cpu = if let (Some(begin), Some(end)) =
                (last.cpu_stat.as_ref(), sample.cpu_stat.as_ref())
            {
                Some(CgroupCpuModel::new(&begin, &end, delta))
            } else {
                None
            };
            let io: Option<BTreeMap<String, CgroupIoModel>> = if let (Some(begin), Some(end)) =
                (last.io_stat.as_ref(), sample.io_stat.as_ref())
            {
                Some(
                    end.iter()
                        .filter_map(|(device_name, end_io_stat)| {
                            begin.get(device_name).map(|begin_io_stat| {
                                (
                                    device_name.clone(),
                                    CgroupIoModel::new(&begin_io_stat, &end_io_stat, delta),
                                )
                            })
                        })
                        .collect(),
                )
            } else {
                None
            };
            let io_total = if let Some(ref io_map) = io {
                Some(
                    io_map
                        .iter()
                        .fold(CgroupIoModel::empty(), |acc, (_, model)| acc + model),
                )
            } else {
                None
            };
            let pressure = if let (Some(begin), Some(end)) =
                (last.pressure.as_ref(), sample.pressure.as_ref())
            {
                Some(CgroupPressureModel::new(&begin, &end, delta))
            } else {
                None
            };
            (cpu, io, io_total, pressure)
        } else {
            // No cumulative data
            (None, None, None, None)
        };
        let memory =
            if let (Some(mem), Some(mem_stat)) = (sample.memory_current, &sample.memory_stat) {
                Some(CgroupMemoryModel::new(mem as u64, mem_stat))
            } else {
                None
            };
        // recursively calculate view of children
        // `children` is optional, but we treat it the same as an empty map
        let empty = BTreeMap::new();
        let children = sample
            .children
            .as_ref()
            .unwrap_or(&empty)
            .iter()
            .map(|(child_name, child_sample)| {
                CgroupModel::new(
                    child_name.clone(),
                    format!("{}/{}", full_path, child_name),
                    depth + 1,
                    &child_sample,
                    last.map_or(None, |(last, delta)| {
                        last.children
                            .as_ref()
                            .unwrap_or(&empty)
                            .get(child_name)
                            .map(|child_last| (child_last, delta))
                    }),
                )
            })
            .collect::<BTreeSet<CgroupModel>>();
        let nr_descendants: u32 = children.iter().fold(0, |acc, c| acc + c.count);
        CgroupModel {
            name,
            full_path,
            cpu,
            memory,
            io,
            io_total,
            pressure,
            children,
            count: nr_descendants + 1,
            depth,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CgroupCpuModel {
    pub usage_pct: Option<f64>,
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
    pub nr_periods_per_sec: Option<f64>,
    pub nr_throttled_per_sec: Option<f64>,
    pub throttled_pct: Option<f64>,
}

impl CgroupCpuModel {
    pub fn new(
        begin: &cgroupfs::CpuStat,
        end: &cgroupfs::CpuStat,
        delta: Duration,
    ) -> CgroupCpuModel {
        // Calculates percentage of a core consumed during the given
        // time period.
        let usec_pct = |a_opt, b_opt| {
            if let (Some(a), Some(b)) = (a_opt, b_opt) {
                if a <= b {
                    return Some((b - a) as f64 * 100.0 / delta.as_micros() as f64);
                }
            }
            None
        };
        CgroupCpuModel {
            usage_pct: usec_pct(begin.usage_usec, end.usage_usec),
            user_pct: usec_pct(begin.user_usec, end.user_usec),
            system_pct: usec_pct(begin.system_usec, end.system_usec),
            nr_periods_per_sec: count_per_sec!(begin.nr_periods, end.nr_periods, delta),
            nr_throttled_per_sec: count_per_sec!(begin.nr_throttled, end.nr_throttled, delta),
            throttled_pct: usec_pct(begin.throttled_usec, end.throttled_usec),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CgroupIoModel {
    pub rbytes_per_sec: Option<f64>,
    pub wbytes_per_sec: Option<f64>,
    pub rios_per_sec: Option<f64>,
    pub wios_per_sec: Option<f64>,
    pub dbytes_per_sec: Option<f64>,
    pub dios_per_sec: Option<f64>,
}

impl CgroupIoModel {
    pub fn new(begin: &cgroupfs::IoStat, end: &cgroupfs::IoStat, delta: Duration) -> CgroupIoModel {
        CgroupIoModel {
            rbytes_per_sec: count_per_sec!(begin.rbytes, end.rbytes, delta),
            wbytes_per_sec: count_per_sec!(begin.wbytes, end.wbytes, delta),
            rios_per_sec: count_per_sec!(begin.rios, end.rios, delta),
            wios_per_sec: count_per_sec!(begin.wios, end.wios, delta),
            dbytes_per_sec: count_per_sec!(begin.dbytes, end.dbytes, delta),
            dios_per_sec: count_per_sec!(begin.dios, end.dios, delta),
        }
    }

    pub fn empty() -> CgroupIoModel {
        CgroupIoModel {
            rbytes_per_sec: None,
            wbytes_per_sec: None,
            rios_per_sec: None,
            wios_per_sec: None,
            dbytes_per_sec: None,
            dios_per_sec: None,
        }
    }
}

impl std::ops::Add<&CgroupIoModel> for CgroupIoModel {
    type Output = Self;

    fn add(self, other: &Self) -> Self {
        let f = |a: Option<f64>, b: Option<f64>| {
            if a.is_none() && b.is_none() {
                None
            } else {
                Some(a.unwrap_or(0.0) + b.unwrap_or(0.0))
            }
        };
        Self {
            rbytes_per_sec: f(self.rbytes_per_sec, other.rbytes_per_sec),
            wbytes_per_sec: f(self.wbytes_per_sec, other.wbytes_per_sec),
            rios_per_sec: f(self.rios_per_sec, other.rios_per_sec),
            wios_per_sec: f(self.wios_per_sec, other.wios_per_sec),
            dbytes_per_sec: f(self.dbytes_per_sec, other.dbytes_per_sec),
            dios_per_sec: f(self.dios_per_sec, other.dios_per_sec),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CgroupMemoryModel {
    pub total: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub slab: Option<u64>,
    pub sock: Option<u64>,
    pub shmem: Option<u64>,
    pub file_mapped: Option<u64>,
    pub file_dirty: Option<u64>,
    pub file_writeback: Option<u64>,
    pub anon_thp: Option<u64>,
    pub inactive_anon: Option<u64>,
    pub active_anon: Option<u64>,
    pub inactive_file: Option<u64>,
    pub active_file: Option<u64>,
    pub unevictable: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    // TODO: memory.stat has a lot of cumulative stats that need to be
    // added
}

impl CgroupMemoryModel {
    pub fn new(current: u64, stat: &cgroupfs::MemoryStat) -> CgroupMemoryModel {
        CgroupMemoryModel {
            total: Some(current),
            anon: stat.anon.map(|v| v as u64),
            file: stat.file.map(|v| v as u64),
            kernel_stack: stat.kernel_stack.map(|v| v as u64),
            slab: stat.slab.map(|v| v as u64),
            sock: stat.sock.map(|v| v as u64),
            shmem: stat.shmem.map(|v| v as u64),
            file_mapped: stat.file_mapped.map(|v| v as u64),
            file_dirty: stat.file_dirty.map(|v| v as u64),
            file_writeback: stat.file_writeback.map(|v| v as u64),
            anon_thp: stat.anon_thp.map(|v| v as u64),
            inactive_anon: stat.inactive_anon.map(|v| v as u64),
            active_anon: stat.active_anon.map(|v| v as u64),
            inactive_file: stat.inactive_file.map(|v| v as u64),
            active_file: stat.active_file.map(|v| v as u64),
            unevictable: stat.unevictable.map(|v| v as u64),
            slab_reclaimable: stat.slab_reclaimable.map(|v| v as u64),
            slab_unreclaimable: stat.slab_unreclaimable.map(|v| v as u64),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CgroupPressureModel {
    pub cpu_some_pct: Option<f64>,
    pub io_some_pct: Option<f64>,
    pub io_full_pct: Option<f64>,
    pub memory_some_pct: Option<f64>,
    pub memory_full_pct: Option<f64>,
}

impl CgroupPressureModel {
    fn new(
        begin: &cgroupfs::Pressure,
        end: &cgroupfs::Pressure,
        delta: Duration,
    ) -> CgroupPressureModel {
        let usec_pct = |a_opt, b_opt| {
            if let (Some(a), Some(b)) = (a_opt, b_opt) {
                if a <= b {
                    return Some((b - a) as f64 * 100.0 / delta.as_micros() as f64);
                }
            }
            None
        };
        CgroupPressureModel {
            cpu_some_pct: usec_pct(begin.cpu.some.total, end.cpu.some.total),
            io_some_pct: usec_pct(begin.io.some.total, end.io.some.total),
            io_full_pct: usec_pct(begin.io.full.total, end.io.full.total),
            memory_some_pct: usec_pct(begin.memory.some.total, end.memory.some.total),
            memory_full_pct: usec_pct(begin.memory.full.total, end.memory.full.total),
        }
    }
}
