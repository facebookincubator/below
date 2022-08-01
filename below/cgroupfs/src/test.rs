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

use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::CgroupReader;
use crate::Error;

struct TestCgroup {
    tempdir: TempDir,
}

impl TestCgroup {
    fn new() -> TestCgroup {
        TestCgroup {
            tempdir: TempDir::new().expect("Failed to create tempdir"),
        }
    }

    fn path(&self) -> &Path {
        self.tempdir.path()
    }

    fn get_reader(&self) -> CgroupReader {
        CgroupReader::new_with_relative_path_inner(
            self.path().to_path_buf(),
            PathBuf::from(OsStr::new("")),
            false,
        )
        .expect("Failed to construct reader")
    }

    fn get_reader_validate(&self) -> Result<CgroupReader, Error> {
        CgroupReader::new(self.path().to_path_buf())
    }

    fn create_file_with_content<P: AsRef<Path>>(&self, p: P, content: &[u8]) {
        let path = self.path().join(p);
        let mut file = File::create(&path).expect(&format!("Failed to create {}", path.display()));
        file.write_all(content)
            .expect(&format!("Failed to write to {}", path.display()));
    }

    fn create_child<P: AsRef<Path>>(&self, p: P) {
        let path = self.path().join(p);
        std::fs::create_dir(&path)
            .expect(&format!("Failed to create child cgroup {}", path.display()));
    }
}

#[test]
fn test_read_inode_number() {
    let cgroup = TestCgroup::new();
    let cgroup_reader = cgroup.get_reader();
    let inode = cgroup_reader
        .read_inode_number()
        .expect("Failed to read inode number");
    assert_eq!(
        inode,
        std::fs::metadata(cgroup.path())
            .expect("Failed to read inode number with fs::metadata")
            .st_ino()
    );
}

#[test]
fn test_memory_current_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.current", b"1234\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_current()
        .expect("Failed to read memory.current");
    assert_eq!(val, 1234);
}

#[test]
fn test_memory_current_parse_failure() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.current", b"1234.0\n");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_current()
        .expect_err("Did not fail to read memory.current");
    match err {
        Error::UnexpectedLine(_, _) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_memory_current_invalid_format() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.current", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_current()
        .expect_err("Did not fail to read memory.current");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_memory_swap_current_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.swap.current", b"1234\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_swap_current()
        .expect("Failed to read memory.swap.current");
    assert_eq!(val, 1234);
}

#[test]
fn test_memory_zswap_current_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.zswap.current", b"1234\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_zswap_current()
        .expect("Failed to read memory.zswap.current");
    assert_eq!(val, 1234);
}

#[test]
fn test_memory_high_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.high", b"1234\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_high()
        .expect("Failed to read memory.high")
        .expect("memory.high should not be none in this case.");
    assert_eq!(val, 1234);
}

#[test]
fn test_memory_high_max_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.high", b"max\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_high()
        .expect("Failed to read memory.high")
        .expect("memory.high should not be none in this case.");
    assert_eq!(val, -1);
}

#[test]
fn test_memory_high_failure() {
    let cgroup = TestCgroup::new();
    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_high()
        .expect("Failed to read memory.high");
    assert_eq!(val, None);

    cgroup.create_file_with_content("memory.high", b"-1\n");
    let val = cgroup_reader.read_memory_high();
    assert!(val.is_err());
}

#[test]
fn test_memory_stat_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.stat", b"slab 1234\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_stat()
        .expect("Failed to read memory.stat");
    assert_eq!(val.slab.expect("Failed to populate slab field"), 1234);
}

#[test]
fn test_memory_stat_overflow() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.stat", b"slab 14914318128160131214\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_stat()
        .expect("Failed to read memory.stat");
    assert_eq!(
        val.slab.expect("Failed to populate slab field") as u64,
        14914318128160131214
    );
}

#[test]
fn test_memory_stat_parse_failure() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.stat", b"slab 1234\nlol\n");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_stat()
        .expect_err("Did not fail to read memory.stat");
    match err {
        Error::UnexpectedLine(_, _) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_memory_stat_invalid_format() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.stat", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_stat()
        .expect_err("Did not fail to read memory.stat");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}

#[test]
fn test_cpu_stat_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("cpu.stat", b"usage_usec 1234\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_cpu_stat()
        .expect("Failed to read cpu.stat");
    assert_eq!(
        val.usage_usec.expect("Failed to populate usage_usec field"),
        1234
    );
}

#[test]
fn test_cpu_stat_parse_failure() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("cpu.stat", b"usage_usec 1234\nlol\n");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_cpu_stat()
        .expect_err("Did not fail to read cpu.stat");
    match err {
        Error::UnexpectedLine(_, _) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_cpu_stat_invalid_format() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("cpu.stat", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_cpu_stat()
        .expect_err("Did not fail to read cpu.stat");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}

#[test]
fn test_io_stat_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("io.stat", b"253:0 rbytes=531 wbytes=162379 rios=61 wios=81 dbytes=0 dios=0\n13:0 rbytes=135 wbytes=162379 rios=61 wios=81 dbytes=0 dios=0 cost.usage=25 cost.wait=38 cost.indebt=64 cost.indelay=0\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_io_stat()
        .expect("Failed to read io.stat");
    assert_eq!(
        val["253:0"]
            .rbytes
            .expect("Failed to populate rbytes field"),
        531
    );
    assert!(val["253:0"].cost_usage.is_none());
    assert_eq!(
        val["13:0"].rbytes.expect("Failed to populate rbytes field"),
        135
    );
    assert_eq!(
        val["13:0"]
            .cost_usage
            .expect("Failed to populate cost_usage field"),
        25
    );
}

#[test]
fn test_io_stat_parse_failure() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("io.stat", b"usage_usec 1234\n");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_io_stat()
        .expect_err("Did not fail to read io.stat");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_io_stat_empty_file() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("io.stat", b"");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_io_stat()
        .expect("Failed to read io.stat");
    assert!(val.is_empty());
}

