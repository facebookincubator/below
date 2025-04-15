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
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::fd::BorrowedFd;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use nix::sys::statfs::RDTGROUP_SUPER_MAGIC;
use nix::sys::statfs::fstatfs;
use openat::Dir;
use openat::SimpleType;
use thiserror::Error;

mod types;
pub use types::*;

#[cfg(test)]
mod test;

pub const DEFAULT_RESCTRL_ROOT: &str = "/sys/fs/resctrl";

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file format: {0:?}")]
    InvalidFileFormat(PathBuf),
    #[error("{1:?}: {0:?}")]
    IoError(PathBuf, #[source] std::io::Error),
    #[error("Unexpected line ({1}) in file: {0:?}")]
    UnexpectedLine(PathBuf, String),
    #[error("Not resctrl filesystem: {0:?}")]
    NotResctrl(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

/// resctrlfs can give us a NotFound for various files and directories. In a lot of cases, these
/// are expected (e.g. when control or monitoring are disabled). Thus we translate these errors to
/// `None`.
fn wrap<S: Sized>(v: std::result::Result<S, Error>) -> std::result::Result<Option<S>, Error> {
    if let Err(Error::IoError(_, ref e)) = v {
        if e.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        if e.kind() == std::io::ErrorKind::Other {
            if let Some(errno) = e.raw_os_error() {
                if errno == /* ENODEV */ 19 {
                    // If the resctrl group is removed after a control file is opened,
                    // ENODEV may returned. Ignore it.
                    return Ok(None);
                }
            }
        }
    }
    v.map(Some)
}

/// Parse a node range and return the set of nodes. This is either a range "x-y"
/// or a single value "x".
fn parse_node_range(s: &str) -> std::result::Result<BTreeSet<u32>, String> {
    fn parse_node(s: &str) -> std::result::Result<u32, String> {
        s.parse()
            .map_err(|_| format!("id must be non-negative int: {}", s))
    }
    match s.split_once('-') {
        Some((first, last)) => {
            let first = parse_node(first)?;
            let last = parse_node(last)?;
            if first > last {
                return Err(format!("Invalid range: {}", s));
            }
            Ok((first..(last + 1)).collect())
        }
        None => Ok(BTreeSet::from([parse_node(s)?])),
    }
}

/// Parse a node range list (this is the format for resctrl cpus_list file and
/// also the format for cpusets in cgroupfs). e.g. "0-2,4" would return the set
/// {0, 1, 2, 4}.
fn nodes_from_str(s: &str) -> std::result::Result<BTreeSet<u32>, String> {
    let mut nodes = BTreeSet::new();
    if s.is_empty() {
        return Ok(nodes);
    }
    for range_str in s.split(',') {
        let mut to_append = parse_node_range(range_str)?;
        nodes.append(&mut to_append);
    }
    Ok(nodes)
}

/// Format a set of nodes as a node range list. This is the inverse of
/// `nodes_to_str`.
fn fmt_nodes(f: &mut std::fmt::Formatter<'_>, nodes: &BTreeSet<u32>) -> std::fmt::Result {
    fn print_range(
        f: &mut std::fmt::Formatter<'_>,
        range_start: u32,
        range_end: u32,
    ) -> std::fmt::Result {
        if range_start == range_end {
            write!(f, "{}", range_start)
        } else {
            write!(f, "{}-{}", range_start, range_end)
        }
    }

    let mut range_start = *nodes.iter().next().unwrap_or(&u32::MAX);
    let mut range_end = range_start;
    for cpu in nodes {
        if range_end + 1 == *cpu || range_end == *cpu {
            range_end = *cpu;
        } else {
            print_range(f, range_start, range_end)?;
            write!(f, ",")?;
            range_start = *cpu;
            range_end = *cpu;
        }
    }
    if !nodes.is_empty() {
        print_range(f, range_start, range_end)?;
    }
    Ok(())
}

impl FromStr for Cpuset {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        Ok(Cpuset {
            cpus: nodes_from_str(s)?,
        })
    }
}

impl std::fmt::Display for Cpuset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt_nodes(f, &self.cpus)
    }
}

