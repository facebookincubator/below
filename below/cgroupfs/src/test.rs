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

use paste::paste;
use tempfile::TempDir;

use crate::CgroupReader;
use crate::Error;
use crate::MemoryNumaStat;

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

macro_rules! test_success {
    ($name:ident, $filename:literal, $contents:literal, $expected_val:stmt) => {
        paste! {
            #[test]
            fn [<test_ $name _success>]() {
                let cgroup = TestCgroup::new();
                cgroup.create_file_with_content($filename, $contents);
                let cgroup_reader = cgroup.get_reader();
                let val = cgroup_reader
                    .$name()
                    .expect(concat!("Failed to read ", $filename));
                assert_eq!(val, {$expected_val});
            }
        }
    };
}

macro_rules! test_failure {
    ($name:ident, $filename:literal, $err_contents:literal) => {
        paste! {
            #[test]
            fn [<test_ $name _failure>]() {
                let cgroup = TestCgroup::new();
                let cgroup_reader = cgroup.get_reader();
                let err = cgroup_reader.$name().expect_err(
                    concat!("Did not fail to read ", $filename));
                match err {
                    Error::IoError(_, e)
                        if e.kind() == std::io::ErrorKind::NotFound => (),
                    _ => panic!("Got unexpected error type {}", err),
                };
                cgroup.create_file_with_content($filename, $err_contents);
                let val = cgroup_reader.$name();
                assert!(val.is_err());
            }
        }
    };
}

macro_rules! singleline_integer_or_max_test {
    ($name:ident, $filename:literal) => {
        test_success!($name, $filename, b"1234\n", 1234);
        test_failure!($name, $filename, b"-1\n");

        paste! {
            #[test]
            fn [<test_ $name _max_success>]() {
                let cgroup = TestCgroup::new();
                cgroup.create_file_with_content($filename, b"max\n");
                let cgroup_reader = cgroup.get_reader();
                let val = cgroup_reader
                    .$name()
                    .expect(concat!("Failed to read ", $filename));
                assert_eq!(val, -1); // -1 means "max"
            }
        }
    };
}

singleline_integer_or_max_test!(read_memory_low, "memory.low");
singleline_integer_or_max_test!(read_memory_high, "memory.high");
singleline_integer_or_max_test!(read_memory_max, "memory.max");
singleline_integer_or_max_test!(read_memory_swap_max, "memory.swap.max");
singleline_integer_or_max_test!(read_memory_zswap_max, "memory.zswap.max");

test_success!(read_cpu_weight, "cpu.weight", b"10000\n", 10000);
test_failure!(read_cpu_weight, "cpu.weight", b"5000000000\n");

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

// TODO(brianc118): don't dup test names
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

