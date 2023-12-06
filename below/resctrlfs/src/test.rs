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
use std::fs::create_dir_all;
use std::fs::File;
use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::path::Path;
use std::path::PathBuf;

use maplit::btreemap;
use maplit::btreeset;
use paste::paste;
use tempfile::TempDir;

use crate::*;

macro_rules! test_success {
    ($name:ident, $filename:literal, $contents:literal, $expected_val:stmt, $suffix:ident) => {
        paste! {
            #[test]
            fn [<test_ $name _success_ $suffix>]() {
                let test_group = TestGenericGroup::new();
                let reader = ResctrlGroupReader::new(test_group.path())
                    .expect("Failed to create reader");
                test_group.create_file_with_content($filename, $contents);
                let val = reader
                    .$name()
                    .expect(concat!("Failed to read ", $filename));
                assert_eq!(val, {$expected_val});
            }
        }
    };
    ($name:ident, $filename:literal, $contents:literal, $expected_val:stmt) => {
        test_success!($name, $filename, $contents, $expected_val, "");
    };
}

macro_rules! test_failure {
    ($name:ident, $filename:literal, $err_contents:literal, $suffix:ident) => {
        paste! {
            #[test]
            fn [<test_ $name _failure_ $suffix>]() {
                let test_group = TestGenericGroup::new();
                let reader = ResctrlGroupReader::new(test_group.path())
                    .expect("Failed to create reader");
                test_group.create_file_with_content($filename, $err_contents);
                let val = reader.$name();
                assert!(val.is_err());
            }
        }
    };
    ($name:ident, $filename:literal, $err_contents:literal) => {
        test_failure!($name, $filename, $err_contents, "");
    };
}

trait TestGroupCommon {
    fn path(&self) -> PathBuf;

    fn create_child_dir<P: AsRef<Path>>(&self, p: P) -> PathBuf {
        let path = self.path().join(p);
        std::fs::create_dir(&path)
            .unwrap_or_else(|_| panic!("Failed to create child dir {}", path.display()));
        path
    }

    fn create_file_with_content<P: AsRef<Path>>(&self, p: P, content: &[u8]) {
        let path = self.path().join(p);
        create_dir_all(path.parent().unwrap()).unwrap();
        let mut file =
            File::create(&path).unwrap_or_else(|_| panic!("Failed to create {}", path.display()));
        file.write_all(content)
            .unwrap_or_else(|_| panic!("Failed to write to {}", path.display()));
    }

    fn set_cpus_list(&self, list: &[u8]) {
        self.create_file_with_content(OsStr::new("cpus_list"), list);
    }

    fn set_mode(&self, mode: &[u8]) {
        self.create_file_with_content(OsStr::new("mode"), mode);
    }
}

struct TestResctrlfs {
    tempdir: TempDir,
    ctrl_mon: TestCtrlMonGroup,
}

struct TestCtrlMonGroup {
    path: PathBuf,
}

struct TestMonGroup {
    path: PathBuf,
}

struct TestGenericGroup {
    tempdir: TempDir,
}

impl TestGroupCommon for TestResctrlfs {
    fn path(&self) -> PathBuf {
        self.tempdir.path().to_path_buf()
    }
}

impl TestResctrlfs {
    fn new() -> TestResctrlfs {
        let tempdir = TempDir::new().expect("Failed to create tempdir");
        let ctrl_mon = TestCtrlMonGroup::new(tempdir.path().to_path_buf());
        TestResctrlfs { tempdir, ctrl_mon }
    }

    fn initialize(&self) {
        self.create_child_dir(OsStr::new("info"));
        self.ctrl_mon.initialize(b"0-7\n", b"shareable\n");
    }

    fn create_child_ctrl_mon<P: AsRef<Path>>(&self, p: P) -> TestCtrlMonGroup {
        let path = self.create_child_dir(p);
        TestCtrlMonGroup::new(path)
    }

    fn create_child_mon_group<P: AsRef<Path>>(&self, p: P) -> TestMonGroup {
        let path = self.create_child_dir(PathBuf::from(OsStr::new("mon_groups")).join(p));
        TestMonGroup::new(path)
    }
}

