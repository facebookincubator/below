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

use rand_distr::Distribution;
use rand_distr::Uniform;
use slog::error;
use slog::warn;
use slog::{self};
use std::collections::HashMap;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

pub mod btrfs_api;

#[cfg(not(fbcode_build))]
pub use btrfs_api::open_source::btrfs_sys::*;
#[cfg(fbcode_build)]
pub use btrfs_sys::*;

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
    logger: slog::Logger,
}

impl BtrfsReader {
    pub fn new(samples: u64, min_pct: f64, logger: slog::Logger) -> BtrfsReader {
        BtrfsReader::new_with_path(DEFAULT_ROOT.to_string(), samples, min_pct, logger)
    }

    pub fn new_with_path(
        p: String,
        samples: u64,
        min_pct: f64,
        logger: slog::Logger,
    ) -> BtrfsReader {
        BtrfsReader {
            samples,
            min_pct,
            path: p.into(),
            logger,
        }
    }

    pub fn sample(&self) -> Result<BtrfsMap> {
        let f = File::open(&self.path).map_err(|e| self.io_error(&self.path, e))?;

        let fd = f.as_raw_fd();

        #[derive(Debug)]
        struct ChunkInfo {
            pos: u64,
            chunk_offset: u64,
            chunk_length: u64,
            chunk_type: u64,
        }

        let samples = self.samples;
        let mut chunks = Vec::<ChunkInfo>::new();
        let mut total_chunk_length = 0;
        let mut chunks_size = 0;
        btrfs_api::tree_search_cb(
            fd,
            btrfs_api::BTRFS_CHUNK_TREE_OBJECTID as u64,
            btrfs_api::SearchKey::ALL,
            |sh, data| {
                match sh.type_ {
                    btrfs_api::BTRFS_CHUNK_ITEM_KEY => {
                        let chunk = unsafe { &*(data.as_ptr() as *const btrfs_api::btrfs_chunk) };
                        chunks.push(ChunkInfo {
                            pos: total_chunk_length,
                            chunk_offset: sh.offset,
                            chunk_length: chunk.length,
                            chunk_type: chunk.type_,
                        });
                        chunks_size += 1;
                        total_chunk_length += chunk.length;
                    }
                    _ => {}
                };
            },
        )
        .map_err(Error::SysError)?;

        let mut roots = Roots::new(fd);
        let uniform = Uniform::new(0, total_chunk_length);
        let mut rng = rand::thread_rng();

        let mut sample_tree = SampleTree::new();
        let mut total_samples = 0;

        let mut random_positions = Vec::new();
        for _ in 0..samples {
            random_positions.push(uniform.sample(&mut rng));
        }
        random_positions.sort_unstable();

        let mut chunk_idx = 0;
        for random_position in &random_positions {
            while random_position > &(chunks[chunk_idx].pos + chunks[chunk_idx].chunk_length) {
                chunk_idx += 1;
            }

            let random_chunk = &chunks[chunk_idx];
            total_samples += 1;
            match (random_chunk.chunk_type as u32) & btrfs_api::BTRFS_BLOCK_GROUP_TYPE_MASK {
                btrfs_api::BTRFS_BLOCK_GROUP_DATA => {
                    let random_offset =
                        random_chunk.chunk_offset + (random_position - random_chunk.pos);
                    let mut err = Ok(());
                    btrfs_api::logical_ino(fd, random_offset, false, |res| match res {
                        Ok(inodes) => {
                            for inode in inodes {
                                btrfs_api::ino_lookup(fd, inode.root, inode.inum, |res| match res {
                                    Ok(path) => match roots.get_root(inode.root) {
                                        Ok(root_path) => {
                                            let root_path_it = root_path.iter().map(|s| s.as_str());
                                            let inode_path = path
                                                .to_str()
                                                .expect("Could not convert path to string")
                                                .split('/')
                                                .filter(|s| !s.is_empty());
                                            sample_tree.add(root_path_it.chain(inode_path));
                                        }
                                        Err(e) => {
                                            err = Err(e);
                                        }
                                    },
                                    Err(btrfs_api::Error::SysError(nix::errno::Errno::ENOENT)) => {}
                                    Err(e) => {
                                        warn!(
                                            self.logger,
                                            "INO_LOOKUP Returned error {} for inode.root {} and inode.inum {}",
                                            e,
                                            inode.root,
                                            inode.inum
                                        );
                                    }
                                })
                            }
                        }
                        Err(btrfs_api::Error::SysError(nix::errno::Errno::ENOENT)) => {}
                        Err(e) => {
                            warn!(
                                self.logger,
                                "LOGICAL_INO returned error {} for random offset {} ",
                                e,
                                random_offset
                            );
                        }
                    });
                    err?;
                }
                btrfs_api::BTRFS_BLOCK_GROUP_METADATA => {}
                btrfs_api::BTRFS_BLOCK_GROUP_SYSTEM => {}
                _ => {}
            };
        }

        sample_tree.convert(
            total_samples,
            total_chunk_length,
            Some(self.min_pct as f64 / 100.0),
        )
    }

    fn io_error<P: AsRef<Path>>(&self, file_name: P, e: std::io::Error) -> Error {
        let mut p = self.path.clone();
        p.push(file_name);
        Error::IoError(p, e)
    }
}
