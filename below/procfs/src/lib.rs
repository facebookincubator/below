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
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;
use libc::CLOCK_BOOTTIME;
use libc::clock_gettime;
use libc::timespec;
use nix::sys;
use openat::Dir;
use openat::SimpleType;
use parking_lot::Condvar;
use parking_lot::Mutex;
use slog::debug;
use thiserror::Error;
use threadpool::ThreadPool;

mod types;
pub use types::*;

#[cfg(test)]
mod test;

use common::util;

pub const KSM_SYSFS: &str = "/sys/kernel/mm/ksm";
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
    #[error("Error when reading uptime")]
    UptimeError,
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
        let mut items = $line.split_ascii_whitespace();

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
    buffer: RefCell<Vec<u8>>,
}

impl Default for ProcReader {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcReader {
    pub fn new() -> ProcReader {
        ProcReader {
            path: Path::new("/proc").to_path_buf(),
            // 5 threads max
            threadpool: ThreadPool::with_name("procreader_worker".to_string(), 5),
            buffer: RefCell::new(Vec::new()),
        }
    }

    pub fn new_with_custom_procfs(path: PathBuf) -> ProcReader {
        let mut reader = ProcReader::new();
        reader.path = path;
        reader
    }

    fn read_file_to_str(&self, path: &Path) -> Result<RefMut<'_, str>> {
        File::open(path)
            .and_then(|file| util::read_kern_file_to_internal_buffer(&self.buffer, file))
            .map_err(|e| Error::IoError(path.to_path_buf(), e))
    }

    fn read_uptime_secs(&self) -> Result<u64> {
        unsafe {
            let mut ts: timespec = std::mem::zeroed();
            if clock_gettime(CLOCK_BOOTTIME, &mut ts) == 0 {
                Ok(ts.tv_sec as u64)
            } else {
                Err(Error::UptimeError)
            }
        }
    }

    fn process_cpu_stat(path: &Path, line: &str) -> Result<CpuStat> {
        // Format is like "cpu9 6124418 452468 3062529 230073290 216237 0 45647 0 0 0"
        let mut items = line.split_ascii_whitespace();
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
        let content = self.read_file_to_str(&path)?;
        Ok(content.trim_matches('\n').trim().into())
    }