impl TestGroupCommon for TestCtrlMonGroup {
    fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

impl TestCtrlMonGroup {
    fn new(path: PathBuf) -> TestCtrlMonGroup {
        TestCtrlMonGroup { path }
    }

    fn initialize(&self, cpus_list: &[u8], mode: &[u8]) {
        self.set_cpus_list(cpus_list);
        self.set_mode(mode);
        self.create_child_dir(OsStr::new("mon_data"));
        self.create_child_dir(OsStr::new("mon_groups"));
    }

    fn create_child_mon_group<P: AsRef<Path>>(&self, p: P) -> TestMonGroup {
        let path = self.create_child_dir(PathBuf::from(OsStr::new("mon_groups")).join(p));
        TestMonGroup::new(path)
    }
}

impl TestGroupCommon for TestMonGroup {
    fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

impl TestMonGroup {
    fn new(path: PathBuf) -> TestMonGroup {
        TestMonGroup { path }
    }

    fn initialize(&self, cpus_list: &[u8]) {
        self.set_cpus_list(cpus_list);
        self.create_child_dir(OsStr::new("mon_data"));
        self.create_child_dir(OsStr::new("mon_groups"));
    }
}

impl TestGenericGroup {
    fn new() -> TestGenericGroup {
        let tempdir = TempDir::new().expect("Failed to create tempdir");
        TestGenericGroup { tempdir }
    }
}

impl TestGroupCommon for TestGenericGroup {
    fn path(&self) -> PathBuf {
        self.tempdir.path().to_path_buf()
    }
}

#[test]
fn test_resctrlfs_read_empty() {
    let resctrlfs = TestResctrlfs::new();
    resctrlfs.initialize();
    let reader = ResctrlReader::new(resctrlfs.path().to_path_buf(), false)
        .expect("Failed to construct reader");
    reader.read_all().expect("Failed to read all");
}

#[test]
fn test_resctrlfs_read_simple() {
    let resctrlfs = TestResctrlfs::new();
    {
        // Set up filesystem
        resctrlfs.initialize();
        resctrlfs.create_file_with_content(OsStr::new("mon_data/mon_L3_00/llc_occupancy"), b"0\n");
        resctrlfs.create_file_with_content(OsStr::new("mon_data/mon_L3_11/llc_occupancy"), b"11\n");
        resctrlfs
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_total_bytes"), b"100\n");
        resctrlfs
            .create_file_with_content(OsStr::new("mon_data/mon_L3_11/mbm_total_bytes"), b"111\n");
        resctrlfs
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_local_bytes"), b"200\n");
        resctrlfs
            .create_file_with_content(OsStr::new("mon_data/mon_L3_11/mbm_local_bytes"), b"211\n");

        let ctrl_mon_1 = resctrlfs.create_child_ctrl_mon(OsStr::new("ctrl_mon_1"));
        ctrl_mon_1.initialize(b"0-3\n", b"shareable\n");
        ctrl_mon_1.create_file_with_content(OsStr::new("mon_data/mon_L3_00/llc_occupancy"), b"0\n");
        ctrl_mon_1
            .create_file_with_content(OsStr::new("mon_data/mon_L3_12/llc_occupancy"), b"11\n");
        ctrl_mon_1
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_total_bytes"), b"100\n");
        ctrl_mon_1
            .create_file_with_content(OsStr::new("mon_data/mon_L3_12/mbm_total_bytes"), b"111\n");
        ctrl_mon_1
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_local_bytes"), b"200\n");
        ctrl_mon_1
            .create_file_with_content(OsStr::new("mon_data/mon_L3_12/mbm_local_bytes"), b"211\n");

        let _ctrl_mon_2 = resctrlfs
            .create_child_ctrl_mon(OsStr::new("ctrl_mon_2"))
            .initialize(b"4-5\n", b"exclusive\n");

        let inner_mon = ctrl_mon_1.create_child_mon_group(OsStr::new("mon_1"));
        inner_mon.initialize(b"1-2\n");
        inner_mon.create_file_with_content(OsStr::new("mon_data/mon_L3_00/llc_occupancy"), b"0\n");
        inner_mon.create_file_with_content(OsStr::new("mon_data/mon_L3_13/llc_occupancy"), b"11\n");
        inner_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_total_bytes"), b"100\n");
        inner_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_13/mbm_total_bytes"), b"111\n");
        inner_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_local_bytes"), b"200\n");
        inner_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_13/mbm_local_bytes"), b"211\n");

        let top_level_mon = resctrlfs.create_child_mon_group(OsStr::new("mon_0"));
        top_level_mon.initialize(b"0-1\n");
        top_level_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/llc_occupancy"), b"0\n");
        top_level_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_14/llc_occupancy"), b"11\n");
        top_level_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_total_bytes"), b"100\n");
        top_level_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_14/mbm_total_bytes"), b"111\n");
        top_level_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_00/mbm_local_bytes"), b"200\n");
        top_level_mon
            .create_file_with_content(OsStr::new("mon_data/mon_L3_14/mbm_local_bytes"), b"211\n");
    }

    let reader = ResctrlReader::new(resctrlfs.path().to_path_buf(), false)
        .expect("Failed to construct reader");
    let sample = reader.read_all().expect("Failed to read all");
    assert_eq!(sample.mode, Some(GroupMode::Shareable));
    assert_eq!(
        sample.cpuset,
        Some(Cpuset {
            cpus: btreeset! {0, 1,
            2, 3, 4, 5, 6, 7}
        })
    );
    assert_eq!(
        sample.mon_stat,
        Some(MonStat {
            l3_mon_stat: Some(btreemap! {
                0 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(0)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(100)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(200))
                },
                11 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(11)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(111)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(211))
                }
            })
        })
    );
    assert!(sample.ctrl_mon_groups.is_some());
    assert_eq!(sample.ctrl_mon_groups.as_ref().unwrap().len(), 2);

    let ctrl_mon_1 = &sample.ctrl_mon_groups.as_ref().unwrap()["ctrl_mon_1"];
    assert!(ctrl_mon_1.inode_number.is_some());
    assert_eq!(ctrl_mon_1.mode, Some(GroupMode::Shareable));
    assert_eq!(
        ctrl_mon_1.cpuset,
        Some(Cpuset {
            cpus: btreeset! {0,1,2,3}
        })
    );
    assert_eq!(
        ctrl_mon_1.mon_stat,
        Some(MonStat {
            l3_mon_stat: Some(btreemap! {
                0 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(0)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(100)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(200))
                },
                12 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(11)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(111)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(211))
                }
            })
        })
    );

    let ctrl_mon_2 = &sample.ctrl_mon_groups.as_ref().unwrap()["ctrl_mon_2"];
    assert!(ctrl_mon_2.inode_number.is_some());
    assert_eq!(ctrl_mon_2.mode, Some(GroupMode::Exclusive));
    assert_eq!(
        ctrl_mon_2.cpuset,
        Some(Cpuset {
            cpus: btreeset! {4,5}
        })
    );
    assert_eq!(
        ctrl_mon_2.mon_stat,
        Some(MonStat {
            l3_mon_stat: Some(btreemap! {})
        })
    );

    let inner_mon = &ctrl_mon_1.mon_groups.as_ref().unwrap()["mon_1"];
    assert!(inner_mon.inode_number.is_some());
    assert_eq!(
        inner_mon.cpuset,
        Some(Cpuset {
            cpus: btreeset! {1,2}
        })
    );
    assert_eq!(
        inner_mon.mon_stat,
        Some(MonStat {
            l3_mon_stat: Some(btreemap! {
                0 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(0)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(100)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(200))
                },
                13 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(11)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(111)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(211))
                }
            })
        })
    );

    let top_level_mon = &sample.mon_groups.as_ref().unwrap()["mon_0"];
    assert!(top_level_mon.inode_number.is_some());
    assert_eq!(
        top_level_mon.cpuset,
        Some(Cpuset {
            cpus: btreeset! {0, 1}
        })
    );
    assert_eq!(
        top_level_mon.mon_stat,
        Some(MonStat {
            l3_mon_stat: Some(btreemap! {
                0 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(0)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(100)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(200))
                },
                14 => L3MonStat {
                    llc_occupancy_bytes: Some(RmidBytes::Bytes(11)),
                    mbm_total_bytes: Some(RmidBytes::Bytes(111)),
                    mbm_local_bytes: Some(RmidBytes::Bytes(211))
                }
            })
        })
    );
}

