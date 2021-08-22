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

use super::*;

use std::io::Write;

use tempdir::TempDir;

#[test]
fn test_config_default() {
    let below_config: BelowConfig = Default::default();
    assert_eq!(below_config.log_dir.to_string_lossy(), "/var/log/below");
    assert_eq!(
        below_config.store_dir.to_string_lossy(),
        "/var/log/below/store"
    );
    assert_eq!(
        below_config.cgroup_root.to_string_lossy(),
        cgroupfs::DEFAULT_CG_ROOT
    );
    assert_eq!(below_config.cgroup_filter_out, String::new());
    assert_eq!(below_config.killswitch_store_cursor, false);
}

#[test]
fn test_config_fs_failure() {
    let tempdir = TempDir::new("below_config_fs_failuer").expect("Failed to create temp dir");
    let path = tempdir.path();
    match BelowConfig::load(&path.to_path_buf()) {
        Ok(_) => panic!("Below should not load if the non existing path is not default path"),
        Err(e) => assert_eq!(
            format!("{}", e),
            format!("{} exists and is not a file", path.to_string_lossy())
        ),
    }

    let path = tempdir.path().join("below.config");
    match BelowConfig::load(&path) {
        Ok(_) => panic!("Below should not load if the non existing path is not default path"),
        Err(e) => assert_eq!(
            format!("{}", e),
            format!("No such file or directory: {}", path.to_string_lossy())
        ),
    }
}

#[test]
fn test_config_load_success() {
    let tempdir = TempDir::new("below_config_load").expect("Failed to create temp dir");
    let path = tempdir.path().join("below.config");

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .expect("Fail to open below.conf in tempdir");
    let config_str = r#"
        log_dir = '/var/log/below'
        store_dir = '/var/log/below'
        cgroup_root = '/sys/fs/cgroup'
        cgroup_filter_out = 'user.slice'
        killswitch_store_cursor = true
        # I'm a comment
        something_else = "demacia"
    "#;
    file.write_all(config_str.as_bytes())
        .expect("Faild to write temp conf file during testing ignore");
    file.flush().expect("Failed to flush during testing ignore");

    let below_config = match BelowConfig::load(&path) {
        Ok(b) => b,
        Err(e) => panic!("{:#}", e),
    };
    assert_eq!(below_config.log_dir.to_string_lossy(), "/var/log/below");
    assert_eq!(below_config.store_dir.to_string_lossy(), "/var/log/below");
    assert_eq!(below_config.cgroup_root.to_string_lossy(), "/sys/fs/cgroup");
    assert_eq!(below_config.cgroup_filter_out, "user.slice");
    assert_eq!(below_config.killswitch_store_cursor, true);
}

#[test]
fn test_config_load_failed() {
    let tempdir = TempDir::new("below_config_load_failed").expect("Failed to create temp dir");
    let path = tempdir.path().join("below.config");
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .expect("Fail to open below.conf in tempdir");
    let config_str = r#"
        log_dir = '/var/log/below'
        store_dir = '/var/log/below'
        # I'm a comment
        something_else = "demacia"
        Some invalid string that is not a comment
    "#;
    file.write_all(config_str.as_bytes())
        .expect("Faild to write temp conf file during testing ignore");
    file.flush()
        .expect("Failed to flush during testing failure");

    match BelowConfig::load(&path) {
        Ok(_) => panic!("Below should not load since it is an invalid configuration file"),
        Err(e) => assert!(format!("{}", e).starts_with("Failed to parse config file")),
    }
}

#[test]
fn test_config_partial_load() {
    let tempdir = TempDir::new("below_config_load").expect("Failed to create temp dir");
    let path = tempdir.path().join("below.config");

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .expect("Fail to open below.conf in tempdir");
    let config_str = r#"
        log_dir = 'my magic string'
    "#;
    file.write_all(config_str.as_bytes())
        .expect("Faild to write temp conf file during testing ignore");
    file.flush().expect("Failed to flush during testing ignore");

    let below_config = match BelowConfig::load(&path) {
        Ok(b) => b,
        Err(e) => panic!("{:#}", e),
    };
    assert_eq!(below_config.log_dir.to_string_lossy(), "my magic string");
    assert_eq!(
        below_config.store_dir.to_string_lossy(),
        "/var/log/below/store"
    );
}
