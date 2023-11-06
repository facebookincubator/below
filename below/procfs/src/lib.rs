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

#![deny(clippy::all)]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use lazy_static::lazy_static;
use nix::sys;
use openat::Dir;
use slog::error;
use thiserror::Error;
use threadpool::ThreadPool;

use common::logutil::get_logger;

mod types;
pub use types::*;

#[cfg(test)]
mod test;

pub const NET_SYSFS: &str = "/sys/class/net/";
pub const NET_PROCFS: &str = "/proc/net";

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
    static ref PAGE_SIZE: u64 = {
        page_size()
    };
}

fn ticks_per_second() -> u64 {
    match unsafe { libc::sysconf(libc::_SC_CLK_TCK) } {
        -1 => panic!("Failed to query clock tick rate"),
        x => x as u64,
    }
}

fn page_size() -> u64 {
    match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
        -1 => panic!("Failed to query clock tick rate"),
        x => x as u64,
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
    #[error("Unexpected line ({1}) in file: {0:?}")]
    UnexpectedLine(PathBuf, String),
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
                    line: $line.to_string(),
                    item: s.to_string(),
                    type_name: stringify!($t).to_string(),
                    path: $path.to_path_buf(),
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
        parse_item!($path, $rhs, u64, $line).map(|opt| opt.map(|v| v * *MICROS_PER_TICK))
    };
}

macro_rules! parse_sec {
    ($path:expr, $rhs:expr, $line:ident) => {
        parse_item!($path, $rhs, u64, $line).map(|opt| opt.map(|v| v / *TICKS_PER_SECOND))
    };
}

macro_rules! parse_kb {
    ($path:expr, $rhs:expr, $line:ident) => {
        parse_item!($path, $rhs, u64, $line).map(|opt| opt.map(|v| v * 1024))
    };
}

pub struct ProcReader {
    path: PathBuf,
    threadpool: ThreadPool,
}

impl ProcReader {
    pub fn new() -> ProcReader {
        ProcReader {
            path: Path::new("/proc").to_path_buf(),
            // 5 threads max
            threadpool: ThreadPool::with_name("procreader_worker".to_string(), 5),
        }
    }

    pub fn new_with_custom_procfs(path: PathBuf) -> ProcReader {
        let mut reader = ProcReader::new();
        reader.path = path;
        reader
    }

    fn read_uptime_secs(&self) -> Result<u64> {
        let path = self.path.join("uptime");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let mut buf_reader = BufReader::new(file);
        let mut line = String::new();
        buf_reader
            .read_line(&mut line)
            .map_err(|e| Error::IoError(path.clone(), e))?;

        let mut items = line.split_whitespace();

        match parse_item!(path, items.next(), f64, line) {
            Ok(Some(uptime)) => Ok(uptime.round() as u64),
            Ok(None) => Err(Error::InvalidFileFormat(path)),
            Err(e) => Err(e),
        }
    }