#[test]
fn test_read_inode_number() {
    let group = TestGenericGroup::new();
    let reader = ResctrlGroupReader::new(group.path()).expect("Failed to construct reader");
    let inode = reader
        .read_inode_number()
        .expect("Failed to read inode number");
    assert_eq!(
        inode,
        std::fs::metadata(group.path())
            .expect("Failed to read inode number with fs::metadata")
            .st_ino()
    );
}

test_success!(
    read_cpuset,
    "cpus_list",
    b"",
    Cpuset {
        cpus: BTreeSet::new()
    },
    empty_file
);
test_success!(
    read_cpuset,
    "cpus_list",
    b"\n",
    Cpuset {
        cpus: BTreeSet::new()
    },
    single_empty_line
);
test_success!(
    read_cpuset,
    "cpus_list",
    b"1\n",
    Cpuset {
        cpus: BTreeSet::from([1])
    },
    single_cpu
);
test_success!(
    read_cpuset,
    "cpus_list",
    b"1,3-5\n",
    Cpuset {
        cpus: BTreeSet::from([1, 3, 4, 5])
    },
    multi_cpu_with_range
);
test_failure!(read_cpuset, "cpus_list", b"-1\n", negative_cpu);
test_failure!(read_cpuset, "cpus_list", b"c\n", invalid_char);

