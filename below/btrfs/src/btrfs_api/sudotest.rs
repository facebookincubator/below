pub use btrfs::btrfs_api::*;

use openat::Dir;
use std::fs;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use nix::sys::statfs::{fstatfs, FsType};

fn is_btrfs(base_path: &Path) -> bool {
    let dir = Dir::open(base_path)
        .map_err(|e| Error::IoError(base_path.to_path_buf(), e))
        .expect("Could not open directory");

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
        let f = File::open(&base_path).expect("Failed to open file");
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
        let f = File::open(&base_path).expect("Failed to open file");
        let fd = f.as_raw_fd();
        ino_lookup(fd, BTRFS_FS_TREE_OBJECTID as u64, inode, |res| {
            res.expect("ino lookup failed");
        });
    } else {
        println!("Not on Btrfs");
    }
}
