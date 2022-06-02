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

use btrfs_sys::*;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub mod btrfs_api;

mod types;
pub use types::*;

#[cfg(test)]
mod test;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file format: {0:?}")]
    InvalidFileFormat(PathBuf),
    #[error("{1:?}: {0:?}")]
    IoError(PathBuf, #[source] std::io::Error),
    #[error("Failed call to btrfs")]
    SysError(btrfs_api::Error),
}

impl From<btrfs_api::Error> for Error {
    fn from(item: btrfs_api::Error) -> Self {
        Error::SysError(item)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub const DEFAULT_ROOT: &str = "/";
pub const DEFAULT_SAMPLES: u64 = 100;
pub const DEFAULT_MIN_PCT: f64 = 0.0;

// The SampleTree structure stores a hierarchical structure
// of path names that we have some size estimations for. This
// is supposed to follow the structure of files in the subvolume
struct SampleTree {
    // total number of samples under this tree.
    total: usize,
    children: HashMap<String, SampleTree>,
}

impl Default for SampleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleTree {
    fn new() -> Self {
        Self {
            total: 0,
            children: HashMap::new(),
        }
    }

    // path implements an iterator trait. This method is recursive and completes because path.next()
    // consumes one path instance from the iterator at a time.
    fn add<'a>(&mut self, mut path: impl Iterator<Item = &'a str>) {
        if let Some(p) = path.next() {
            self.total += 1;
            self.children
                .entry(p.to_string())
                .or_insert(SampleTree::new())
                .add(path);
        }
    }

    // This method parses the sample tree and outputs the paths corresponding to subvolumes that occupy
    // more than min_disk_fraction of the disk.
    fn convert(
        &self,
        total_samples: usize,
        total_length: u64,
        min_disk_fraction: Option<f64>,
    ) -> Result<BtrfsMap> {
        let mut btrfs_map: BtrfsMap = Default::default();
        self.convert_internal(
            total_samples,
            total_length,
            min_disk_fraction,
            "".to_string(),
            &mut btrfs_map,
        )?;

        Ok(btrfs_map)
    }

    fn convert_internal(
        &self,
        total_samples: usize,
        total_length: u64,
        min_disk_fraction: Option<f64>,
        base_path: String,
        btrfs_map: &mut BtrfsMap,
    ) -> Result<()> {
        for (p, child_tree) in &self.children {
            let dfraction = (child_tree.total as f64) / (total_samples as f64);
            let dbytes = (total_length as f64 * dfraction) as u64;

            match min_disk_fraction {
                Some(min_disk_fraction) if dfraction < min_disk_fraction => continue,
                _ => {}
            }

            let path = format!("{}/{}", base_path, p);

            let btrfs_stat = BtrfsStat {
                name: Some(path.clone()),
                disk_fraction: Some(dfraction * 100.0),
                disk_bytes: Some(dbytes),
            };

            btrfs_map.insert(path.clone(), btrfs_stat);

            child_tree.convert_internal(
                total_samples,
                total_length,
                min_disk_fraction,
                path,
                btrfs_map,
            )?;
        }

        Ok(())
    }
}

// This structure contains for each btrfs instance a hashmap of the subvolume ids and
// their respective paths.
struct Roots {
    fd: i32,
    // hashmap key is subvolume id and value is vector with the path of that subvolume
    m: HashMap<u64, Rc<Vec<String>>>,
}

impl Roots {
    fn new(fd: i32) -> Self {
        Self {
            fd,
            m: HashMap::from([(BTRFS_FS_TREE_OBJECTID as u64, Rc::new(Vec::new()))]),
        }
    }

    fn get_root(&mut self, root_id: u64) -> Result<Rc<Vec<String>>> {
        match self.m.get(&root_id) {
            Some(path) => Ok(Rc::clone(path)),
            None => {
                let root_backref = btrfs_api::find_root_backref(self.fd, root_id)?;
                match root_backref {
                    Some((name, parent_id)) => {
                        let rec_root = self.get_root(parent_id)?;
                        let mut path = Vec::clone(&rec_root);
                        path.push(name);
                        let path_rc = Rc::new(path);
                        self.m.insert(root_id, path_rc.clone());
                        Ok(path_rc)
                    }
                    None => Err(Error::SysError(btrfs_api::Error::SysError(
                        nix::errno::Errno::ENOENT,
                    ))),
                }
            }
        }
    }
}

pub struct BtrfsReader {
    samples: u64,
    min_pct: f64,
    path: PathBuf,
}

impl BtrfsReader {
    pub fn new() -> BtrfsReader {
        BtrfsReader::new_with_path(DEFAULT_ROOT.to_string())
    }

    pub fn new_with_path(p: String) -> BtrfsReader {
        let path = p.into();
        BtrfsReader {
            samples: DEFAULT_SAMPLES,
            min_pct: DEFAULT_MIN_PCT,
            path,
        }
    }

    pub fn sample(&self) -> Result<BtrfsMap> {
        // Stub implementation of sample. This does nothing for now.
        Ok(Default::default())
    }

    fn io_error<P: AsRef<Path>>(&self, file_name: P, e: std::io::Error) -> Error {
        let mut p = self.path.clone();
        p.push(file_name);
        Error::IoError(p, e)
    }
}
