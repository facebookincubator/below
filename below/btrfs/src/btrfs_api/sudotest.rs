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

use std::fs;
use std::fs::File;
use std::os::fd::BorrowedFd;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

#[cfg(fbcode_build)]
pub use btrfs::btrfs_api::*;
#[cfg(fbcode_build)]
use btrfs::BtrfsReader;
use common::logutil::get_logger;
use nix::sys::statfs::fstatfs;
use nix::sys::statfs::FsType;
use openat::Dir;

#[cfg(not(fbcode_build))]
pub use crate::btrfs_api::*;
#[cfg(not(fbcode_build))]
use crate::BtrfsReader;

// Currently, sudotests test basic functionality. Will testing infrastructure in later commits

fn is_btrfs(base_path: &Path) -> bool {
    let dir = Dir::open(base_path)
        .map_err(|e| Error::IoError(base_path.to_path_buf(), e))
        .expect("Could not open directory");

    // SAFETY: Fix when https://github.com/nix-rust/nix/issues/2546 is
    let dir = unsafe { BorrowedFd::borrow_raw(dir.as_raw_fd()) };
    let statfs = match fstatfs(&dir) {
        Ok(s) => s,
        Err(_) => {
            return false;
        }
    };

    statfs.filesystem_type() == FsType(libc::BTRFS_SUPER_MAGIC)
}

#[test]
fn logical_ino_test() {
    let base_path = Path::new(&"/");
    if is_btrfs(base_path) {
        let f = File::open(base_path).expect("Failed to open file");
        let fd = f.as_raw_fd();
        logical_ino(fd, 0, false, |res| match res {
            Ok(_) => {}
            // it's OK for now to have the offset not pointing to any extent
            Err(Error::SysError(nix::errno::Errno::ENOENT)) => {}
            Err(err) => {
                panic!("{:?}", err);
            }
        });
    } else {
        println!("Not on Btrfs");
    }
}

#[test]
fn ino_lookup_test() {
    let base_path = Path::new(&"/");
    let meta = fs::metadata(base_path).expect("Could not find inode");
    let inode = meta.ino();
    if is_btrfs(base_path) {
        let f = File::open(base_path).expect("Failed to open file");
        let fd = f.as_raw_fd();
        ino_lookup(fd, BTRFS_FS_TREE_OBJECTID as u64, inode, |res| {
            res.expect("ino lookup failed");
        });
    } else {
        println!("Not on Btrfs");
    }
}

#[test]
#[ignore]
fn tree_search_cb_test() {
    let base_path = Path::new(&"/");
    if is_btrfs(base_path) {
        let f = File::open(base_path).expect("File did not open");
        let fd = f.as_raw_fd();
        let mut chunk_length = 0;
        let _ = tree_search_cb(
            fd,
            BTRFS_CHUNK_TREE_OBJECTID as u64,
            SearchKey::ALL,
            |sh, data| match sh.type_ {
                BTRFS_CHUNK_ITEM_KEY => {
                    let chunk = unsafe { &*(data.as_ptr() as *const btrfs_chunk) };
                    chunk_length += chunk.length;
                }
                _ => {}
            },
        );
        assert!(chunk_length > 0);
    } else {
        println!("Not on Btrfs");
    }
}

#[test]
fn find_root_backref_test() {
    let base_path = Path::new(&"/");
    if is_btrfs(base_path) {
        let f = File::open(base_path).expect("File did not open");
        let fd = f.as_raw_fd();
        find_root_backref(fd, BTRFS_FS_TREE_OBJECTID.into()).expect("Unexpected error");
    } else {
        println!("Not on Btrfs");
    }
}

#[test]
#[ignore]
fn test_sample() {
    if is_btrfs(Path::new(&"/")) {
        let logger = get_logger();
        let btrfs_reader = BtrfsReader::new(100, 0.0, logger);
        let btrfs_map = btrfs_reader.sample().expect("Sample returned error");
        assert!(!btrfs_map.is_empty());
    } else {
        println!("Not on Btrfs");
    }
}