    fn process_cpu_stat(path: &PathBuf, line: &String) -> Result<CpuStat> {
        // Format is like "cpu9 6124418 452468 3062529 230073290 216237 0 45647 0 0 0"
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

    pub fn read_kernel_version(&self) -> Result<String> {
        let path = self.path.join("sys/kernel/osrelease");
        match std::fs::read_to_string(&path) {
            Ok(kernel_version) => Ok(kernel_version.trim_matches('\n').trim().into()),
            Err(e) => Err(Error::IoError(path, e)),
        }
    }

    pub fn read_stat(&self) -> Result<Stat> {
        let path = self.path.join("stat");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut stat: Stat = Default::default();
        let mut cpus_map = BTreeMap::new();
        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let mut items = line.split_whitespace();
            if let Some(item) = items.next() {
                match item {
                    "intr" => {
                        stat.total_interrupt_count = parse_item!(&path, items.next(), u64, line)?
                    }
                    "ctxt" => stat.context_switches = parse_item!(&path, items.next(), u64, line)?,
                    "btime" => {
                        stat.boot_time_epoch_secs = parse_item!(&path, items.next(), u64, line)?
                    }
                    "processes" => {
                        stat.total_processes = parse_item!(&path, items.next(), u64, line)?
                    }
                    "procs_running" => {
                        stat.running_processes = parse_item!(&path, items.next(), u32, line)?
                    }
                    "procs_blocked" => {
                        stat.blocked_processes = parse_item!(&path, items.next(), u32, line)?
                    }
                    x => {
                        if x == "cpu" {
                            stat.total_cpu = Some(Self::process_cpu_stat(&path, &line)?);
                        } else if let Some(cpu_suffix) = x.strip_prefix("cpu") {
                            let cpu_id =
                                parse_item!(&path, Some(cpu_suffix.to_owned()), u32, line)?
                                    .unwrap();
                            let existing =
                                cpus_map.insert(cpu_id, Self::process_cpu_stat(&path, &line)?);
                            if existing.is_some() {
                                return Err(Error::UnexpectedLine(path, line));
                            }
                        }
                    }
                }
            }
        }
        if !cpus_map.is_empty() {
            stat.cpus_map = Some(cpus_map);
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
                        meminfo.total_huge_pages = parse_item!(path, items.next(), u64, line)?
                    }
                    "HugePages_Free:" => {
                        meminfo.free_huge_pages = parse_item!(path, items.next(), u64, line)?
                    }
                    "Hugepagesize:" => {
                        meminfo.huge_page_size = parse_kb!(path, items.next(), line)?
                    }
                    "Hugetlb:" => meminfo.hugetlb = parse_kb!(path, items.next(), line)?,
                    "CmaTotal:" => meminfo.cma_total = parse_kb!(path, items.next(), line)?,
                    "CmaFree:" => meminfo.cma_free = parse_kb!(path, items.next(), line)?,
                    "VmallocTotal:" => meminfo.vmalloc_total = parse_kb!(path, items.next(), line)?,
                    "VmallocUsed:" => meminfo.vmalloc_used = parse_kb!(path, items.next(), line)?,
                    "VmallocChunk:" => meminfo.vmalloc_chunk = parse_kb!(path, items.next(), line)?,
                    "DirectMap4k:" => meminfo.direct_map_4k = parse_kb!(path, items.next(), line)?,
                    "DirectMap2M:" => meminfo.direct_map_2m = parse_kb!(path, items.next(), line)?,
                    "DirectMap1G:" => meminfo.direct_map_1g = parse_kb!(path, items.next(), line)?,
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
                    "pgpgin" => vmstat.pgpgin = parse_item!(path, items.next(), u64, line)?,
                    "pgpgout" => vmstat.pgpgout = parse_item!(path, items.next(), u64, line)?,
                    "pswpin" => vmstat.pswpin = parse_item!(path, items.next(), u64, line)?,
                    "pswpout" => vmstat.pswpout = parse_item!(path, items.next(), u64, line)?,
                    "pgsteal_kswapd" => {
                        vmstat.pgsteal_kswapd = parse_item!(path, items.next(), u64, line)?
                    }
                    "pgsteal_direct" => {
                        vmstat.pgsteal_direct = parse_item!(path, items.next(), u64, line)?
                    }
                    "pgscan_kswapd" => {
                        vmstat.pgscan_kswapd = parse_item!(path, items.next(), u64, line)?
                    }
                    "pgscan_direct" => {
                        vmstat.pgscan_direct = parse_item!(path, items.next(), u64, line)?
                    }
                    "oom_kill" => vmstat.oom_kill = parse_item!(path, items.next(), u64, line)?,
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

    fn read_disk_fsinfo(&self, mount_info: &MountInfo) -> Option<(f32, u64)> {
        if let Some(mount_point) = &mount_info.mount_point {
            if let Ok(stat) = sys::statvfs::statvfs(Path::new(&mount_point)) {
                let disk_usage = ((stat.blocks() - stat.blocks_available()) as f32 * 100.0)
                    / stat.blocks() as f32;
                let partition_size = stat.blocks() as u64 * stat.block_size() as u64;
                return Some((disk_usage, partition_size));
            }
        }
        None
    }

    fn read_mount_info_map(&self) -> Result<HashMap<String, MountInfo>> {
        // Map contains a MountInfo object corresponding to the first
        // mount of each mount source. The first mount is what shows in
        // the 'df' command and reflects the usage of the device.
        let path = self.path.join("self/mountinfo");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut mount_info_map: HashMap<String, MountInfo> = HashMap::new();

        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;
            if let Ok(mount_info) = self.process_mount_info(&path, &line) {
                if let Some(mount_source) = mount_info.mount_source.clone() {
                    mount_info_map.entry(mount_source).or_insert(mount_info);
                }
            }
        }

        if mount_info_map.is_empty() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(mount_info_map)
        }
    }

    fn process_mount_info(&self, path: &Path, line: &str) -> Result<MountInfo> {
        let mut items = line.split_whitespace();
        let mut mount_info = MountInfo {
            mnt_id: parse_item!(path, items.next(), i32, line)?,
            parent_mnt_id: parse_item!(path, items.next(), i32, line)?,
            majmin: parse_item!(path, items.next(), String, line)?,
            root: parse_item!(path, items.next(), String, line)?,
            mount_point: parse_item!(path, items.next(), String, line)?,
            mount_options: parse_item!(path, items.next(), String, line)?,
            ..Default::default()
        };

        let mut items = items.skip(2);
        mount_info.fs_type = parse_item!(path, items.next(), String, line)?;
        mount_info.mount_source = parse_item!(path, items.next(), String, line)?;

        if mount_info == Default::default() {
            Err(Error::InvalidFileFormat(path.to_path_buf()))
        } else {
            Ok(mount_info)
        }
    }

    pub fn read_disk_stats_and_fsinfo(&self) -> Result<DiskMap> {
        let path = self.path.join("diskstats");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut disk_map: DiskMap = Default::default();
        let mount_info_map = self.read_mount_info_map().unwrap_or_default();

        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let stats_vec: Vec<&str> = line.split(' ').filter(|item| !item.is_empty()).collect();
            let mut stats_iter = stats_vec.iter();
            let mut disk_stat = DiskStat {
                major: parse_item!(path, stats_iter.next(), u64, line)?,
                ..Default::default()
            };
            if disk_stat.major.is_none() {
                continue;
            }
            disk_stat.minor = parse_item!(path, stats_iter.next(), u64, line)?;

            disk_stat.name = parse_item!(path, stats_iter.next(), String, line)?;

            let disk_name = disk_stat.name.as_ref().unwrap().to_string();

            disk_stat.read_completed = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.read_merged = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.read_sectors = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.time_spend_read_ms = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.write_completed = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.write_merged = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.write_sectors = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.time_spend_write_ms = parse_item!(path, stats_iter.next(), u64, line)?;
            let mut stats_iter = stats_iter.skip(3);
            disk_stat.discard_completed = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.discard_merged = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.discard_sectors = parse_item!(path, stats_iter.next(), u64, line)?;
            disk_stat.time_spend_discard_ms = parse_item!(path, stats_iter.next(), u64, line)?;

            let device_path = format!("/dev/{}", disk_name);
            if let Some(mount_info) = mount_info_map.get(&device_path) {
                if let Some((disk_usage, partition_size)) = self.read_disk_fsinfo(mount_info) {
                    disk_stat.disk_usage = Some(disk_usage);
                    disk_stat.partition_size = Some(partition_size);
                }
                disk_stat.filesystem_type = mount_info.fs_type.clone();
            }

            disk_map.insert(disk_name, disk_stat);
        }

        if disk_map.is_empty() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(disk_map)
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
        pidstat.minflt = parse_item!(path, items.get(8), u64, line)?;
        pidstat.majflt = parse_item!(path, items.get(10), u64, line)?;
        pidstat.user_usecs = parse_usec!(path, items.get(12), line)?;
        pidstat.system_usecs = parse_usec!(path, items.get(13), line)?;
        pidstat.num_threads = parse_item!(path, items.get(18), u64, line)?;

        let uptime = self.read_uptime_secs()?;
        pidstat.running_secs = parse_sec!(path, items.get(20), line)?
            .map(|running_secs_since_boot| (uptime - running_secs_since_boot) as u64);

        pidstat.rss_bytes =
            parse_item!(path, items.get(22), u64, line)?.map(|pages| pages * *PAGE_SIZE);
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

    pub fn read_tid_stat(&self, tid: u32) -> Result<PidStat> {
        let mut p = self.path.join(tid.to_string());
        p.push("task");
        p.push(tid.to_string());
        self.read_pid_stat_from_path(p)
    }

    fn read_pid_status_from_path<P: AsRef<Path>>(&self, path: P) -> Result<PidStatus> {
        let path = path.as_ref().join("status");

        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        let mut pidstatus: PidStatus = Default::default();

        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;

            let mut items = line.split(':');
            if let Some(item) = items.next() {
                let mut values = items.flat_map(|s| s.split_whitespace());
                match item {
                    "NStgid" => {
                        pidstatus.ns_tgid = Some(values.filter_map(|s| s.parse().ok()).collect());
                    }
                    "VmSize" => pidstatus.vm_size = parse_kb!(path, values.next(), line)?,
                    "VmLck" => pidstatus.lock = parse_kb!(path, values.next(), line)?,
                    "VmPin" => pidstatus.pin = parse_kb!(path, values.next(), line)?,
                    "RssAnon" => pidstatus.anon = parse_kb!(path, values.next(), line)?,
                    "RssFile" => pidstatus.file = parse_kb!(path, values.next(), line)?,
                    "RssShmem" => pidstatus.shmem = parse_kb!(path, values.next(), line)?,
                    "VmPTE" => pidstatus.pte = parse_kb!(path, values.next(), line)?,
                    "VmSwap" => pidstatus.swap = parse_kb!(path, values.next(), line)?,
                    "HugetlbPages" => pidstatus.huge_tlb = parse_kb!(path, values.next(), line)?,
                    _ => {}
                }
            }
        }

        Ok(pidstatus)
    }

    pub fn read_pid_mem(&self, pid: u32) -> Result<PidStatus> {
        self.read_pid_status_from_path(self.path.join(pid.to_string()))
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
                    "read_bytes:" => pidio.rbytes = parse_item!(path, items.next(), u64, line)?,
                    "write_bytes:" => pidio.wbytes = parse_item!(path, items.next(), u64, line)?,
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

        let mut cgroup_path = None;
        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;
            // Lines contain three colon separated fields:
            //   hierarchy-ID:controller-list:cgroup-path
            // A line starting with "0::" would be an entry for cgroup v2.
            // Otherwise, the line containing "pids" controller is what we want
            // for cgroup v1.
            let parts: Vec<_> = line.splitn(3, ':').collect();
            if parts.len() == 3 {
                if parts[0] == "0" && parts[1] == "" {
                    cgroup_path = Some(parts[2].to_owned());
                    // cgroup v2 takes precedence
                    break;
                } else if parts[1].split(',').any(|c| c == "pids") {
                    cgroup_path = Some(parts[2].to_owned());
                }
            }
        }
        cgroup_path.ok_or_else(|| Error::InvalidFileFormat(path))
    }

    pub fn read_pid_cgroup(&self, pid: u32) -> Result<String> {
        Self::read_pid_cgroup_from_path(self.path.join(pid.to_string()))
    }

    pub fn read_pid_cmdline(&mut self, pid: u32) -> Result<Option<Vec<String>>> {
        self.read_pid_cmdline_from_path(self.path.join(pid.to_string()))
    }

    fn read_pid_cmdline_from_path_blocking<P: AsRef<Path>>(path: P) -> Result<Option<Vec<String>>> {
        let path = path.as_ref().join("cmdline");
        let mut file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let mut buf = Vec::new();
        match file
            .read_to_end(&mut buf)
            .map_err(|e| Error::IoError(path.clone(), e))?
        {
            // It's a zombie process and those don't have cmdlines
            0 => Ok(None),
            _ => {
                Ok(Some(
                    buf
                        // /proc/pid/cmdline is split by nul bytes
                        .split(|c| *c == 0)
                        .filter(|s| !s.is_empty())
                        .map(|s| {
                            // Choose not to error on invalid utf8 b/c it's a process's
                            // right to do crazy things if they want. No need for us to
                            // erorr on it.
                            String::from_utf8_lossy(s).to_string()
                        })
                        .collect::<Vec<String>>(),
                ))
            }
        }
    }

    /// Do /proc/pid/cmdline reads off-thread in a threadpool b/c kernel needs
    /// to take the target process's mmap_sem semaphore and could block for a long
    /// time. This way, we don't suffer a priority inversion (b/c this crate can be
    /// run from a high priority binary).
    fn read_pid_cmdline_from_path<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<Option<Vec<String>>> {
        let path = path.as_ref().to_owned();
        let (tx, rx) = channel();
        self.threadpool.execute(move || {
            // This is OK to ignore. cmdline receiver hanging up is expected
            // after timeout.
            let _ = tx.send(Self::read_pid_cmdline_from_path_blocking(path));
        });

        // 20ms should be more than enough for an in-memory procfs read and also high enough for a
        // page fault
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(c) => c,
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => panic!("cmdline sender hung up"),
        }
    }

    fn read_pid_exe_path_from_path<P: AsRef<Path>>(path: P) -> Result<String> {
        let path = path.as_ref().join("exe");
        std::fs::read_link(path.clone())
            .map_err(|e| Error::IoError(path, e))
            .map(|p| p.to_string_lossy().into_owned())
    }

    pub fn read_pid_exe_path(&self, pid: u32) -> Result<String> {
        Self::read_pid_exe_path_from_path(self.path.join(pid.to_string()))
    }

    pub fn read_all_pids(&mut self) -> Result<PidMap> {
        let mut pidmap: PidMap = Default::default();
        for entry in
            std::fs::read_dir(&self.path).map_err(|e| Error::IoError(self.path.clone(), e))?
        {
            let entry = match entry {
                Err(ref e)
                    if e.raw_os_error()
                        .map_or(false, |ec| ec == 2 || ec == 3 /* ENOENT or ESRCH */) =>
                {
                    continue;
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
                    continue;
                }
                res => pidinfo.stat = res?,
            }

            match self.read_pid_status_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error()
                        .map_or(false, |ec| ec == 2 || ec == 3 /* ENOENT or ESRCH */) =>
                {
                    continue;
                }
                res => pidinfo.status = res?,
            }

            match Self::read_pid_io_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error().map_or(false, |ec| {
                        ec == 2 || ec == 3 /* ENOENT or ESRCH */
                    }) =>
                {
                    continue;
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
                    continue;
                }
                res => pidinfo.cgroup = res?,
            }

            match self.read_pid_cmdline_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error()
                        .map_or(false, |ec| ec == 2 || ec == 3 /* ENOENT or ESRCH */) =>
                {
                    continue;
                }
                res => pidinfo.cmdline_vec = res?,
            }

            // Swallow the error since reading the /proc/pid/exe
            // 1. will need root permission to trace link
            // 2. Even with root permission, some exe will have broken link, kworker for example.
            if let Ok(s) = Self::read_pid_exe_path_from_path(entry.path()) {
                pidinfo.exe_path = Some(s);
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
            'R' => Some(PidState::Running),
            'S' => Some(PidState::Sleeping),
            'D' => Some(PidState::UninterruptibleSleep),
            'Z' => Some(PidState::Zombie),
            'T' => Some(PidState::Stopped),
            't' => Some(PidState::TracingStopped),
            'x' | 'X' => Some(PidState::Dead),
            'I' => Some(PidState::Idle),
            'P' => Some(PidState::Parked),
            _ => None,
        }
    }

    fn as_char(&self) -> Option<char> {
        match *self {
            PidState::Running => Some('R'),
            PidState::Sleeping => Some('S'),
            PidState::UninterruptibleSleep => Some('D'),
            PidState::Zombie => Some('Z'),
            PidState::Stopped => Some('T'),
            PidState::TracingStopped => Some('t'),
            PidState::Dead => Some('X'),
            PidState::Idle => Some('I'),
            PidState::Parked => Some('P'),
        }
    }
}

