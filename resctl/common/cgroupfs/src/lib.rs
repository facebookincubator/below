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
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use openat::{AsPath, Dir, SimpleType};
use thiserror::Error;

pub use cgroupfs_thrift::types::{
    CpuPressure, CpuStat, IoPressure, IoStat, MemoryEvents, MemoryPressure, MemoryStat, Pressure,
    PressureMetrics,
};

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
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct CgroupReader {
    relative_path: PathBuf,
    dir: Dir,
}

impl CgroupReader {
    pub fn new(root: PathBuf) -> Result<CgroupReader> {
        CgroupReader::new_with_relative_path(root, PathBuf::from(OsStr::new("")))
    }

    pub fn new_with_relative_path(root: PathBuf, relative_path: PathBuf) -> Result<CgroupReader> {
        let mut path = root.clone();
        match relative_path.strip_prefix("/") {
            Ok(p) => path.push(p),
            _ => path.push(&relative_path),
        };
        let dir = Dir::open(&path).map_err(|e| Error::IoError(path, e))?;
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

    /// Read a stat from a file that has a single line
    fn read_singleline_stat_file(&self, file_name: &str) -> Result<u64> {
        let file = self
            .dir
            .open_file(file_name)
            .map_err(|e| self.io_error(file_name, e))?;
        let buf_reader = BufReader::new(file);
        for line in buf_reader.lines() {
            let line = line.map_err(|e| self.io_error(file_name, e))?;
            return line
                .parse::<u64>()
                .map_err(move |_| self.unexpected_line(file_name, line));
        }
        Err(self.invalid_file_format(file_name))
    }

    /// Read memory.current - returning current cgroup memory
    /// consumption in bytes
    pub fn read_memory_current(&self) -> Result<u64> {
        self.read_singleline_stat_file("memory.current")
    }

    /// Read memory.high - returning memory.high consumption in bytes
    /// Will return -1 if the content is max
    /// Will return None if the file is missing
    pub fn read_memory_high(&self) -> Result<Option<i64>> {
        match self.read_singleline_stat_file("memory.high") {
            Ok(v) => Ok(Some(v as i64)),
            Err(Error::IoError(_, e)) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(Error::UnexpectedLine(_, line)) if line.starts_with("max") => Ok(Some(-1)),
            Err(e) => Err(e),
        }
    }

    /// Read memory.swap.current - returning current cgroup memory
    /// swap consumption in bytes
    pub fn read_memory_swap_current(&self) -> Result<u64> {
        self.read_singleline_stat_file("memory.swap.current")
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

    /// Read cpu.pressure
    pub fn read_cpu_pressure(&self) -> Result<CpuPressure> {
        let file_name = "cpu.pressure";
        let mut pressure = PressureMetrics::read(&self, file_name)?;
        let some_pressure = pressure
            .remove("some")
            .ok_or_else(|| self.invalid_file_format(file_name))?;

        Ok(CpuPressure {
            some: some_pressure,
        })
    }

    /// Read io.pressure
    pub fn read_io_pressure(&self) -> Result<IoPressure> {
        let file_name = "io.pressure";
        let mut pressure = PressureMetrics::read(&self, file_name)?;
        let some_pressure = pressure
            .remove("some")
            .ok_or_else(|| self.invalid_file_format(file_name))?;
        let full_pressure = pressure
            .remove("full")
            .ok_or_else(|| self.invalid_file_format(file_name))?;
        Ok(IoPressure {
            some: some_pressure,
            full: full_pressure,
        })
    }

    /// Read memory.pressure
    pub fn read_memory_pressure(&self) -> Result<MemoryPressure> {
        let file_name = "memory.pressure";
        let mut pressure = PressureMetrics::read(&self, file_name)?;
        let some_pressure = pressure
            .remove("some")
            .ok_or_else(|| self.invalid_file_format(file_name))?;
        let full_pressure = pressure
            .remove("full")
            .ok_or_else(|| self.invalid_file_format(file_name))?;
        Ok(MemoryPressure {
            some: some_pressure,
            full: full_pressure,
        })
    }

    /// Read all pressure metrics
    pub fn read_pressure(&self) -> Result<Pressure> {
        Ok(Pressure {
            cpu: self.read_cpu_pressure()?,
            io: self.read_io_pressure()?,
            memory: self.read_memory_pressure()?,
        })
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
                    let val = items[1].parse::<u64>().map_err(|_| r.unexpected_line(file_name, line.clone()))? as i64;
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

// Trait to add a read() method for `<string> key=value` formatted files
trait NameKVRead: Sized {
    fn read<P: AsRef<Path> + AsPath + Clone>(
        r: &CgroupReader,
        file_name: P,
    ) -> Result<BTreeMap<String, Self>>;
}

macro_rules! name_key_equal_value_format {
    ($struct:ident; $allows_empty:expr; [ $($field:ident,)+ ]) => (
        impl NameKVRead for $struct {
            fn read<P: AsRef<Path> + AsPath + Clone>(r: &CgroupReader, file_name: P) -> Result<BTreeMap<String, $struct>> {
                let mut map = BTreeMap::new();
                let file = r.dir.open_file(file_name.clone()).map_err(|e| r.io_error(file_name.clone(), e))?;
                let buf_reader = BufReader::new(file);
                for line in buf_reader.lines() {
                    let line = line.map_err(|e| r.io_error(file_name.clone(), e))?;
                    let items = line.split_whitespace().collect::<Vec<_>>();
                    // as an example, io.stat looks like:
                    // 253:0 rbytes=531745786880 wbytes=1623798909952 ...
                    let mut s = $struct::default();
                    for item in items.iter().skip(1) {
                        let kv = item.split("=").collect::<Vec<_>>();
                        if kv.len() != 2 {
                            return Err(r.invalid_file_format(file_name));
                        }
                        let key = kv[0];
                        match key.as_ref() {
                            $(stringify!($field) => s.$field = Some(
                                kv[1].parse().map_err(|_| r.unexpected_line(file_name.clone(), line.clone()))?
                            ),)*
                            _ => (),
                        };
                    }
                    if s == $struct::default() {
                        return Err(r.invalid_file_format(file_name))
                    }
                    map.insert(items[0].to_string(), s);
                }
                if !$allows_empty && map.is_empty() {
                     Err(r.invalid_file_format(file_name))
                } else {
                    Ok(map)
                }
            }
        }
    );
}

name_key_equal_value_format!(IoStat; true; [
    rbytes,
    wbytes,
    rios,
    wios,
    dbytes,
    dios,
]);

name_key_equal_value_format!(PressureMetrics; false; [
    avg10,
    avg60,
    avg300,
    total,
]);