    pub fn read_stat(&self) -> Result<Stat> {
        let path = self.path.join("stat");
        let content = self.read_file_to_str(&path)?;
        let mut stat: Stat = Default::default();
        let mut cpus_map = BTreeMap::new();

        for line in content.lines() {
            let mut items = line.split_ascii_whitespace();
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
                            stat.total_cpu = Some(Self::process_cpu_stat(&path, line)?);
                        } else if let Some(cpu_suffix) = x.strip_prefix("cpu") {
                            let cpu_id =
                                parse_item!(&path, Some(cpu_suffix.to_owned()), u32, line)?
                                    .unwrap();
                            let existing =
                                cpus_map.insert(cpu_id, Self::process_cpu_stat(&path, line)?);
                            if existing.is_some() {
                                return Err(Error::UnexpectedLine(path, line.to_string()));
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
        let content = self.read_file_to_str(&path)?;
        let mut meminfo: MemInfo = Default::default();

        for line in content.lines() {
            let mut items = line.split_ascii_whitespace();
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
        let content = self.read_file_to_str(&path)?;
        let mut vmstat: VmStat = Default::default();

        for line in content.lines() {
            let mut items = line.split_ascii_whitespace();
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

    pub fn read_slabinfo(&self) -> Result<Vec<SlabInfo>> {
        let path = self.path.join("slabinfo");
        let content = self.read_file_to_str(&path)?;
        let mut slab_info_vec = vec![];

        // The first line is version, second line is headers:
        //
        // slabinfo - version: 2.1
        // # name            <active_objs> <num_objs> <objsize> <objperslab> <pagesperslab> : tunables <limit> <batchcount> <sharedfactor> : slabdata <active_slabs> <num_slabs> <sharedavail>
        //
        for line in content.lines().skip(2) {
            let mut items = line.split_ascii_whitespace();
            let slab_info = SlabInfo {
                name: Some(items.next().unwrap().to_owned()),
                active_objs: parse_item!(path, items.next(), u64, line)?,
                num_objs: parse_item!(path, items.next(), u64, line)?,
                obj_size: parse_item!(path, items.next(), u64, line)?,
                obj_per_slab: parse_item!(path, items.next(), u64, line)?,
                pages_per_slab: parse_item!(path, items.next(), u64, line)?,
                active_slabs: parse_item!(path, items.nth(7), u64, line)?,
                num_slabs: parse_item!(path, items.next(), u64, line)?,
            };
            slab_info_vec.push(slab_info);
        }
        Ok(slab_info_vec)
    }

    fn read_disk_fsinfo(&self, mount_info: &MountInfo) -> Option<(f32, u64)> {
        if let Some(mount_point) = &mount_info.mount_point
            && let Ok(stat) = sys::statvfs::statvfs(Path::new(&mount_point))
        {
            let disk_usage =
                ((stat.blocks() - stat.blocks_available()) as f32 * 100.0) / stat.blocks() as f32;
            #[allow(clippy::unnecessary_cast)]
            let partition_size = stat.blocks() as u64 * stat.block_size() as u64;
            return Some((disk_usage, partition_size));
        }
        None
    }

    fn read_mount_info_map(&self) -> Result<HashMap<String, MountInfo>> {
        // Map contains a MountInfo object corresponding to the first
        // mount of each mount source. The first mount is what shows in
        // the 'df' command and reflects the usage of the device.
        let path = self.path.join("self/mountinfo");
        let content = self.read_file_to_str(&path)?.to_string();
        let mut mount_info_map: HashMap<String, MountInfo> = HashMap::new();

        for line in content.lines() {
            if let Ok(mount_info) = self.process_mount_info(&path, line)
                && let Some(mount_source) = mount_info.mount_source.clone()
            {
                mount_info_map.entry(mount_source).or_insert(mount_info);
            }
        }

        if mount_info_map.is_empty() {
            Err(Error::InvalidFileFormat(path))
        } else {
            Ok(mount_info_map)
        }
    }

    fn process_mount_info(&self, path: &Path, line: &str) -> Result<MountInfo> {
        let mut items = line.split_ascii_whitespace();
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
        let content = self.read_file_to_str(&path)?.to_string();
        let mut disk_map: DiskMap = Default::default();
        let mount_info_map = self.read_mount_info_map().unwrap_or_default();

        for line in content.lines() {
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
                disk_stat.filesystem_type.clone_from(&mount_info.fs_type)
            }

            let sysfs_path = format!(
                "/sys/dev/block/{}:{}",
                disk_stat.major.unwrap(),
                disk_stat.minor.unwrap()
            );
            // lsblk checks this file for partition
            // https://kernel.googlesource.com/pub/scm/utils/util-linux/util-linux/+/v2.25.2/lib/sysfs.c#322
            disk_stat.is_partition = Some(Path::new(&sysfs_path).join("start").exists());

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
        let content = self.read_file_to_str(&path)?;
        let mut pidstat: PidStat = Default::default();

        let mut line = content.to_string();
        {
            let b_opt = line.find('(');
            let e_opt = line.rfind(')');
            if let (Some(b), Some(e)) = (b_opt, e_opt) {
                pidstat.comm = Some(line[b + 1..e].to_string());
                line.replace_range(b..e + 1, "");
            }
        }

        for (index, item) in line.split_ascii_whitespace().enumerate() {
            match index {
                0 => pidstat.pid = parse_item!(path, Some(item), i32, line)?,
                1 => {
                    if let Some(c) = parse_item!(path, Some(item), char, line)? {
                        if let Some(state) = PidState::from_char(c) {
                            pidstat.state = Some(state);
                        } else {
                            return Err(Error::InvalidPidState(path, c));
                        }
                    }
                }
                2 => pidstat.ppid = parse_item!(path, Some(item), i32, line)?,
                3 => pidstat.pgrp = parse_item!(path, Some(item), i32, line)?,
                4 => pidstat.session = parse_item!(path, Some(item), i32, line)?,
                8 => pidstat.minflt = parse_item!(path, Some(item), u64, line)?,
                10 => pidstat.majflt = parse_item!(path, Some(item), u64, line)?,
                12 => pidstat.user_usecs = parse_usec!(path, Some(item), line)?,
                13 => pidstat.system_usecs = parse_usec!(path, Some(item), line)?,
                18 => pidstat.num_threads = parse_item!(path, Some(item), u64, line)?,
                20 => {
                    let uptime = self.read_uptime_secs()?;
                    pidstat.running_secs = parse_sec!(path, Some(item), line)?
                        .map(|running_secs_since_boot| uptime - running_secs_since_boot);
                }
                22 => {
                    pidstat.rss_bytes =
                        parse_item!(path, Some(item), u64, line)?.map(|pages| pages * *PAGE_SIZE)
                }
                37 => pidstat.processor = parse_item!(path, Some(item), i32, line)?,
                _ => {}
            }
        }

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
        let content = self.read_file_to_str(&path)?;
        let mut pidstatus: PidStatus = Default::default();

        for line in content.lines() {
            let mut items = line.split(':');
            if let Some(item) = items.next() {
                let mut values = items.flat_map(|s| s.split_ascii_whitespace());
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

    fn read_pid_io_from_path<P: AsRef<Path>>(&self, path: P) -> Result<PidIo> {
        let path = path.as_ref().join("io");
        let content = self.read_file_to_str(&path)?;
        let mut pidio: PidIo = Default::default();

        for line in content.lines() {
            let mut items = line.split_ascii_whitespace();
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
        self.read_pid_io_from_path(self.path.join(pid.to_string()))
    }

    fn read_pid_cgroup_from_path<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let path = path.as_ref().join("cgroup");
        let content = self.read_file_to_str(&path)?;

        let mut cgroup_path = None;
        for line in content.lines() {
            // Lines contain three colon separated fields:
            //   hierarchy-ID:controller-list:cgroup-path
            // A line starting with "0::" would be an entry for cgroup v2.
            // Otherwise, the line containing "pids" controller is what we want
            // for cgroup v1.
            let mut parts = line.splitn(3, ':');
            if let (Some(hierarchy_id), Some(controller_list), Some(path)) =
                (parts.next(), parts.next(), parts.next())
            {
                if hierarchy_id == "0" && controller_list.is_empty() {
                    return Ok(path.to_owned());
                } else if controller_list.split(',').any(|c| c == "pids") {
                    // Not return, since if cgroup v2 is found it takes precedence
                    cgroup_path = Some(path.to_owned());
                }
            }
        }
        cgroup_path.ok_or(Error::InvalidFileFormat(path))
    }

    pub fn read_pid_cgroup(&self, pid: u32) -> Result<String> {
        self.read_pid_cgroup_from_path(self.path.join(pid.to_string()))
    }

    pub fn read_pid_cmdline(&self, pid: u32) -> Result<Option<Vec<String>>> {
        self.read_pid_cmdline_from_path(self.path.join(pid.to_string()))
    }

    fn read_pid_cmdline_from_path_blocking<P: AsRef<Path>>(path: P) -> Result<Option<Vec<String>>> {
        let path = path.as_ref().join("cmdline");
        let mut file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let mut buf = [0; 4096];
        let bytes_read = file
            .read(&mut buf)
            .map_err(|e| Error::IoError(path.clone(), e))?;

        if bytes_read == 0 {
            // It's a zombie process and those don't have cmdlines
            return Ok(None);
        }

        Ok(Some(
            buf[..bytes_read]
                // /proc/pid/cmdline is split by nul bytes
                .split(|c| *c == 0)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    // It's a process' right to put invalid UTF8 here
                    String::from_utf8_lossy(s).to_string()
                })
                .collect::<Vec<String>>(),
        ))
    }

    /// Do /proc/pid/cmdline reads off-thread in a threadpool b/c kernel needs
    /// to take the target process's mmap_sem semaphore and could block for a long
    /// time. This way, we don't suffer a priority inversion (b/c this crate can be
    /// run from a high priority binary).
    fn read_pid_cmdline_from_path<P: AsRef<Path>>(&self, path: P) -> Result<Option<Vec<String>>> {
        // If the current number of active workers is equal to or exceeds the maximum, we avoid
        // enqueuing a new job to prevent unnecessary work since the task would likely time out.
        // This check is inherently racy: we might erroneously assume the pool is full when it
        // isn't, but not the reverse, because this is the only place where new tasks are scheduled
        // synchronously.
        if self.threadpool.active_count() >= self.threadpool.max_count() {
            return Ok(None);
        }

        let path = path.as_ref().to_owned();
        let cmdline_data = Arc::new((Mutex::new(None), Condvar::new()));
        let cmdline_data_clone = Arc::clone(&cmdline_data);

        self.threadpool.execute(move || {
            let result = Self::read_pid_cmdline_from_path_blocking(path);
            let (mutex, cvar) = &*cmdline_data_clone;
            *mutex.lock() = Some(result);
            cvar.notify_one();
        });

        let (mutex, cvar) = &*cmdline_data;
        let mut data_lock = mutex.lock();
        if data_lock.is_none() {
            // 20ms should be more than enough for an in-memory procfs read or a page fault
            cvar.wait_for(&mut data_lock, Duration::from_millis(20));
        }
        data_lock.take().unwrap_or(Ok(None))
    }

    fn read_pid_exe_path_from_path<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let path = path.as_ref().join("exe");
        std::fs::read_link(path.clone())
            .map_err(|e| Error::IoError(path, e))
            .map(|p| p.to_string_lossy().into_owned())
    }

    pub fn read_pid_exe_path(&self, pid: u32) -> Result<String> {
        self.read_pid_exe_path_from_path(self.path.join(pid.to_string()))
    }

    fn ascii_digits_to_i32(digits: &[u8]) -> Option<i32> {
        let mut result = 0i32;
        for digit in digits {
            let value = digit.wrapping_sub(b'0');
            if value <= 9 {
                result = result * 10 + value as i32;
            } else {
                return None;
            }
        }
        Some(result)
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
                    continue;
                }
                ent => ent.map_err(|e| Error::IoError(self.path.clone(), e))?,
            };

            if !entry
                .file_type()
                .map_err(|e| Error::IoError(self.path.clone(), e))?
                .is_dir()
            {
                continue;
            }

            let pid = match Self::ascii_digits_to_i32(entry.file_name().as_bytes()) {
                Some(pid) => pid,
                None => continue,
            };

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

            match self.read_pid_io_from_path(entry.path()) {
                Err(Error::IoError(_, ref e))
                    if e.raw_os_error().is_some_and(|ec| {
                        ec == 2 || ec == 3 /* ENOENT or ESRCH */
                    }) =>
                {
                    continue;
                }
                Err(Error::IoError(_, ref e))
                    // EACCES (b/c /proc/pid/io requires elevated perms due to security concerns). Just leave io info empty
                    if e.raw_os_error() == Some(13) => {}
                res => pidinfo.io = res?,
            }

            match self.read_pid_cgroup_from_path(entry.path()) {
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
            if let Ok(s) = self.read_pid_exe_path_from_path(entry.path()) {
                pidinfo.exe_path = Some(s);
            }

            pidmap.insert(pid, pidinfo);
        }

        if pidmap == Default::default() {
            Err(Error::InvalidFileFormat(self.path.clone()))
        } else {
            Ok(pidmap)
        }
    }

    pub fn read_sysctl_file<T: FromStr>(&self, directory: &str, key: &str) -> Option<T> {
        let path = self.path.join("sys").join(directory).join(key);
        let content = self.read_file_to_str(&path).ok()?;
        content.split('\n').next()?.parse::<T>().ok()
    }

    pub fn read_sysctl(&self) -> Sysctl {
        Sysctl {
            kernel_hung_task_detect_count: self
                .read_sysctl_file("kernel", "hung_task_detect_count"),
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
    logger: slog::Logger,
    interface_dir: Dir,
    proc_net_dir: Dir,
}

impl NetReader {
    pub fn new(logger: slog::Logger) -> Result<NetReader> {
        Self::new_with_custom_path(logger, NET_SYSFS.into(), NET_PROCFS.into())
    }

    pub fn new_with_custom_path(
        logger: slog::Logger,
        interface_path: PathBuf,
        proc_net_path: PathBuf,
    ) -> Result<NetReader> {
        let interface_dir =
            Dir::open(&interface_path).map_err(|e| Error::IoError(interface_path, e))?;
        let proc_net_dir =
            Dir::open(&proc_net_path).map_err(|e| Error::IoError(proc_net_path, e))?;

        Ok(NetReader {
            logger,
            interface_dir,
            proc_net_dir,
        })
    }

    fn read_iface_stat(
        interface_dir: &Dir,
        cur_path: &Path,
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

    fn read_all_iface_stats(&self, interface: &str, cur_path: &Path) -> Result<InterfaceStat> {
        let interface_dir = self
            .interface_dir
            .read_link(interface)
            .map_err(|e| Error::IoError(cur_path.to_path_buf(), e))?;
        let stats_dir = self
            .interface_dir
            .sub_dir(interface_dir.as_path())
            .map_err(|e| Error::IoError(interface_dir, e))?
            .sub_dir("statistics")
            .map_err(|e| Error::IoError(cur_path.to_path_buf(), e))?;
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
        {
            if let Ok(entry) = entry
                && entry.simple_type() == Some(SimpleType::Symlink)
            {
                let interface = entry.file_name().to_string_lossy();
                let netstat = self.read_all_iface_stats(&interface, &cur_path)?;
                netmap.insert(interface.into(), netstat);
            }
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
        let content: Vec<String> = buf_reader.lines().map_while(|line| line.ok()).collect();

        let mut res = BTreeMap::new();
        for topic in content.chunks(2) {
            let fields: Vec<&str> = topic[0].split(':').collect();
            let vals: Vec<&str> = topic[1].split(':').collect();

            if fields.len() != 2 || vals.len() != 2 || fields.len() != vals.len() {
                return Err(Error::InvalidFileFormat(cur_path));
            }

            let key_header = fields[0];
            let keys: Vec<&str> = fields[1].split_ascii_whitespace().collect();
            let vals: Vec<&str> = vals[1].split_ascii_whitespace().collect();

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

            let kv = line.split_ascii_whitespace().collect::<Vec<&str>>();
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
        // Any of these files could be missing, however unlikely.
        // An interface file could be missing if it is deleted while reading the directory.
        // For example, if ipv6 is disabled, /proc/net/snmp6 won't exist.
        // Similarly, one could even disable ipv4, in which case /proc/net/snmp won't exist.
        // Thus, we handle ENOENT errors by setting corresponding fields to `None`.
        let netstat_map = handle_enoent(&self.logger, self.read_kv_diff_line("netstat"))?;
        let snmp_map = handle_enoent(&self.logger, self.read_kv_diff_line("snmp"))?;
        let snmp6_map = handle_enoent(&self.logger, self.read_kv_same_line("snmp6"))?;
        let iface_map = handle_enoent(&self.logger, self.read_net_map())?;

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

pub struct KsmReader {
    path: PathBuf,
}

impl Default for KsmReader {
    fn default() -> Self {
        Self::new()
    }
}

impl KsmReader {
    pub fn new() -> KsmReader {
        KsmReader {
            path: Path::new(KSM_SYSFS).to_path_buf(),
        }
    }

    pub fn new_with_custom_path(path: PathBuf) -> KsmReader {
        KsmReader { path }
    }

    pub fn read_ksm(&self) -> Ksm {
        Ksm {
            advisor_max_cpu: self.read("advisor_max_cpu"),
            advisor_max_pages_to_scan: self.read("advisor_max_pages_to_scan"),
            advisor_min_pages_to_scan: self.read("advisor_min_pages_to_scan"),
            advisor_mode: self.read_selection("advisor_mode"),
            advisor_target_scan_time: self.read("advisor_target_scan_time"),
            full_scans: self.read("full_scans"),
            general_profit: self.read("general_profit"),
            ksm_zero_pages: self.read("ksm_zero_pages"),
            max_page_sharing: self.read("max_page_sharing"),
            merge_across_nodes: self.read("merge_across_nodes"),
            pages_scanned: self.read("pages_scanned"),
            pages_shared: self.read("pages_shared"),
            pages_sharing: self.read("pages_sharing"),
            pages_skipped: self.read("pages_skipped"),
            pages_to_scan: self.read("pages_to_scan"),
            pages_unshared: self.read("pages_unshared"),
            pages_volatile: self.read("pages_volatile"),
            run: self.read("run"),
            sleep_millisecs: self.read("sleep_millisecs"),
            smart_scan: self.read("smart_scan"),
            stable_node_chains: self.read("stable_node_chains"),
            stable_node_chains_prune_millisecs: self.read("stable_node_chains_prune_millisecs"),
            stable_node_dups: self.read("stable_node_dups"),
            use_zero_pages: self.read("use_zero_pages"),
        }
    }

    fn read<F>(&self, name: &str) -> Option<F>
    where
        F: FromStr,
    {
        std::fs::read_to_string(self.path.join(name))
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    // parses from string representing one selection out of some choices
    // i.e. "one [two] three" -> returns "two"
    fn read_selection(&self, name: &str) -> Option<String> {
        let val: String = self.read(name)?;
        let left_bracket_idx = val.find('[')?;
        let right_bracket_idx = val.rfind(']')?;
        if left_bracket_idx >= right_bracket_idx {
            return None;
        }
        Some(val[left_bracket_idx + 1..right_bracket_idx].to_string())
    }
}

/// Wraps the result into an `Option` if the result is not an error.
/// If the error is of type `ENOENT`, it is returned as `Ok(None)`.
/// Else, the error itself is returned.
fn handle_enoent<K, V>(
    logger: &slog::Logger,
    result: Result<BTreeMap<K, V>>,
) -> Result<Option<BTreeMap<K, V>>> {
    match result {
        Ok(map) => Ok(Some(map)),
        Err(Error::IoError(_, err)) if err.kind() == ErrorKind::NotFound => {
            debug!(logger, "{:?}", err);
            Ok(None)
        }
        Err(err) => Err(err),
    }
}