test_success!(
    read_mode,
    "mode",
    b"exclusive\n",
    GroupMode::Exclusive,
    exclusive
);
test_success!(
    read_mode,
    "mode",
    b"shareable\n",
    GroupMode::Shareable,
    shareable
);
test_failure!(read_mode, "mode", b"invalid_mode\n", invalid);
test_failure!(read_mode, "mode", b"\n", empty);

test_success!(
    read_l3_mon_stat,
    "llc_occupancy",
    b"123456789\n",
    L3MonStat {
        llc_occupancy_bytes: Some(RmidBytes::Bytes(123456789)),
        ..Default::default()
    },
    llc_occupancy
);
test_success!(
    read_l3_mon_stat,
    "llc_occupancy",
    b"Unavailable\n",
    L3MonStat {
        llc_occupancy_bytes: Some(RmidBytes::Unavailable),
        ..Default::default()
    },
    llc_occupancy_unavailable
);
test_failure!(
    read_l3_mon_stat,
    "llc_occupancy",
    b"-1\n",
    llc_occupancy_negative
);

test_success!(
    read_l3_mon_stat,
    "mbm_total_bytes",
    b"123\n",
    L3MonStat {
        mbm_total_bytes: Some(RmidBytes::Bytes(123)),
        ..Default::default()
    },
    mbm_total_bytes
);
test_success!(
    read_l3_mon_stat,
    "mbm_total_bytes",
    b"Unavailable\n",
    L3MonStat {
        mbm_total_bytes: Some(RmidBytes::Unavailable),
        ..Default::default()
    },
    mbm_total_bytes_unavailable
);

test_success!(
    read_l3_mon_stat,
    "mbm_local_bytes",
    b"123\n",
    L3MonStat {
        mbm_local_bytes: Some(RmidBytes::Bytes(123)),
        ..Default::default()
    },
    mbm_local_bytes
);
test_success!(
    read_l3_mon_stat,
    "mbm_local_bytes",
    b"Unavailable\n",
    L3MonStat {
        mbm_local_bytes: Some(RmidBytes::Unavailable),
        ..Default::default()
    },
    mbm_local_bytes_unavailable
);