impl FromStr for GroupMode {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "shareable" => Ok(GroupMode::Shareable),
            "exclusive" => Ok(GroupMode::Exclusive),
            _ => Err(format!("Unknown group mode: {}", s)),
        }
    }
}

impl std::fmt::Display for GroupMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupMode::Shareable => write!(f, "shareable"),
            GroupMode::Exclusive => write!(f, "exclusive"),
        }
    }
}

impl FromStr for RmidBytes {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "Unavailable" => Ok(RmidBytes::Unavailable),
            _ => Ok(RmidBytes::Bytes(s.parse().map_err(|_| "Not a number")?)),
        }
    }
}

/// A reader for a resctrl MON or CTRL_MON or root group.
struct ResctrlGroupReader {
    path: PathBuf,
    dir: Dir,
}

/// Reader to read entire resctrl hierarchy.
pub struct ResctrlReader {
    path: PathBuf,
}

impl ResctrlGroupReader {
    /// Create a new reader for a resctrl MON or CTRL_MON or root group.
    fn new(path: PathBuf) -> Result<ResctrlGroupReader> {
        let dir = Dir::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        Ok(ResctrlGroupReader { path, dir })
    }

    /// Return the name of the group.
    fn name(&self) -> String {
        self.path
            .file_name()
            .expect("Unexpected .. in path")
            .to_string_lossy()
            .to_string()
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

    /// Helper to create InvalidFileFormat error
    fn invalid_file_format<P: AsRef<Path>>(&self, file_name: P) -> Error {
        let mut p = self.path.clone();
        p.push(file_name);
        Error::InvalidFileFormat(p)
    }

    /// Helper to create IoError error
    fn io_error<P: AsRef<Path>>(&self, file_name: P, e: std::io::Error) -> Error {
        let mut p = self.path.clone();
        p.push(file_name);
        Error::IoError(p, e)
    }

    /// Helper to create UnexpectedLine error
    fn unexpected_line<P: AsRef<Path>>(&self, file_name: P, line: String) -> Error {
        let mut p = self.path.clone();
        p.push(file_name);
        Error::UnexpectedLine(p, line)
    }

    /// Return L3 cache ID for given mon_stat_dir name. e.g. "mon_L3_01" returns 1.
    fn maybe_get_l3_mon_stat_dir_id(&self) -> Result<u64> {
        let name = self.name();
        if !name.starts_with("mon_L3_") {
            return Err(self.invalid_file_format(""));
        }
        name[7..]
            .parse::<u64>()
            .map_err(|_| self.invalid_file_format(""))
    }

    /// Read the inode number of the group.
    fn read_inode_number(&self) -> Result<u64> {
        let meta = self.dir.metadata(".").map_err(|e| self.io_error("", e))?;
        Ok(meta.stat().st_ino)
    }

    /// Read cpuset from cpus_list file
    fn read_cpuset(&self) -> Result<Cpuset> {
        self.read_empty_or_singleline_file("cpus_list")
    }

    /// Read mode file. Only applicable for CTRL_MON and root group.
    fn read_mode(&self) -> Result<GroupMode> {
        self.read_singleline_file("mode")
    }

    /// Read all L3_mon data for this group.
    fn read_l3_mon_stat(&self) -> Result<L3MonStat> {
        Ok(L3MonStat {
            llc_occupancy_bytes: wrap(self.read_singleline_file("llc_occupancy"))?,
            mbm_total_bytes: wrap(self.read_singleline_file("mbm_total_bytes"))?,
            mbm_local_bytes: wrap(self.read_singleline_file("mbm_local_bytes"))?,
        })
    }

    /// Read mon_stat directory if it exists otherwise return None.
    fn read_mon_stat(&self) -> Result<MonStat> {
        Ok(MonStat {
            l3_mon_stat: Some(
                self.child_iter("mon_data".into())?
                    .flat_map(|child| {
                        child
                            .read_l3_mon_stat()
                            .map(|v| child.maybe_get_l3_mon_stat_dir_id().map(|id| (id, v)))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?,
            ),
        })
    }

    /// Read current group as a MON group
    fn read_mon_group(&self) -> Result<MonGroupStat> {
        Ok(MonGroupStat {
            inode_number: Some(self.read_inode_number()?),
            cpuset: Some(self.read_cpuset()?),
            mon_stat: wrap(self.read_mon_stat())?,
        })
    }

    /// Read current group as a CTRL_MON group
    fn read_ctrl_mon_group(&self) -> Result<CtrlMonGroupStat> {
        Ok(CtrlMonGroupStat {
            inode_number: Some(self.read_inode_number()?),
            cpuset: Some(self.read_cpuset()?),
            mode: wrap(self.read_mode())?,
            mon_stat: wrap(self.read_mon_stat())?,
            mon_groups: wrap(self.read_child_mon_groups())?,
        })
    }

    /// Get iterator of child group readers
    fn child_iter(
        &self,
        child_dir_name: PathBuf,
    ) -> Result<impl Iterator<Item = ResctrlGroupReader> + '_> {
        Ok(self
            .dir
            .list_dir(&child_dir_name)
            .map_err(|e| self.io_error(&child_dir_name, e))?
            .filter_map(move |entry| match entry {
                Ok(entry) if entry.simple_type() == Some(SimpleType::Dir) => {
                    let relative_path = child_dir_name.join(entry.file_name());
                    let sub_dir = match self.dir.sub_dir(relative_path.as_path()) {
                        Ok(d) => d,
                        Err(_) => return None,
                    };
                    let mut path = self.path.clone();
                    path.push(entry.file_name());
                    Some(ResctrlGroupReader { path, dir: sub_dir })
                }
                _ => None,
            }))
    }

    /// Read child MON groups
    fn read_child_mon_groups(&self) -> Result<BTreeMap<String, MonGroupStat>> {
        self.child_iter("mon_groups".into())?
            .map(|child| child.read_mon_group().map(|v| (child.name(), v)))
            .collect::<Result<BTreeMap<_, _>>>()
    }

    /// Read child CTRL MON groups
    fn read_child_ctrl_mon_groups(&self) -> Result<BTreeMap<String, CtrlMonGroupStat>> {
        self.child_iter(".".into())?
            .filter(|r| !["info", "mon_groups", "mon_data"].contains(&r.name().as_str()))
            .map(|child| child.read_ctrl_mon_group().map(|v| (child.name(), v)))
            .collect::<Result<BTreeMap<_, _>>>()
    }
}

impl ResctrlReader {
    pub fn new(path: PathBuf, validate: bool) -> Result<ResctrlReader> {
        let dir = Dir::open(&path).map_err(|e| Error::IoError(path.clone(), e))?;
        // Check that it's a resctrl fs
        if validate {
            // SAFETY: Fix when https://github.com/nix-rust/nix/issues/2546 is
            let dir = unsafe { BorrowedFd::borrow_raw(dir.as_raw_fd()) };
            let statfs = match fstatfs(dir) {
                Ok(s) => s,
                Err(e) => {
                    return Err(Error::IoError(
                        path,
                        std::io::Error::new(ErrorKind::Other, format!("Failed to fstatfs: {}", e)),
                    ));
                }
            };

            if statfs.filesystem_type() != RDTGROUP_SUPER_MAGIC {
                return Err(Error::NotResctrl(path));
            }
        }
        Ok(ResctrlReader { path })
    }

    pub fn root() -> Result<ResctrlReader> {
        Self::new(DEFAULT_RESCTRL_ROOT.into(), true)
    }

    pub fn read_all(&self) -> Result<ResctrlSample> {
        let reader = ResctrlGroupReader::new(self.path.clone())?;
        Ok(ResctrlSample {
            cpuset: Some(reader.read_cpuset()?),
            mode: wrap(reader.read_mode())?,
            mon_stat: wrap(reader.read_mon_stat())?,
            ctrl_mon_groups: Some(reader.read_child_ctrl_mon_groups()?),
            mon_groups: wrap(reader.read_child_mon_groups())?,
        })
    }
}
