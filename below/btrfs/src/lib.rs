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

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

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

pub type Result<T> = std::result::Result<T, Error>;

pub const DEFAULT_ROOT: &str = "/";
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
