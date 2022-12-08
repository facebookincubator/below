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
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use nix::sys::statfs::fstatfs;
use nix::sys::statfs::CGROUP2_SUPER_MAGIC;
use openat::AsPath;
use openat::Dir;
use openat::SimpleType;
use thiserror::Error;

mod types;
pub use types::*;

#[cfg(test)]
mod test;

pub const DEFAULT_CG_ROOT: &str = "/sys/fs/cgroup";

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file format: {0:?}")]
    InvalidFileFormat(PathBuf),
    #[error("{1:?}: {0:?}")]
    IoError(PathBuf, #[source] std::io::Error),
    #[error("Unexpected line ({1}) in file: {0:?}")]
    UnexpectedLine(PathBuf, String),
    #[error("Not cgroup2 filesystem: {0:?}")]
    NotCgroup2(PathBuf),
    #[error("Pressure metrics not supported: {0:?}")]
    PressureNotSupported(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct CgroupReader {
    relative_path: PathBuf,
    dir: Dir,
}

macro_rules! impl_read_pressure {
    ( $fn:ident, $e:expr, $typ:tt, FullPressureSupported ) => {
        /// Read $typ
        pub fn $fn(&self) -> Result<$typ> {
            let (some_pressure, full_pressure_opt, file_name) =
                impl_read_pressure!(Internal, &self, $e);
            let full_pressure =
                full_pressure_opt.ok_or_else(|| self.invalid_file_format(file_name))?;
            Ok($typ {
                some: some_pressure,
                full: full_pressure,
            })
        }
    };
    ( $fn:ident, $e:expr, $typ:tt, FullPressureMaybeSupported ) => {
        /// Read $typ
        pub fn $fn(&self) -> Result<$typ> {
            let (some_pressure, full_pressure_opt, _) = impl_read_pressure!(Internal, &self, $e);
            Ok($typ {
                some: some_pressure,
                full: full_pressure_opt,
            })
        }
    };
    ( Internal, $self:expr, $e:expr) => {{
        let file_name = concat!($e, ".pressure");
        let mut pressure = PressureMetrics::read($self, file_name)?;
        (
            pressure
                .remove("some")
                .ok_or_else(|| $self.invalid_file_format(file_name))?,
            pressure.remove("full"),
            file_name,
        )
    }};
}

macro_rules! parse_and_set_fields {
    ($struct:expr; $key:expr; $value:expr; [ $($field:ident,)+  ]) => (
        match $key {
            $(stringify!($field) => $struct.$field = Some($value),)*
            _ => (),
        }
    )
}

impl CgroupReader {
    pub fn new(root: PathBuf) -> Result<CgroupReader> {
        CgroupReader::new_with_relative_path(root, PathBuf::from(OsStr::new("")))
    }

    pub fn new_with_relative_path(root: PathBuf, relative_path: PathBuf) -> Result<CgroupReader> {
        CgroupReader::new_with_relative_path_inner(root, relative_path, true)
    }

    fn new_with_relative_path_inner(
        root: PathBuf,
        relative_path: PathBuf,
        validate: bool,
    ) -> Result<CgroupReader> {
        let mut path = root.clone();
        match relative_path.strip_prefix("/") {
            Ok(p) => path.push(p),
            _ => path.push(&relative_path),
        };
        let dir = Dir::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;

        // Check that it's a cgroup2 fs
        if validate {
            let statfs = match fstatfs(&dir) {
                Ok(s) => s,
                Err(e) => {
                    return Err(Error::IoError(
                        path,
                        std::io::Error::new(ErrorKind::Other, format!("Failed to fstatfs: {}", e)),
                    ));
                }
            };

            if statfs.filesystem_type() != CGROUP2_SUPER_MAGIC {
                return Err(Error::NotCgroup2(path));
            }
        }

        Ok(CgroupReader { relative_path, dir })
    }

    pub fn root() -> Result<CgroupReader> {
        CgroupReader::new(Path::new(DEFAULT_CG_ROOT).to_path_buf())
    }

    /// Returns the cgroup name (e.g. the path relative to the cgroup root)
    /// Invoking this on the root cgroup will return an empty path
    pub fn name(&self) -> &Path {
        &self.relative_path
    }

    pub fn read_inode_number(&self) -> Result<u64> {
        let meta = self
            .dir
            .metadata(".")
            .map_err(|e| self.io_error(self.dir.recover_path().unwrap_or_else(|_| "".into()), e))?;
        Ok(meta.stat().st_ino as u64)
    }

    /// Read a value from a file that has a single line. If the file is empty,
    /// the value will be derived from an empty string.
    fn read_empty_or_singleline_file<T: FromStr>(&self, file_name: &str) -> Result<T> {
        let file = self
            .dir
            .open_file(file_name)
            .map_err(|e| self.io_error(file_name, e))?;
        let buf_reader = BufReader::new(file);
        let line = buf_reader
            .lines()
            .next()
            .unwrap_or_else(|| Ok("".to_owned()));
        let line = line.map_err(|e| self.io_error(file_name, e))?;
        line.parse::<T>()
            .map_err(move |_| self.unexpected_line(file_name, line))
    }

    /// Read a value from a file that has a single line. If the file is empty,
    /// InvalidFileFormat is returned.
    fn read_singleline_file<T: FromStr>(&self, file_name: &str) -> Result<T> {
        let file = self
            .dir
            .open_file(file_name)
            .map_err(|e| self.io_error(file_name, e))?;
        let buf_reader = BufReader::new(file);
        if let Some(line) = buf_reader.lines().next() {
            let line = line.map_err(|e| self.io_error(file_name, e))?;
            return line
                .parse::<T>()
                .map_err(move |_| self.unexpected_line(file_name, line));
        }
        Err(self.invalid_file_format(file_name))
    }

    /// Read a stat from a file that has a single non-negative integer or "max"
    /// line. Will return -1 if the context is "max".
    fn read_singleline_integer_or_max_stat_file(&self, file_name: &str) -> Result<i64> {
        match self.read_singleline_file::<u64>(file_name) {
            Ok(v) => Ok(v as i64),
            Err(Error::UnexpectedLine(_, line)) if line.starts_with("max") => Ok(-1),
            Err(e) => Err(e),
        }
    }

    /// Read a single line from a file representing a space separated list of
    /// cgroup controllers.
    fn read_singleline_controllers(&self, file_name: &str) -> Result<BTreeSet<String>> {
        let s = self.read_empty_or_singleline_file::<String>(file_name)?;
        if s.is_empty() {
            return Ok(BTreeSet::new());
        }
        Ok(s.split(' ').map(String::from).collect())
    }

    /// Read cgroup.controllers
    pub fn read_cgroup_controllers(&self) -> Result<BTreeSet<String>> {
        self.read_singleline_controllers("cgroup.controllers")
    }

    /// Read cgroup.subtree_control
    pub fn read_cgroup_subtree_control(&self) -> Result<BTreeSet<String>> {
        self.read_singleline_controllers("cgroup.subtree_control")
    }

    /// Read memory.low - returning memory.low limit in bytes
    /// Will return -1 if the content is max
    pub fn read_memory_low(&self) -> Result<i64> {
        self.read_singleline_integer_or_max_stat_file("memory.low")
    }

    /// Read memory.high - returning memory.high limit in bytes
    /// Will return -1 if the content is max
    pub fn read_memory_high(&self) -> Result<i64> {
        self.read_singleline_integer_or_max_stat_file("memory.high")
    }

    /// Read memory.max - returning memory.max max in bytes
    /// Will return -1 if the content is max
    pub fn read_memory_max(&self) -> Result<i64> {
        self.read_singleline_integer_or_max_stat_file("memory.max")
    }

    /// Read memory.swap.max - returning memory.swap.max max in bytes
    /// Will return -1 if the content is max
    pub fn read_memory_swap_max(&self) -> Result<i64> {
        self.read_singleline_integer_or_max_stat_file("memory.swap.max")
    }

    /// Read memory.zswap.max - returning memory.zswap.max max in bytes
    /// Will return -1 if the content is max
    pub fn read_memory_zswap_max(&self) -> Result<i64> {
        self.read_singleline_integer_or_max_stat_file("memory.zswap.max")
    }

    /// Read memory.current - returning current cgroup memory
    /// consumption in bytes
    pub fn read_memory_current(&self) -> Result<u64> {
        self.read_singleline_file("memory.current")
    }

    /// Read memory.swap.current - returning current cgroup memory
    /// swap consumption in bytes
    pub fn read_memory_swap_current(&self) -> Result<u64> {
        self.read_singleline_file("memory.swap.current")
    }

    /// Read memory.zswap.current - returning current cgroup memory
    /// zswap consumption in bytes
    pub fn read_memory_zswap_current(&self) -> Result<u64> {
        self.read_singleline_file("memory.zswap.current")
    }

    /// Read cpu.stat - returning assorted cpu consumption statistics
    pub fn read_cpu_stat(&self) -> Result<CpuStat> {
        CpuStat::read(&self)
    }

    /// Read io.stat - returning assorted io consumption statistics
    pub fn read_io_stat(&self) -> Result<BTreeMap<String, IoStat>> {
        IoStat::read(&self, "io.stat")
    }

    /// Read memory.stat - returning assorted memory consumption
    /// statistics
    pub fn read_memory_stat(&self) -> Result<MemoryStat> {
        MemoryStat::read(&self)
    }

    pub fn read_memory_events(&self) -> Result<MemoryEvents> {
        MemoryEvents::read(&self)
    }

    pub fn read_cgroup_stat(&self) -> Result<CgroupStat> {
        CgroupStat::read(self)
    }

    /// Read cpu.weight
    pub fn read_cpu_weight(&self) -> Result<u32> {
        self.read_singleline_file::<u32>("cpu.weight")
    }

    impl_read_pressure!(
        read_cpu_pressure,
        "cpu",
        CpuPressure,
        FullPressureMaybeSupported
    );

    impl_read_pressure!(read_io_pressure, "io", IoPressure, FullPressureSupported);

    impl_read_pressure!(
        read_memory_pressure,
        "memory",
        MemoryPressure,
        FullPressureSupported
    );

    /// Read all pressure metrics
    pub fn read_pressure(&self) -> Result<Pressure> {
        Ok(Pressure {
            cpu: self.read_cpu_pressure()?,
            io: self.read_io_pressure()?,
            memory: self.read_memory_pressure()?,
        })
    }

    // Reads memory.numa_stat - the return value is a map from numa_node_id ->
    // memory breakdown
    pub fn read_memory_numa_stat(&self) -> Result<BTreeMap<u32, MemoryNumaStat>> {
        let mut s: BTreeMap<u32, MemoryNumaStat> = BTreeMap::new();
        let file_name = "memory.numa_stat";
        let file = self
            .dir
            .open_file(file_name)
            .map_err(|e| self.io_error(file_name, e))?;
        let buf_reader = BufReader::new(file);
        for line in buf_reader.lines() {
            let line = line.map_err(|e| self.io_error(file_name, e))?;
            let items = line.split_whitespace().collect::<Vec<_>>();
            // Need to have at least the field name + at least one N0=val item
            if items.len() < 2 {
                return Err(self.unexpected_line(file_name, line));
            }
            let field = items[0];
            for item in items.iter().skip(1) {
                let kv = item.split('=').collect::<Vec<_>>();
                if kv.len() != 2 || kv[0].len() < 2 || !kv[0].starts_with('N') {
                    return Err(self.unexpected_line(file_name, line));
                }
                let mut kchars = kv[0].chars();
                kchars.next();
                let numa_node = kchars
                    .as_str()
                    .parse::<u32>()
                    .map_err(|_| self.unexpected_line(file_name, line.clone()))?;
                parse_and_set_fields!(
                    s.entry(numa_node).or_default();
                    field;
                    kv[1].parse().map_err(|_| self.unexpected_line(file_name, line.clone()))?;
                    [
                        anon,
                        file,
                        kernel_stack,
                        pagetables,
                        shmem,
                        file_mapped,
                        file_dirty,
                        file_writeback,
                        swapcached,
                        anon_thp,
                        file_thp,
                        shmem_thp,
                        inactive_anon,
                        active_anon,
                        inactive_file,
                        active_file,
                        unevictable,
                        slab_reclaimable,
                        slab_unreclaimable,
                        workingset_refault_anon,
                        workingset_refault_file,
                        workingset_activate_anon,
                        workingset_activate_file,
                        workingset_restore_anon,
                        workingset_restore_file,
                        workingset_nodereclaim,
                    ]
                );
            }
        }

        if s.is_empty() {
            return Err(self.invalid_file_format(file_name));
        }

        for (_node, memory_numa_stat) in s.iter() {
            if *memory_numa_stat == MemoryNumaStat::default() {
                return Err(self.invalid_file_format(file_name));
            }
        }

        Ok(s)
    }

    /// Return an iterator over child cgroups
    pub fn child_cgroup_iter(&self) -> Result<impl Iterator<Item = CgroupReader> + '_> {
        Ok(self
            .dir
            .list_dir(".")
            .map_err(|e| self.io_error("", e))?
            .filter_map(move |entry| match entry {
                Ok(entry) if entry.simple_type() == Some(SimpleType::Dir) => {
                    let dir = match self.dir.sub_dir(entry.file_name()) {
                        Ok(d) => d,
                        Err(_) => return None,
                    };
                    let mut relative_path = self.relative_path.clone();
                    relative_path.push(entry.file_name());
                    Some(CgroupReader { relative_path, dir })
                }
                _ => None,
            }))
    }

    fn invalid_file_format<P: AsRef<Path>>(&self, file_name: P) -> Error {
        let mut p = self.relative_path.clone();
        p.push(file_name);
        Error::InvalidFileFormat(p)
    }

    fn io_error<P: AsRef<Path>>(&self, file_name: P, e: std::io::Error) -> Error {
        let mut p = self.relative_path.clone();
        p.push(file_name);
        Error::IoError(p, e)
    }

    fn unexpected_line<P: AsRef<Path>>(&self, file_name: P, line: String) -> Error {
        let mut p = self.relative_path.clone();
        p.push(file_name);
        Error::UnexpectedLine(p, line)
    }

    fn pressure_not_supported<P: AsRef<Path>>(&self, file_name: P) -> Error {
        let mut p = self.relative_path.clone();
        p.push(file_name);
        Error::PressureNotSupported(p)
    }
}

// Trait to add a read() method for `key value` formatted files
trait KVRead: Sized {
    fn read(reader: &CgroupReader) -> Result<Self>;
}

// This macro generates the read() method for the given struct, file
// name, and keys. If a line does not exist in the file then the
// corresponding field is left as `None`. If lines include fields that
// are not listed, they are ignored.
macro_rules! key_values_format {
    ($struct:ident; $file:expr; [ $( $field:ident ),+ ]) => (
        impl KVRead for $struct {
            fn read(r: &CgroupReader) -> Result<$struct> {
                let mut s = $struct::default();
                let file_name = stringify!($file);
                let file = r.dir.open_file(file_name).map_err(|e| r.io_error(file_name, e))?;
                let buf_reader = BufReader::new(file);
                for line in buf_reader.lines() {
                    let line = line.map_err(|e| r.io_error(file_name, e))?;
                    let items = line.split_whitespace().collect::<Vec<_>>();
                    if items.len() != 2 {
                        return Err(r.unexpected_line(file_name, line));
                    }
                    let key = items[0];
                    let val = items[1].parse::<_>().map_err(|_| r.unexpected_line(file_name, line.clone()))?;
                    match key.as_ref() {
                        $(stringify!($field) => s.$field = Some(val),)*
                        _ => (),
                    };
                }
                if s == $struct::default() {
                    Err(r.invalid_file_format(file_name))
                } else {
                    Ok(s)
                }
            }
        }
    )
}

key_values_format!(CpuStat; cpu.stat; [
    usage_usec,
    user_usec,
    system_usec,
    nr_periods,
    nr_throttled,
    throttled_usec
]);

key_values_format!(MemoryStat; memory.stat; [
    anon,
    file,
    kernel_stack,
    slab,
    sock,
    shmem,
    file_mapped,
    file_dirty,
    file_writeback,
    anon_thp,
    inactive_anon,
    active_anon,
    inactive_file,
    active_file,
    unevictable,
    slab_reclaimable,
    slab_unreclaimable,
    pgfault,
    pgmajfault,
    workingset_refault,
    workingset_activate,
    workingset_nodereclaim,
    pgrefill,
    pgscan,
    pgsteal,
    pgactivate,
    pgdeactivate,
    pglazyfree,
    pglazyfreed,
    thp_fault_alloc,
    thp_collapse_alloc
]);

key_values_format!(MemoryEvents; memory.events; [
    low,
    high,
    max,
    oom,
    oom_kill
]);

key_values_format!(CgroupStat; cgroup.stat; [nr_descendants, nr_dying_descendants]);

// Trait to add a read() method for `<string> key=value` formatted files
trait NameKVRead: Sized {
    fn read<P: AsRef<Path> + AsPath + Clone>(
        r: &CgroupReader,
        file_name: P,
    ) -> Result<BTreeMap<String, Self>>;
}

struct AllowsEmpty(bool);
struct AllowsPressureEOpNotSupp(bool);

macro_rules! name_key_equal_value_format {
    ($struct:ident; $allows_empty:expr; $allows_pressure_eopnotsupp:expr; [ $($field:ident,)+ ]) => (
        impl NameKVRead for $struct {
            fn read<P: AsRef<Path> + AsPath + Clone>(r: &CgroupReader, file_name: P) -> Result<BTreeMap<String, $struct>> {
                let mut map = BTreeMap::new();
                let file = r.dir.open_file(file_name.clone()).map_err(|e| r.io_error(file_name.clone(), e))?;
                let buf_reader = BufReader::new(file);
                for line in buf_reader.lines() {
                    let line = line.map_err(|e| {
                        // Capture a different error if pressure
                        // metrics aren't supported
                        if $allows_pressure_eopnotsupp.0 {
                            if let Some(errno) = e.raw_os_error() {
                                if errno == /* EOPNOTSUPP */ 95 {
                                    return r.pressure_not_supported(file_name.clone());
                                }
                            }
                        }
                        r.io_error(file_name.clone(), e)
                    })?;
                    let items = line.split_whitespace().collect::<Vec<_>>();
                    // as an example, io.stat looks like:
                    // 253:0 rbytes=531745786880 wbytes=1623798909952 ...
                    let mut s = $struct::default();
                    for item in items.iter().skip(1) {
                        let kv = item.split("=").collect::<Vec<_>>();
                        if kv.len() != 2 {
                            return Err(r.invalid_file_format(file_name));
                        }
                        // Certain keys such as cost.usage can not be struct fields so must use cost_usage
                        let key = kv[0].replace(".", "_");
                        parse_and_set_fields!(
                            s;
                            key.as_ref();
                            kv[1].parse().map_err(|_| r.unexpected_line(file_name.clone(), line.clone()))?;
                            [ $($field,)* ]
                        )
                    };
                    if s == $struct::default() {
                        return Err(r.invalid_file_format(file_name))
                    }
                    map.insert(items[0].to_string(), s);
                }
                if !$allows_empty.0 && map.is_empty() {
                     Err(r.invalid_file_format(file_name))
                } else {
                    Ok(map)
                }
            }
        }
    );
}

name_key_equal_value_format!(IoStat; AllowsEmpty(true); AllowsPressureEOpNotSupp(false); [
    rbytes,
    wbytes,
    rios,
    wios,
    dbytes,
    dios,
    cost_usage,
    cost_wait,
    cost_indebt,
    cost_indelay,
]);

name_key_equal_value_format!(PressureMetrics; AllowsEmpty(false); AllowsPressureEOpNotSupp(true); [
    avg10,
    avg60,
    avg300,
    total,
]);