#[test]
fn test_cpu_pressure_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content(
        "cpu.pressure",
        b"some avg10=0.00 avg60=0.00 avg300=0.00 total=619176290",
    );

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_cpu_pressure()
        .expect("Failed to read cpu.pressure");
    assert_eq!(
        val.some.total.expect("Failed to populate total field"),
        619176290
    );
    assert_eq!(val.full, None);
}

#[test]
fn test_cpu_pressure_full() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content(
        "cpu.pressure",
        b"some avg10=0.00 avg60=0.00 avg300=0.00 total=619176290\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=34509874",
    );

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_cpu_pressure()
        .expect("Failed to read cpu.pressure");
    assert_eq!(
        val.some.total.expect("Failed to populate total field"),
        619176290
    );

    assert_eq!(
        val.full
            .expect("Failed to read cpu.pressure full")
            .total
            .expect("Failed to populate total field"),
        34509874
    );
}

#[test]
fn test_cpu_pressure_empty_file() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("cpu.pressure", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_cpu_pressure()
        .expect_err("Did not fail to read cpu.pressure");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}

#[test]
fn test_io_pressure_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("io.pressure", b"some avg10=0.00 avg60=0.00 avg300=0.00 total=619176290\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=61917\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_io_pressure()
        .expect("Failed to read io.pressure");
    assert_eq!(
        val.some.total.expect("Failed to populate total field"),
        619176290
    );
    assert_eq!(
        val.full.total.expect("Failed to populate total field"),
        61917
    );
}

#[test]
fn test_io_pressure_empty_file() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("io.pressure", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_io_pressure()
        .expect_err("Did not fail to read io.pressure");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}

#[test]
fn test_memory_pressure_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.pressure", b"some avg10=0.00 avg60=0.00 avg300=0.00 total=619176290\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=61917\n");

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_pressure()
        .expect("Failed to read memory.pressure");
    assert_eq!(
        val.some.total.expect("Failed to populate total field"),
        619176290
    );
    assert_eq!(
        val.full.total.expect("Failed to populate total field"),
        61917
    );
}

#[test]
fn test_memory_pressure_empty_file() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.pressure", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_pressure()
        .expect_err("Did not fail to read memory.pressure");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}

#[test]
fn test_child_cgroup_iter() {
    let root = TestCgroup::new();
    let children = vec![
        OsStr::new("child1"),
        OsStr::new("child2"),
        OsStr::new("child3"),
    ];

    for child in &children {
        root.create_child(child);
    }

    let mut reported_children: Vec<_> = root
        .get_reader()
        .child_cgroup_iter()
        .expect("Failed to enumerate child cgroups")
        .map(|c| {
            c.name()
                .file_name()
                .expect("Failed to get path file name")
                .to_os_string()
        })
        .collect();

    reported_children.sort();

    assert_eq!(reported_children, children,);
}

#[test]
fn test_child_cgroup_iter_with_file() {
    let root = TestCgroup::new();
    let children = vec![
        OsStr::new("child1"),
        OsStr::new("child2"),
        OsStr::new("child3"),
    ];

    for child in &children {
        root.create_child(child);
    }

    root.create_file_with_content("memory.current", b"1234\n");

    let mut reported_children: Vec<_> = root
        .get_reader()
        .child_cgroup_iter()
        .expect("Failed to enumerate child cgroups")
        .map(|c| {
            c.name()
                .file_name()
                .expect("Failed to get path file name")
                .to_os_string()
        })
        .collect();
    reported_children.sort();

    assert_eq!(reported_children, children,);
}

#[test]
fn test_child_cgroup_iter_empty() {
    let root = TestCgroup::new();

    let reported_children: Vec<_> = root
        .get_reader()
        .child_cgroup_iter()
        .expect("Failed to enumerate child cgroups")
        .collect();

    // For some reason, setting this to
    // assert!(reported_children.empty());
    // causes a link-time failure
    assert_eq!(reported_children.len(), 0);
}

#[test]
fn test_root_cgroup_name_is_empty() {
    let root = TestCgroup::new();
    assert_eq!(root.get_reader().name(), OsStr::new(""));
}

#[test]
fn test_validate_cgroup2_fs() {
    let root = TestCgroup::new();
    assert!(root.get_reader_validate().is_err());
}

#[test]
fn test_cgroup_stat_success() {
    let expected_nr_descendants = 10;
    let expected_nr_dying_descendants = 20;
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content(
        "cgroup.stat",
        format!(
             "nr_descendants {expected_nr_descendants}\nnr_dying_descendants {expected_nr_dying_descendants}")
             .as_bytes());

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_cgroup_stat()
        .expect("Failed to read cgroup.stat");
    assert_eq!(
        val.nr_descendants
            .expect("Failed to populate nr_descendants field"),
        expected_nr_descendants
    );
    assert_eq!(
        val.nr_dying_descendants
            .expect("Failed to populate nr_dying_descendants field"),
        expected_nr_dying_descendants
    );
}

#[test]
fn test_cgroup_stat_parse_failure() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content(
        "cgroup.stat",
        b"nr_descendants garbage\nnr_dying_descendantsa garbage",
    );

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_cgroup_stat()
        .expect_err("Failed to read cgroup.stat");
    match err {
        Error::UnexpectedLine(_, _) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_cgroup_stat_invalid_format() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("cgroup.stat", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_cgroup_stat()
        .expect_err("Did not fail to read cgroup.stat");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}
