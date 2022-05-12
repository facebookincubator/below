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

use std::{ffi::CStr, ops::RangeInclusive};

pub use btrfs_sys::*;

#[cfg(test)]
mod test;

mod utils;
pub use crate::btrfs_api::utils::*;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("System Error: {0}")]
    SysError(nix::errno::Errno),
    #[error("{1:?}: {0:?}")]
    IoError(PathBuf, #[source] std::io::Error),
    #[error("Not btrfs filesystem: {0:?}")]
    NotBtrfs(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

// Magic numbers for ioctl system calls can be found here:
// https://elixir.bootlin.com/linux/latest/source/include/uapi/linux/btrfs.h
mod ioctl {
    use super::*;
    nix::ioctl_readwrite!(search_v2, BTRFS_IOCTL_MAGIC, 17, btrfs_ioctl_search_args_v2);
    nix::ioctl_readwrite!(
        ino_lookup,
        BTRFS_IOCTL_MAGIC,
        18,
        btrfs_ioctl_ino_lookup_args
    );
    nix::ioctl_readwrite!(ino_paths, BTRFS_IOCTL_MAGIC, 35, btrfs_ioctl_ino_path_args);
    nix::ioctl_readwrite!(
        logical_ino_v2,
        BTRFS_IOCTL_MAGIC,
        59,
        btrfs_ioctl_logical_ino_args
    );
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
// This struct is derived from here:
// https://elixir.bootlin.com/linux/latest/source/fs/btrfs/ioctl.c#L4195
pub struct LogicalInoItem {
    pub inum: u64,
    pub offset: u64,
    pub root: u64,
}

pub fn logical_ino(
    fd: i32,
    logical: u64,
    ignoring_offset: bool,
    mut cb: impl FnMut(Result<&[LogicalInoItem]>),
) {
    let mut data = WithMemAfter::<btrfs_data_container, 4096>::new();

    let mut args = btrfs_ioctl_logical_ino_args {
        logical,
        size: data.total_size() as u64,
        reserved: Default::default(),
        flags: if ignoring_offset {
            BTRFS_LOGICAL_INO_ARGS_IGNORE_OFFSET as u64
        } else {
            0
        },
        inodes: data.as_mut_ptr() as u64,
    };
    unsafe {
        match ioctl::logical_ino_v2(fd, &mut args) {
            Ok(_) => {
                let inodes = std::slice::from_raw_parts(
                    data.extra_ptr() as *const LogicalInoItem,
                    // Magic number 3 comes from size_of(LogicalInoItem) / size_of(u64)
                    // (the elements of btrfs_data_container val are u64).
                    (data.elem_cnt / 3) as usize,
                );
                cb(Ok(inodes));
            }
            Err(err) => {
                cb(Err(Error::SysError(err)));
            }
        }
    }
}

pub fn ino_lookup(fd: i32, root: u64, inum: u64, mut cb: impl FnMut(Result<&CStr>)) {
    let mut args = btrfs_ioctl_ino_lookup_args {
        treeid: root,
        objectid: inum,
        name: [0; BTRFS_INO_LOOKUP_PATH_MAX as usize],
    };

    unsafe {
        match ioctl::ino_lookup(fd, &mut args) {
            Ok(_) => {
                cb(Ok(CStr::from_ptr(args.name.as_ptr())));
            }
            Err(err) => {
                cb(Err(Error::SysError(err)));
            }
        }
    }
}

pub struct SearchKey {
    pub objectid: u64,
    pub typ: u8,
    pub offset: u64,
}

impl SearchKey {
    pub const MIN: Self = SearchKey::new(u64::MIN, u8::MIN, u64::MIN);
    pub const MAX: Self = SearchKey::new(u64::MAX, u8::MAX, u64::MAX);

    pub const ALL: RangeInclusive<Self> = Self::MIN..=Self::MAX;

    pub const fn range_fixed_id_type(objectid: u64, typ: u8) -> RangeInclusive<Self> {
        Self::new(objectid, typ, u64::MIN)..=Self::new(objectid, typ, u64::MAX)
    }

    pub const fn new(objectid: u64, typ: u8, offset: u64) -> Self {
        Self {
            objectid,
            typ,
            offset,
        }
    }

    pub fn next(&self) -> Self {
        let (offset, carry1) = self.offset.overflowing_add(1);
        let (typ, carry2) = self.typ.overflowing_add(carry1 as u8);
        let (objectid, _) = self.objectid.overflowing_add(carry2 as u64);
        SearchKey {
            objectid,
            typ,
            offset,
        }
    }

    fn from(h: &btrfs_ioctl_search_header) -> Self {
        SearchKey {
            objectid: h.objectid,
            typ: h.type_ as u8,
            offset: h.offset,
        }
    }
}

pub fn tree_search_cb(
    fd: i32,
    tree_id: u64,
    range: RangeInclusive<SearchKey>,
    mut cb: impl FnMut(&btrfs_ioctl_search_header, &[u8]),
) -> Result<()> {
    const BUF_SIZE: usize = 16 * 1024;
    let mut args = WithMemAfter::<btrfs_ioctl_search_args_v2, BUF_SIZE>::new();
    args.key = btrfs_ioctl_search_key {
        tree_id,
        min_objectid: range.start().objectid,
        max_objectid: range.end().objectid,
        min_offset: range.start().offset,
        max_offset: range.end().offset,
        min_transid: u64::MIN,
        max_transid: u64::MAX,
        min_type: range.start().typ as u32,
        max_type: range.end().typ as u32,
        nr_items: u32::MAX,

        unused: 0,
        unused1: 0,
        unused2: 0,
        unused3: 0,
        unused4: 0,
    };
    args.buf_size = args.extra_size() as u64;

    loop {
        args.key.nr_items = u32::MAX;
        unsafe {
            ioctl::search_v2(fd, args.as_mut_ptr()).map_err(Error::SysError)?;
        }
        if args.key.nr_items == 0 {
            break;
        }

        let mut ptr = args.buf.as_ptr() as *const u8;
        let mut last_search_header: *const btrfs_ioctl_search_header = std::ptr::null();
        for _ in 0..args.key.nr_items {
            let search_header =
                unsafe { get_and_move_typed::<btrfs_ioctl_search_header>(&mut ptr) };

            let data = unsafe {
                std::slice::from_raw_parts(
                    get_and_move(&mut ptr, (*search_header).len as usize),
                    (*search_header).len as usize,
                )
            };
            last_search_header = search_header;
            unsafe {
                cb(&*search_header, data);
            }
        }

        let min_key = unsafe { SearchKey::from(&*last_search_header).next() };

        args.key.min_objectid = min_key.objectid;
        args.key.min_type = min_key.typ as u32;
        args.key.min_offset = min_key.offset;
    }

    Ok(())
}