macro_rules! parse_interface_stats {
    ($net_stat:ident, $dir:ident, $cur_path: ident, $($stat:ident),*) => {
        $($net_stat.$stat = Self::read_iface_stat(&$dir, &$cur_path, stringify!($stat))?);*
    }
}

macro_rules! get_val_from_stats_map {
    ($map:ident, $stat_item:ident {$($field:ident: $key:tt,)*}) => {
        $stat_item {
            $($field: $map.get($key).map(|x| *x)),*
        }
    }
}

pub struct NetReader {
    interface_dir: Dir,
    proc_net_dir: Dir,
}

impl NetReader {
    pub fn new() -> Result<NetReader> {
        Self::new_with_custom_path(NET_SYSFS.into(), NET_PROCFS.into())
    }

    pub fn new_with_custom_path(
        interface_path: PathBuf,
        proc_net_path: PathBuf,
    ) -> Result<NetReader> {
        let interface_dir =
            Dir::open(&interface_path).map_err(|e| Error::IoError(interface_path, e))?;
        let proc_net_dir =
            Dir::open(&proc_net_path).map_err(|e| Error::IoError(proc_net_path, e))?;

        Ok(NetReader {
            interface_dir,
            proc_net_dir,
        })
    }

    fn read_iface_stat(
        interface_dir: &Dir,
        cur_path: &PathBuf,
        stat_item: &str,
    ) -> Result<Option<u64>> {
        let file = match interface_dir.open_file(stat_item) {
            Ok(f) => f,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(None);
                } else {
                    return Err(Error::IoError(cur_path.join(stat_item), e));
                }
            }
        };
        let buf_reader = BufReader::new(file);
        match buf_reader.lines().next() {
            Some(line) => {
                let line = line.map_err(|e| Error::IoError(cur_path.join(stat_item), e))?;
                line.parse::<u64>()
                    .map(Some)
                    .map_err(move |_| Error::UnexpectedLine(cur_path.join(stat_item), line))
            }
            None => Err(Error::InvalidFileFormat(cur_path.join(stat_item))),
        }
    }

    fn read_all_iface_stats(&self, interface: &str, cur_path: &PathBuf) -> Result<InterfaceStat> {
        let interface_dir = self
            .interface_dir
            .read_link(interface)
            .map_err(|e| Error::IoError(cur_path.clone(), e))?;
        let stats_dir = self
            .interface_dir
            .sub_dir(interface_dir.as_path())
            .map_err(|e| Error::IoError(interface_dir, e))?
            .sub_dir("statistics")
            .map_err(|e| Error::IoError(cur_path.clone(), e))?;
        let cur_path = cur_path.join(interface).join("statistics");
        let mut net_stat: InterfaceStat = Default::default();
        parse_interface_stats!(
            net_stat,
            stats_dir,
            cur_path,
            collisions,
            multicast,
            rx_bytes,
            rx_compressed,
            rx_crc_errors,
            rx_dropped,
            rx_errors,
            rx_fifo_errors,
            rx_frame_errors,
            rx_length_errors,
            rx_missed_errors,
            rx_nohandler,
            rx_over_errors,
            rx_packets,
            tx_aborted_errors,
            tx_bytes,
            tx_carrier_errors,
            tx_compressed,
            tx_dropped,
            tx_errors,
            tx_fifo_errors,
            tx_heartbeat_errors,
            tx_packets,
            tx_window_errors
        );
        Ok(net_stat)
    }

    fn read_net_map(&self) -> Result<NetMap> {
        let mut netmap: NetMap = Default::default();
        let cur_path = self
            .interface_dir
            .recover_path()
            .unwrap_or_else(|_| NET_SYSFS.into());

        for entry in self
            .interface_dir
            .list_dir(".")
            .map_err(|e| Error::IoError(cur_path.clone(), e))?
            .filter_map(|entry| match entry {
                Ok(e) => Some(e),
                _ => None,
            })
        {
            let interface = entry.file_name().to_string_lossy();
            let netstat = self.read_all_iface_stats(&interface, &cur_path)?;
            netmap.insert(interface.into(), netstat);
        }

        if netmap == Default::default() {
            Err(Error::InvalidFileFormat(cur_path))
        } else {
            Ok(netmap)
        }
    }

    // format like /proc/net/netstat. Key will be in "{title}_{field}" format
    fn read_kv_diff_line(&self, stats_filename: &str) -> Result<BTreeMap<String, u64>> {
        let cur_path = self
            .proc_net_dir
            .recover_path()
            .unwrap_or_else(|_| NET_PROCFS.into())
            .join(stats_filename);
        let stats_file = self
            .proc_net_dir
            .open_file(stats_filename)
            .map_err(|e| Error::IoError(cur_path.clone(), e))?;

        let buf_reader = BufReader::new(stats_file);
        let content: Vec<String> = buf_reader
            .lines()
            .filter_map(|line| match line {
                Ok(l) => Some(l),
                _ => None,
            })
            .collect();

        let mut res = BTreeMap::new();
        for topic in content.chunks(2) {
            let fields: Vec<&str> = topic[0].split(':').collect();
            let vals: Vec<&str> = topic[1].split(':').collect();

            if fields.len() != 2 || vals.len() != 2 || fields.len() != vals.len() {
                return Err(Error::InvalidFileFormat(cur_path));
            }

            let key_header = fields[0];
            let keys: Vec<&str> = fields[1].split_whitespace().collect();
            let vals: Vec<&str> = vals[1].split_whitespace().collect();

            if keys.is_empty() && keys.len() != vals.len() {
                return Err(Error::InvalidFileFormat(cur_path));
            }

            for (&k, &v) in keys.iter().zip(vals.iter()) {
                // There's case that stats like max_conn may have -1 value that represent no max. It's
                // safe to skip such value and keep the result as None.
                if v.starts_with('-') {
                    continue;
                }

                res.insert(
                    format!("{}_{}", key_header, &k),
                    v.parse::<u64>().map_err(|_| Error::ParseError {
                        line: k.into(),
                        item: v.into(),
                        type_name: "u64".into(),
                        path: cur_path.clone(),
                    })?,
                );
            }
        }

        Ok(res)
    }

    fn read_kv_same_line(&self, stats_filename: &str) -> Result<BTreeMap<String, u64>> {
        let cur_path = self
            .proc_net_dir
            .recover_path()
            .unwrap_or_else(|_| NET_PROCFS.into())
            .join(stats_filename);
        let stats_file = self
            .proc_net_dir
            .open_file(stats_filename)
            .map_err(|e| Error::IoError(cur_path.clone(), e))?;
        let buf_reader = BufReader::new(stats_file);

        let mut res = BTreeMap::new();
        for line in buf_reader.lines() {
            let line = match line {
                Ok(l) => l,
                _ => continue,
            };

            let kv = line.split_whitespace().collect::<Vec<&str>>();
            if kv.len() != 2 {
                return Err(Error::InvalidFileFormat(cur_path));
            }

            res.insert(
                kv[0].into(),
                kv[1].parse::<u64>().map_err(|_| Error::ParseError {
                    line: kv[0].to_string(),
                    item: kv[1].into(),
                    type_name: "u64".into(),
                    path: cur_path.clone(),
                })?,
            );
        }

        Ok(res)
    }

    fn read_tcp_stat(snmp_map: &BTreeMap<String, u64>) -> TcpStat {
        get_val_from_stats_map!(
            snmp_map,
            TcpStat {
                active_opens: "Tcp_ActiveOpens",
                passive_opens: "Tcp_PassiveOpens",
                attempt_fails: "Tcp_AttemptFails",
                estab_resets: "Tcp_EstabResets",
                curr_estab: "Tcp_CurrEstab",
                in_segs: "Tcp_InSegs",
                out_segs: "Tcp_OutSegs",
                retrans_segs: "Tcp_RetransSegs",
                in_errs: "Tcp_InErrs",
                out_rsts: "Tcp_OutRsts",
                in_csum_errors: "Tcp_InCsumErrors",
            }
        )
    }

    fn read_tcp_ext_stat(netstat_map: &BTreeMap<String, u64>) -> TcpExtStat {
        get_val_from_stats_map!(
            netstat_map,
            TcpExtStat {
                syncookies_sent: "TcpExt_SyncookiesSent",
                syncookies_recv: "TcpExt_SyncookiesRecv",
                syncookies_failed: "TcpExt_SyncookiesFailed",
                embryonic_rsts: "TcpExt_EmbryonicRsts",
                prune_called: "TcpExt_PruneCalled",
                tw: "TcpExt_TW",
                paws_estab: "TcpExt_PAWSEstab",
                delayed_acks: "TcpExt_DelayedACKs",
                delayed_ack_locked: "TcpExt_DelayedACKLocked",
                delayed_ack_lost: "TcpExt_DelayedACKLost",
                listen_overflows: "TcpExt_ListenOverflows",
                listen_drops: "TcpExt_ListenDrops",
                tcp_hp_hits: "TcpExt_TCPHPHits",
                tcp_pure_acks: "TcpExt_TCPPureAcks",
                tcp_hp_acks: "TcpExt_TCPHPAcks",
                tcp_reno_recovery: "TcpExt_TCPRenoRecovery",
                tcp_reno_reorder: "TcpExt_TCPRenoReorder",
                tcp_ts_reorder: "TcpExt_TCPTSReorder",
                tcp_full_undo: "TcpExt_TCPFullUndo",
                tcp_partial_undo: "TcpExt_TCPPartialUndo",
                tcp_dsack_undo: "TcpExt_TCPDSACKUndo",
                tcp_loss_undo: "TcpExt_TCPLossUndo",
                tcp_lost_retransmit: "TcpExt_TCPLostRetransmit",
                tcp_reno_failures: "TcpExt_TCPRenoFailures",
                tcp_loss_failures: "TcpExt_TCPLossFailures",
                tcp_fast_retrans: "TcpExt_TCPFastRetrans",
                tcp_slow_start_retrans: "TcpExt_TCPSlowStartRetrans",
                tcp_timeouts: "TcpExt_TCPTimeouts",
            }
        )
    }

    fn read_ip_ext_stat(netstat_map: &BTreeMap<String, u64>) -> IpExtStat {
        get_val_from_stats_map!(
            netstat_map,
            IpExtStat {
                in_mcast_pkts: "IpExt_InMcastPkts",
                out_mcast_pkts: "IpExt_OutMcastPkts",
                in_bcast_pkts: "IpExt_InBcastPkts",
                out_bcast_pkts: "IpExt_OutBcastPkts",
                in_octets: "IpExt_InOctets",
                out_octets: "IpExt_OutOctets",
                in_mcast_octets: "IpExt_InMcastOctets",
                out_mcast_octets: "IpExt_OutMcastOctets",
                in_bcast_octets: "IpExt_InBcastOctets",
                out_bcast_octets: "IpExt_OutBcastOctets",
                in_no_ect_pkts: "IpExt_InNoECTPkts",
            }
        )
    }

    fn read_ip_stat(snmp_map: &BTreeMap<String, u64>) -> IpStat {
        get_val_from_stats_map!(
            snmp_map,
            IpStat {
                forwarding: "Ip_Forwarding",
                in_receives: "Ip_InReceives",
                forw_datagrams: "Ip_ForwDatagrams",
                in_discards: "Ip_InDiscards",
                in_delivers: "Ip_InDelivers",
                out_requests: "Ip_OutRequests",
                out_discards: "Ip_OutDiscards",
                out_no_routes: "Ip_OutNoRoutes",
            }
        )
    }

    fn read_ip6_stat(snmp6_map: &BTreeMap<String, u64>) -> Ip6Stat {
        get_val_from_stats_map!(
            snmp6_map,
            Ip6Stat {
                in_receives: "Ip6InReceives",
                in_hdr_errors: "Ip6InHdrErrors",
                in_no_routes: "Ip6InNoRoutes",
                in_addr_errors: "Ip6InAddrErrors",
                in_discards: "Ip6InDiscards",
                in_delivers: "Ip6InDelivers",
                out_forw_datagrams: "Ip6OutForwDatagrams",
                out_requests: "Ip6OutRequests",
                out_no_routes: "Ip6OutNoRoutes",
                in_mcast_pkts: "Ip6InMcastPkts",
                out_mcast_pkts: "Ip6OutMcastPkts",
                in_octets: "Ip6InOctets",
                out_octets: "Ip6OutOctets",
                in_mcast_octets: "Ip6InMcastOctets",
                out_mcast_octets: "Ip6OutMcastOctets",
                in_bcast_octets: "Ip6InBcastOctets",
                out_bcast_octets: "Ip6OutBcastOctets",
            }
        )
    }

    fn read_icmp_stat(snmp_map: &BTreeMap<String, u64>) -> IcmpStat {
        get_val_from_stats_map!(
            snmp_map,
            IcmpStat {
                in_msgs: "Icmp_InMsgs",
                in_errors: "Icmp_InErrors",
                in_dest_unreachs: "Icmp_InDestUnreachs",
                out_msgs: "Icmp_OutMsgs",
                out_errors: "Icmp_OutErrors",
                out_dest_unreachs: "Icmp_OutDestUnreachs",
            }
        )
    }

    fn read_icmp6_stat(snmp6_map: &BTreeMap<String, u64>) -> Icmp6Stat {
        get_val_from_stats_map!(
            snmp6_map,
            Icmp6Stat {
                in_msgs: "Icmp6InMsgs",
                in_errors: "Icmp6InErrors",
                out_msgs: "Icmp6OutMsgs",
                out_errors: "Icmp6OutErrors",
                in_dest_unreachs: "Icmp6InDestUnreachs",
                out_dest_unreachs: "Icmp6OutDestUnreachs",
            }
        )
    }

    fn read_udp_stat(snmp_map: &BTreeMap<String, u64>) -> UdpStat {
        get_val_from_stats_map!(
            snmp_map,
            UdpStat {
                in_datagrams: "Udp_InDatagrams",
                no_ports: "Udp_NoPorts",
                in_errors: "Udp_InErrors",
                out_datagrams: "Udp_OutDatagrams",
                rcvbuf_errors: "Udp_RcvbufErrors",
                sndbuf_errors: "Udp_SndbufErrors",
                ignored_multi: "Udp_IgnoredMulti",
            }
        )
    }

    fn read_udp6_stat(snmp6_map: &BTreeMap<String, u64>) -> Udp6Stat {
        get_val_from_stats_map!(
            snmp6_map,
            Udp6Stat {
                in_datagrams: "Udp6InDatagrams",
                no_ports: "Udp6NoPorts",
                in_errors: "Udp6InErrors",
                out_datagrams: "Udp6OutDatagrams",
                rcvbuf_errors: "Udp6RcvbufErrors",
                sndbuf_errors: "Udp6SndbufErrors",
                in_csum_errors: "Udp6InCsumErrors",
                ignored_multi: "Udp6IgnoredMulti",
            }
        )
    }

    pub fn read_netstat(&self) -> Result<NetStat> {
        let logger = get_logger();
        let netstat_map = self
            .read_kv_diff_line("netstat")
            .map_err(|err| error!(logger, "Failed to read netstat: {:?}", err))
            .ok();
        let snmp_map = self
            .read_kv_diff_line("snmp")
            .map_err(|err| error!(logger, "Failed to read snmp stats: {:?}", err))
            .ok();
        let snmp6_map = self
            .read_kv_same_line("snmp6")
            .map_err(|err| error!(logger, "Failed to read snmp6 stats: {:?}", err))
            .ok();
        let iface_map = self
            .read_net_map()
            .map_err(|err| error!(logger, "Failed to read interface stats: {:?}", err))
            .ok();

        Ok(NetStat {
            interfaces: iface_map,
            tcp: snmp_map.as_ref().map(Self::read_tcp_stat),
            tcp_ext: netstat_map.as_ref().map(Self::read_tcp_ext_stat),
            ip: snmp_map.as_ref().map(Self::read_ip_stat),
            ip_ext: netstat_map.as_ref().map(Self::read_ip_ext_stat),
            ip6: snmp6_map.as_ref().map(Self::read_ip6_stat),
            icmp: snmp_map.as_ref().map(Self::read_icmp_stat),
            icmp6: snmp6_map.as_ref().map(Self::read_icmp6_stat),
            udp: snmp_map.as_ref().map(Self::read_udp_stat),
            udp6: snmp6_map.as_ref().map(Self::read_udp6_stat),
        })
    }
}
