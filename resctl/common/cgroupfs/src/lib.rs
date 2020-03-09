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
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use thiserror::Error;

pub use cgroupfs_thrift::types::{
    CpuPressure, CpuStat, IoPressure, IoStat, MemoryPressure, MemoryStat, Pressure, PressureMetrics,
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
    root: PathBuf,
    relative_path: PathBuf,
    path: PathBuf,
}

impl CgroupReader {
    pub fn new(root: PathBuf) -> CgroupReader {
        CgroupReader::new_with_relative_path(root, PathBuf::from(OsStr::new("")))
    }

    pub fn new_with_relative_path(root: PathBuf, relative_path: PathBuf) -> CgroupReader {
        let mut path = root.clone();
        match relative_path.strip_prefix("/") {
            Ok(p) => path.push(p),
            _ => path.push(&relative_path),
        };
        CgroupReader {
            root,
            relative_path,
            path,
        }
    }

    pub fn root() -> CgroupReader {
        CgroupReader::new(Path::new(DEFAULT_CG_ROOT).to_path_buf())
    }

    /// Returns the cgroup name (e.g. the path relative to the cgroup root)
    /// Invoking this on the root cgroup will return an empty path
    pub fn name(&self) -> &Path {
        &self.relative_path
    }

    /// Read memory.current - returning current cgroup memory
    /// consumption in bytes
    pub fn read_memory_current(&self) -> Result<u64> {
        let path = self.path.join("memory.current");
        let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        let buf_reader = BufReader::new(file);
        for line in buf_reader.lines() {
            let line = line.map_err(|e| Error::IoError(path.clone(), e))?;
            return line
                .parse()
                .map_err(move |_| Error::UnexpectedLine(path.clone(), line));
        }
        Err(Error::InvalidFileFormat(path))
    }

    /// Read cpu.stat - returning assorted cpu consumption statistics
    pub fn read_cpu_stat(&self) -> Result<CpuStat> {
        CpuStat::read(&self.path)
    }

    /// Read io.stat - returning assorted io consumption statistics
    pub fn read_io_stat(&self) -> Result<BTreeMap<String, IoStat>> {
        IoStat::read(&self.path.join("io.stat"))
    }

    /// Read memory.stat - returning assorted memory consumption
    /// statistics
    pub fn read_memory_stat(&self) -> Result<MemoryStat> {
        MemoryStat::read(&self.path)
    }

    /// Read cpu.pressure
    pub fn read_cpu_pressure(&self) -> Result<CpuPressure> {
        let path = self.path.join("cpu.pressure");
        let mut pressure = PressureMetrics::read(&path)?;
        let some_pressure = pressure
            .remove("some")
            .ok_or_else(|| Error::InvalidFileFormat(path))?;

        Ok(CpuPressure {
            some: some_pressure,
        })
    }

    /// Read io.pressure
    pub fn read_io_pressure(&self) -> Result<IoPressure> {
        let path = self.path.join("io.pressure");
        let mut pressure = PressureMetrics::read(&path)?;
        let some_pressure = pressure
            .remove("some")
            .ok_or_else(|| Error::InvalidFileFormat(path.clone()))?;
        let full_pressure = pressure
            .remove("full")
            .ok_or_else(|| Error::InvalidFileFormat(path))?;
        Ok(IoPressure {
            some: some_pressure,
            full: full_pressure,
        })
    }

    /// Read memory.pressure
    pub fn read_memory_pressure(&self) -> Result<MemoryPressure> {
        let path = self.path.join("memory.pressure");
        let mut pressure = PressureMetrics::read(&path)?;
        let some_pressure = pressure
            .remove("some")
            .ok_or_else(|| Error::InvalidFileFormat(path.clone()))?;
        let full_pressure = pressure
            .remove("full")
            .ok_or_else(|| Error::InvalidFileFormat(path))?;
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
        Ok(std::fs::read_dir(&self.path)
            .map_err(|e| Error::IoError(self.path.clone(), e))?
            .filter_map(move |dentry| match &dentry {
                Ok(d) if d.path().is_dir() => {
                    let mut relative_path = self.relative_path.clone();
                    relative_path.push(d.file_name());
                    Some(CgroupReader::new_with_relative_path(
                        self.root.clone(),
                        relative_path,
                    ))
                }
                _ => None,
            }))
    }
}

// Trait to add a read() method for `key value` formatted files
trait KVRead: Sized {
    fn read<P: AsRef<Path>>(cgroup_path: P) -> Result<Self>;
}

// This macro generates the read() method for the given struct, file
// name, and keys. If a line does not exist in the file then the
// corresponding field is left as `None`. If lines include fields that
// are not listed, they are ignored.
macro_rules! key_values_format {
    ($struct:ident; $file:expr; [ $( $field:ident ),+ ]) => (
        impl KVRead for $struct {
            fn read<P: AsRef<Path>>(cgroup_path: P) -> Result<$struct> {
                let mut s = $struct::default();
                let path = cgroup_path.as_ref().join(stringify!($file));
                let file = File::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
                let buf_reader = BufReader::new(file);
                for line in buf_reader.lines() {
                    let line = line.map_err(|e| Error::IoError(path.clone(), e))?;
                    let items = line.split_whitespace().collect::<Vec<_>>();
                    if items.len() != 2 {
                        return Err(Error::UnexpectedLine(path.clone(), line));
                    }
                    let key = items[0];
                    let val = items[1].parse::<u64>().map_err(|_| Error::UnexpectedLine(path.clone(), line.clone()))? as i64;
                    match key.as_ref() {
                        $(stringify!($field) => s.$field = Some(val),)*
                        _ => (),
                    };
                }
                if s == $struct::default() {
                    Err(Error::InvalidFileFormat(path))
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

// Trait to add a read() method for `<string> key=value` formatted files
trait NameKVRead: Sized {
    fn read(file_path: &PathBuf) -> Result<BTreeMap<String, Self>>;
}

macro_rules! name_key_equal_value_format {
    ($struct:ident; $allows_empty:expr; [ $($field:ident,)+ ]) => (
        impl NameKVRead for $struct {
            fn read(file_path: &PathBuf) -> Result<BTreeMap<String, $struct>> {
                let mut map = BTreeMap::new();
                let file = File::open(&file_path).map_err(|e| Error::IoError(file_path.clone(), e))?;
                let buf_reader = BufReader::new(file);
                for line in buf_reader.lines() {
                    let line = line.map_err(|e| Error::IoError(file_path.clone(), e))?;
                    let items = line.split_whitespace().collect::<Vec<_>>();
                    // as an example, io.stat looks like:
                    // 253:0 rbytes=531745786880 wbytes=1623798909952 ...
                    let mut s = $struct::default();
                    for item in items.iter().skip(1) {
                        let kv = item.split("=").collect::<Vec<_>>();
                        if kv.len() != 2 {
                            return Err(Error::InvalidFileFormat(file_path.clone()));
                        }
                        let key = kv[0];
                        match key.as_ref() {
                            $(stringify!($field) => s.$field = Some(
                                kv[1].parse().map_err(|_| Error::UnexpectedLine(file_path.clone(), line.clone()))?
                            ),)*
                            _ => (),
                        };
                    }
                    if s == $struct::default() {
                        return Err(Error::InvalidFileFormat(file_path.clone()))
                    }
                    map.insert(items[0].to_string(), s);
                }
                if !$allows_empty && map.is_empty() {
                     Err(Error::InvalidFileFormat(file_path.clone()))
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
