#![deny(clippy::all)]
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use thiserror::Error;

pub use procfs_thrift::types::{
    CpuStat, MemInfo, PidInfo, PidIo, PidMap, PidStat, PidState, Stat, VmStat,
};

#[cfg(test)]
mod test;

lazy_static! {
    /// The number of microseconds per clock tick
    ///
    /// Calculated from `sysconf(_SC_CLK_TCK)`
    static ref MICROS_PER_TICK: u64 = {
        1_000_000 / ticks_per_second()
    };

    static ref TICKS_PER_SECOND: u64 = {
        ticks_per_second()
    };

    /// Size of page in bytes
    static ref PAGE_SIZE: i64 = {
        page_size()
    };
}

fn ticks_per_second() -> u64 {
    match unsafe { libc::sysconf(libc::_SC_CLK_TCK) } {
        -1 => panic!("Failed to query clock tick rate"),
        x => x as u64,
    }
}

fn page_size() -> i64 {
    match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
        -1 => panic!("Failed to query clock tick rate"),
        x => x as i64,
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file format: {0:?}")]
    InvalidFileFormat(PathBuf),
    #[error("Invalid Pid State Character {1} in {0:?}")]
    InvalidPidState(PathBuf, char),
    #[error("{1:?}: {0:?}")]
    IoError(PathBuf, #[source] std::io::Error),
    #[error("Failed to parse {item} as {type_name} in line: {line} from {path:?}")]
    ParseError {
        line: String,
        item: String,
        type_name: String,
        path: PathBuf,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! parse_item {
    // Parse rhs (an option, usually from an iterator) into type $t or
    // report a parse error on $line otherwise
    // Returns a Result<Option<$t>>
    ($path:expr, $rhs:expr, $t:tt, $line:ident) => {
        if let Some(s) = $rhs {
            s.parse::<$t>()
                .map_err(|_| Error::ParseError {
                    line: $line.clone(),
                    item: s.to_string(),
                    type_name: stringify!($t).to_string(),
                    path: $path.clone(),
                })
                .map(|v| Some(v))
        } else {
            Ok(None)
        }
    };
    // Parse the second item of $line as type $t with the same
    // semantics as above
    ($path: expr, $line:ident, $t:tt) => {{
        let mut items = $line.split_whitespace();

        // Advance past the name
        items.next();

        parse_item!($path, items.next(), $t, $line)
    }};
}

macro_rules! parse_usec {
    ($path:expr, $rhs:expr, $line:ident) => {
        parse_item!($path, $rhs, u64, $line).map(|opt| opt.map(|v| (v * *MICROS_PER_TICK) as i64))
    };
}

macro_rules! parse_sec {
    ($path:expr, $rhs:expr, $line:ident) => {
        parse_item!($path, $rhs, u64, $line).map(|opt| opt.map(|v| (v / *TICKS_PER_SECOND) as i64))
    };
}

macro_rules! parse_kb {
    ($path:expr, $rhs:expr, $line:ident) => {
        parse_item!($path, $rhs, u64, $line).map(|opt| opt.map(|v| (v * 1024) as i64))
    };
}

pub struct ProcReader {
    path: PathBuf,
}

impl ProcReader {
    pub fn new() -> ProcReader {
        ProcReader {
            path: Path::new("/proc").to_path_buf(),
        }
    }

    pub fn new_with_custom_procfs(path: PathBuf) -> ProcReader {
        ProcReader { path }
    }

    fn read_uptime_secs(&self) -> Result<i64> {
        let path = self.path.join("uptime");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let mut buf_reader = BufReader::new(file);
        let mut line = String::new();
        buf_reader
            .read_line(&mut line)
            .map_err(|e| Error::IoError(path.clone(), e))?;

        let mut items = line.split_whitespace();

        match parse_item!(path, items.next(), f64, line) {
            Ok(Some(uptime)) => Ok(uptime.round() as i64),
            Ok(None) => Err(Error::InvalidFileFormat(path)),
            Err(e) => Err(e),
        }
    }

    fn process_cpu_stat(path: &PathBuf, line: &String) -> Result<CpuStat> {
        //Format is like "cpu9 6124418 452468 3062529 230073290 216237 0 45647 0 0 0"
        let mut items = line.split_whitespace();
        let mut cpu: CpuStat = Default::default();

        // Advance past "cpu*" item
        items.next();

        cpu.user_usec = parse_usec!(path, items.next(), line)?;
        cpu.nice_usec = parse_usec!(path, items.next(), line)?;
        cpu.system_usec = parse_usec!(path, items.next(), line)?;
        cpu.idle_usec = parse_usec!(path, items.next(), line)?;
        cpu.iowait_usec = parse_usec!(path, items.next(), line)?;
        cpu.irq_usec = parse_usec!(path, items.next(), line)?;
        cpu.softirq_usec = parse_usec!(path, items.next(), line)?;
        cpu.stolen_usec = parse_usec!(path, items.next(), line)?;
        cpu.guest_usec = parse_usec!(path, items.next(), line)?;
        cpu.guest_nice_usec = parse_usec!(path, items.next(), line)?;

        Ok(cpu)
    }

    pub fn read_stat(&self) -> Result<Stat> {
        let path = self.path.join("stat");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut stat: Stat = Default::default();
        let mut cpus = Vec::new();
        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let mut items = line.split_whitespace();
            if let Some(item) = items.next() {
                match item {
                    "intr" => {
                        stat.total_interrupt_count = parse_item!(&path, items.next(), i64, line)?
                    }
                    "ctxt" => stat.context_switches = parse_item!(&path, items.next(), i64, line)?,
                    "btime" => {
                        stat.boot_time_epoch_secs = parse_item!(&path, items.next(), i64, line)?
                    }
                    "processes" => {
                        stat.total_processes = parse_item!(&path, items.next(), i64, line)?
                    }
                    "procs_running" => {
                        stat.running_processes = parse_item!(&path, items.next(), i32, line)?
                    }
                    "procs_blocked" => {
                        stat.blocked_processes = parse_item!(&path, items.next(), i32, line)?
                    }
                    x => {
                        if x == "cpu" {
                            stat.total_cpu = Some(Self::process_cpu_stat(&path, &line)?);
                        } else if x.starts_with("cpu") {
                            cpus.push(Self::process_cpu_stat(&path, &line)?);
                        }
                    }
                }
            }
        }
        if !cpus.is_empty() {
            stat.cpus = Some(cpus);
        }

        if stat == Default::default() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(stat)
        }
    }

    pub fn read_meminfo(&self) -> Result<MemInfo> {
        let path = self.path.join("meminfo");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut meminfo: MemInfo = Default::default();

        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let mut items = line.split_whitespace();
            if let Some(item) = items.next() {
                match item {
                    "MemTotal:" => meminfo.total = parse_kb!(path, items.next(), line)?,
                    "MemFree:" => meminfo.free = parse_kb!(path, items.next(), line)?,
                    "MemAvailable:" => meminfo.available = parse_kb!(path, items.next(), line)?,
                    "Buffers:" => meminfo.buffers = parse_kb!(path, items.next(), line)?,
                    "Cached:" => meminfo.cached = parse_kb!(path, items.next(), line)?,
                    "SwapCached:" => meminfo.swap_cached = parse_kb!(path, items.next(), line)?,
                    "Active:" => meminfo.active = parse_kb!(path, items.next(), line)?,
                    "Inactive:" => meminfo.inactive = parse_kb!(path, items.next(), line)?,
                    "Active(anon):" => meminfo.active_anon = parse_kb!(path, items.next(), line)?,
                    "Inactive(anon):" => {
                        meminfo.inactive_anon = parse_kb!(path, items.next(), line)?
                    }
                    "Active(file):" => meminfo.active_file = parse_kb!(path, items.next(), line)?,
                    "Inactive(file):" => {
                        meminfo.inactive_file = parse_kb!(path, items.next(), line)?
                    }
                    "Unevictable:" => meminfo.unevictable = parse_kb!(path, items.next(), line)?,
                    "Mlocked:" => meminfo.mlocked = parse_kb!(path, items.next(), line)?,
                    "SwapTotal:" => meminfo.swap_total = parse_kb!(path, items.next(), line)?,
                    "SwapFree:" => meminfo.swap_free = parse_kb!(path, items.next(), line)?,
                    "Dirty:" => meminfo.dirty = parse_kb!(path, items.next(), line)?,
                    "Writeback:" => meminfo.writeback = parse_kb!(path, items.next(), line)?,
                    "AnonPages:" => meminfo.anon_pages = parse_kb!(path, items.next(), line)?,
                    "Mapped:" => meminfo.mapped = parse_kb!(path, items.next(), line)?,
                    "Shmem:" => meminfo.shmem = parse_kb!(path, items.next(), line)?,
                    "KReclaimable:" => meminfo.kreclaimable = parse_kb!(path, items.next(), line)?,
                    "Slab:" => meminfo.slab = parse_kb!(path, items.next(), line)?,
                    "SReclaimable:" => {
                        meminfo.slab_reclaimable = parse_kb!(path, items.next(), line)?
                    }
                    "SUnreclaim:" => {
                        meminfo.slab_unreclaimable = parse_kb!(path, items.next(), line)?
                    }
                    "KernelStack:" => meminfo.kernel_stack = parse_kb!(path, items.next(), line)?,
                    "PageTables:" => meminfo.page_tables = parse_kb!(path, items.next(), line)?,
                    "AnonHugePages:" => {
                        meminfo.anon_huge_pages = parse_kb!(path, items.next(), line)?
                    }
                    "ShmemHugePages:" => {
                        meminfo.shmem_huge_pages = parse_kb!(path, items.next(), line)?
                    }
                    "FileHugePages:" => {
                        meminfo.file_huge_pages = parse_kb!(path, items.next(), line)?
                    }
                    "HugePages_Total:" => {
                        meminfo.total_huge_pages = parse_item!(path, items.next(), i64, line)?
                    }
                    "HugePages_Free:" => {
                        meminfo.free_huge_pages = parse_item!(path, items.next(), i64, line)?
                    }
                    _ => {}
                }
            }
        }
        if meminfo == Default::default() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(meminfo)
        }
    }

    pub fn read_vmstat(&self) -> Result<VmStat> {
        let path = self.path.join("vmstat");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut vmstat: VmStat = Default::default();

        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let mut items = line.split_whitespace();
            if let Some(item) = items.next() {
                match item {
                    "pgpgin" => vmstat.pgpgin = parse_item!(path, items.next(), i64, line)?,
                    "pgpgout" => vmstat.pgpgout = parse_item!(path, items.next(), i64, line)?,
                    "pswpin" => vmstat.pswpin = parse_item!(path, items.next(), i64, line)?,
                    "pswpout" => vmstat.pswpout = parse_item!(path, items.next(), i64, line)?,
                    "pgsteal_kswapd" => {
                        vmstat.pgsteal_kswapd = parse_item!(path, items.next(), i64, line)?
                    }
                    "pgsteal_direct" => {
                        vmstat.pgsteal_direct = parse_item!(path, items.next(), i64, line)?
                    }
                    "pgscan_kswapd" => {
                        vmstat.pgscan_kswapd = parse_item!(path, items.next(), i64, line)?
                    }
                    "pgscan_direct" => {
                        vmstat.pgscan_direct = parse_item!(path, items.next(), i64, line)?
                    }
                    "oom_kill" => vmstat.oom_kill = parse_item!(path, items.next(), i64, line)?,
                    _ => {}
                }
            }
        }

        if vmstat == Default::default() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(vmstat)
        }
    }

    fn read_pid_stat_from_path<P: AsRef<Path>>(&self, path: P) -> Result<PidStat> {
        let path = path.as_ref().join("stat");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let mut buf_reader = BufReader::new(file);
        let mut pidstat: PidStat = Default::default();

        let mut line = String::new();
        buf_reader
            .read_line(&mut line)
            .map_err(|e| Error::IoError(path.clone(), e))?;

        {
            let b_opt = line.find('(');
            let e_opt = line.rfind(')');
            if let (Some(b), Some(e)) = (b_opt, e_opt) {
                pidstat.comm = Some(line[b + 1..e].to_string());
                line.replace_range(b..e + 1, "");
            }
        }

        let items: Vec<_> = line.split_whitespace().collect();
        pidstat.pid = parse_item!(path, items.get(0), i32, line)?;
        if let Some(c) = parse_item!(path, items.get(1), char, line)? {
            if let Some(state) = PidState::from_char(c) {
                pidstat.state = Some(state);
            } else {
                return Err(Error::InvalidPidState(path, c));
            }
        }
        pidstat.ppid = parse_item!(path, items.get(2), i32, line)?;
        pidstat.pgrp = parse_item!(path, items.get(3), i32, line)?;
        pidstat.session = parse_item!(path, items.get(4), i32, line)?;
        pidstat.minflt = parse_item!(path, items.get(8), i64, line)?;
        pidstat.majflt = parse_item!(path, items.get(10), i64, line)?;
        pidstat.user_usecs = parse_usec!(path, items.get(12), line)?;
        pidstat.system_usecs = parse_usec!(path, items.get(13), line)?;
        pidstat.num_threads = parse_item!(path, items.get(18), i64, line)?;

        let uptime = self.read_uptime_secs()?;
        pidstat.running_secs = parse_sec!(path, items.get(20), line)?
            .map(|running_secs_since_boot| uptime - running_secs_since_boot);

        pidstat.rss_bytes =
            parse_item!(path, items.get(22), i64, line)?.map(|pages| pages * *PAGE_SIZE);
        pidstat.processor = parse_item!(path, items.get(37), i32, line)?;

        if pidstat == Default::default() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(pidstat)
        }
    }

    pub fn read_pid_stat(&self, pid: u32) -> Result<PidStat> {
        self.read_pid_stat_from_path(self.path.join(pid.to_string()))
    }

    fn read_pid_io_from_path<P: AsRef<Path>>(path: P) -> Result<PidIo> {
        let path = path.as_ref().join("io");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut pidio: PidIo = Default::default();

        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let mut items = line.split_whitespace();
            if let Some(item) = items.next() {
                match item {
                    "read_bytes:" => pidio.rbytes = parse_item!(path, items.next(), i64, line)?,
                    "write_bytes:" => pidio.wbytes = parse_item!(path, items.next(), i64, line)?,
                    _ => {}
                }
            }
        }

        if pidio == Default::default() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(pidio)
        }
    }

    pub fn read_pid_io(&self, pid: u32) -> Result<PidIo> {
        Self::read_pid_io_from_path(self.path.join(pid.to_string()))
    }

    fn read_pid_cgroup_from_path<P: AsRef<Path>>(path: P) -> Result<String> {
        let path = path.as_ref().join("cgroup");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;
            if line.len() > 3 && line.starts_with("0::") {
                return Ok(line[3..].to_string());
            }
        }

        Err(Error::InvalidFileFormat(path))
    }

    pub fn read_pid_cgroup(&self, pid: u32) -> Result<String> {
        Self::read_pid_cgroup_from_path(self.path.join(pid.to_string()))
    }

    pub fn read_all_pids(&self) -> Result<PidMap> {
        let mut pidmap: PidMap = Default::default();
        for entry in
            std::fs::read_dir(&self.path).map_err(|e| Error::IoError(self.path.clone(), e))?
        {
            let entry = match entry {
                Err(ref e)
                    if e.raw_os_error()
                        .map_or(false, |ec| ec == 2 || ec == 3 /* ENOENT or ESRCH */) =>
                {
                    continue
                }
                ent => ent.map_err(|e| Error::IoError(self.path.clone(), e))?,
            };
            match entry.metadata() {
                Ok(ref m) if m.is_dir() => {}
                _ => continue,
            }

            let mut is_pid = true;
            for c in entry.file_name().to_string_lossy().chars() {
                if !c.is_ascii_digit() {
                    is_pid = false;
                    break;
                }
            }
            if !is_pid {
                continue;
            }

            let mut pidinfo: PidInfo = Default::default();

            match self.read_pid_stat_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error()
                        .map_or(false, |ec| ec == 2 || ec == 3 /* ENOENT or ESRCH */) =>
                {
                    continue
                }
                res => pidinfo.stat = res?,
            }

            match Self::read_pid_io_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error().map_or(false, |ec| {
                        ec == 2 || ec == 3 /* ENOENT or ESRCH */
                    }) =>
                {
                    continue
                }
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error().map_or(false, |ec| {
                        /* EACCES (b/c /proc/pid/io requires elevated
                         * perms due to security concerns). Just leave
                         * io info empty */
                        ec == 13
                    }) => {}
                res => pidinfo.io = res?,
            }

            match Self::read_pid_cgroup_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error()
                        .map_or(false, |ec| ec == 2 || ec == 3 /* ENOENT or ESRCH */) =>
                {
                    continue
                }
                res => pidinfo.cgroup = res?,
            }

            let file_name = entry.file_name();
            let pid_str = file_name.to_string_lossy();
            let pid = pid_str.parse::<i32>().map_err(|_| Error::ParseError {
                line: String::new(),
                item: pid_str.to_string(),
                type_name: "pid".to_string(),
                path: self.path.clone(),
            })?;
            pidmap.insert(pid, pidinfo);
        }

        if pidmap == Default::default() {
            Err(Error::InvalidFileFormat(self.path.clone()))
        } else {
            Ok(pidmap)
        }
    }
}

pub trait PidStateExt {
    fn from_char(c: char) -> Option<PidState>;
    fn as_char(&self) -> Option<char>;
}

impl PidStateExt for PidState {
    fn from_char(c: char) -> Option<PidState> {
        match c {
            'R' => Some(PidState::RUNNING),
            'S' => Some(PidState::SLEEPING),
            'D' => Some(PidState::DISK_SLEEP),
            'Z' => Some(PidState::ZOMBIE),
            'T' => Some(PidState::STOPPED),
            't' => Some(PidState::TRACING_STOPPED),
            'x' | 'X' => Some(PidState::DEAD),
            'I' => Some(PidState::IDLE),
            'P' => Some(PidState::PARKED),
            _ => None,
        }
    }

    fn as_char(&self) -> Option<char> {
        match *self {
            PidState::RUNNING => Some('R'),
            PidState::SLEEPING => Some('S'),
            PidState::DISK_SLEEP => Some('D'),
            PidState::ZOMBIE => Some('Z'),
            PidState::STOPPED => Some('T'),
            PidState::TRACING_STOPPED => Some('t'),
            PidState::DEAD => Some('X'),
            PidState::IDLE => Some('I'),
            PidState::PARKED => Some('P'),
            _ => None,
        }
    }
}