#[test]
fn test_memory_numa_stat_success() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content(
        "memory.numa_stat",
        b"anon N0=133948178432 N1=85731622912 N2=56469581824 N3=67508137984
file N0=29022474240 N1=28619689984 N2=27863502848 N3=20205821952
kernel_stack N0=139689984 N1=93978624 N2=104693760 N3=145391616
pagetables N0=464572416 N1=392798208 N2=332378112 N3=352788480
shmem N0=27244945408 N1=27178311680 N2=27170930688 N3=13595549696
file_mapped N0=27685949440 N1=27733299200 N2=27522891776 N3=15582023680
file_dirty N0=6488064 N1=17436672 N2=85155840 N3=165040128
file_writeback N0=0 N1=0 N2=38522880 N3=123408384
swapcached N0=0 N1=0 N2=0 N3=0
anon_thp N0=1419771904 N1=673185792 N2=536870912 N3=681574400
file_thp N0=48234496 N1=14680064 N2=0 N3=48234496
shmem_thp N0=48234496 N1=8388608 N2=0 N3=8388608
inactive_anon N0=160479961088 N1=112294313984 N2=83386806272 N3=80141840384
active_anon N0=466096128 N1=398712832 N2=8605696 N3=599347200
inactive_file N0=1189363712 N1=913510400 N2=448503808 N3=2159505408
active_file N0=522350592 N1=460431360 N2=206303232 N3=4300206080
unevictable N0=405504 N1=135168 N2=0 N3=0
slab_reclaimable N0=663340528 N1=563089336 N2=514239048 N3=647222000
slab_unreclaimable N0=686272088 N1=472630728 N2=556250640 N3=693263576
workingset_refault_anon N0=3497864 N1=2225803 N2=2410565 N3=1468001
workingset_refault_file N0=214724399 N1=172943243 N2=2094241456 N3=155295239
workingset_activate_anon N0=477507 N1=238415 N2=318245 N3=235467
workingset_activate_file N0=96231899 N1=75825820 N2=674636976 N3=61718859
workingset_restore_anon N0=182593 N1=55793 N2=96984 N3=53780
workingset_restore_file N0=74008297 N1=63719159 N2=528595708 N3=48463497
workingset_nodereclaim N0=266941 N1=176289 N2=1260264 N3=638641",
    );

    let cgroup_reader = cgroup.get_reader();
    let val = cgroup_reader
        .read_memory_numa_stat()
        .expect("Failed to read numa memory stat");

    assert_eq!(val.len(), 4);
    let node0 = MemoryNumaStat {
        anon: Some(133948178432),
        file: Some(29022474240),
        kernel_stack: Some(139689984),
        pagetables: Some(464572416),
        shmem: Some(27244945408),
        file_mapped: Some(27685949440),
        file_dirty: Some(6488064),
        file_writeback: Some(0),
        swapcached: Some(0),
        anon_thp: Some(1419771904),
        file_thp: Some(48234496),
        shmem_thp: Some(48234496),
        inactive_anon: Some(160479961088),
        active_anon: Some(466096128),
        inactive_file: Some(1189363712),
        active_file: Some(522350592),
        unevictable: Some(405504),
        slab_reclaimable: Some(663340528),
        slab_unreclaimable: Some(686272088),
        workingset_refault_anon: Some(3497864),
        workingset_refault_file: Some(214724399),
        workingset_activate_anon: Some(477507),
        workingset_activate_file: Some(96231899),
        workingset_restore_anon: Some(182593),
        workingset_restore_file: Some(74008297),
        workingset_nodereclaim: Some(266941),
    };
    let node1 = MemoryNumaStat {
        anon: Some(85731622912),
        file: Some(28619689984),
        kernel_stack: Some(93978624),
        pagetables: Some(392798208),
        shmem: Some(27178311680),
        file_mapped: Some(27733299200),
        file_dirty: Some(17436672),
        file_writeback: Some(0),
        swapcached: Some(0),
        anon_thp: Some(673185792),
        file_thp: Some(14680064),
        shmem_thp: Some(8388608),
        inactive_anon: Some(112294313984),
        active_anon: Some(398712832),
        inactive_file: Some(913510400),
        active_file: Some(460431360),
        unevictable: Some(135168),
        slab_reclaimable: Some(563089336),
        slab_unreclaimable: Some(472630728),
        workingset_refault_anon: Some(2225803),
        workingset_refault_file: Some(172943243),
        workingset_activate_anon: Some(238415),
        workingset_activate_file: Some(75825820),
        workingset_restore_anon: Some(55793),
        workingset_restore_file: Some(63719159),
        workingset_nodereclaim: Some(176289),
    };
    let node2 = MemoryNumaStat {
        anon: Some(56469581824),
        file: Some(27863502848),
        kernel_stack: Some(104693760),
        pagetables: Some(332378112),
        shmem: Some(27170930688),
        file_mapped: Some(27522891776),
        file_dirty: Some(85155840),
        file_writeback: Some(38522880),
        swapcached: Some(0),
        anon_thp: Some(536870912),
        file_thp: Some(0),
        shmem_thp: Some(0),
        inactive_anon: Some(83386806272),
        active_anon: Some(8605696),
        inactive_file: Some(448503808),
        active_file: Some(206303232),
        unevictable: Some(0),
        slab_reclaimable: Some(514239048),
        slab_unreclaimable: Some(556250640),
        workingset_refault_anon: Some(2410565),
        workingset_refault_file: Some(2094241456),
        workingset_activate_anon: Some(318245),
        workingset_activate_file: Some(674636976),
        workingset_restore_anon: Some(96984),
        workingset_restore_file: Some(528595708),
        workingset_nodereclaim: Some(1260264),
    };
    let node3 = MemoryNumaStat {
        anon: Some(67508137984),
        file: Some(20205821952),
        kernel_stack: Some(145391616),
        pagetables: Some(352788480),
        shmem: Some(13595549696),
        file_mapped: Some(15582023680),
        file_dirty: Some(165040128),
        file_writeback: Some(123408384),
        swapcached: Some(0),
        anon_thp: Some(681574400),
        file_thp: Some(48234496),
        shmem_thp: Some(8388608),
        inactive_anon: Some(80141840384),
        active_anon: Some(599347200),
        inactive_file: Some(2159505408),
        active_file: Some(4300206080),
        unevictable: Some(0),
        slab_reclaimable: Some(647222000),
        slab_unreclaimable: Some(693263576),
        workingset_refault_anon: Some(1468001),
        workingset_refault_file: Some(155295239),
        workingset_activate_anon: Some(235467),
        workingset_activate_file: Some(61718859),
        workingset_restore_anon: Some(53780),
        workingset_restore_file: Some(48463497),
        workingset_nodereclaim: Some(638641),
    };
    assert_eq!(val[&0], node0);
    assert_eq!(val[&1], node1);
    assert_eq!(val[&2], node2);
    assert_eq!(val[&3], node3);
}

#[test]
fn test_memory_numa_stat_parse_failure() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.numa_stat", b"anon garbage\nfile garbage");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_numa_stat()
        .expect_err("Did not fail to read memory.numa.stat");
    match err {
        Error::UnexpectedLine(_, _) => {}
        _ => panic!("Got unexpected error type {}", err),
    }
}

#[test]
fn test_memory_numa_stat_invalid_format() {
    let cgroup = TestCgroup::new();
    cgroup.create_file_with_content("memory.numa_stat", b"");

    let cgroup_reader = cgroup.get_reader();
    let err = cgroup_reader
        .read_memory_numa_stat()
        .expect_err("Did not fail to read memory.numa_stat");
    match err {
        Error::InvalidFileFormat(_) => {}
        _ => panic!("Got unexpected error type: {}", err),
    }
}
