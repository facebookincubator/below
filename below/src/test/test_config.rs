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

use super::fake_view::FakeView;
use super::*;
use cursive::Cursive;
use view::viewrc::{DefaultFrontView, ViewRc};
use view::{cgroup_view::CgroupView, MainViewState, ViewState};

#[test]
fn test_config_default() {
    let below_config: BelowConfig = Default::default();
    assert_eq!(below_config.log_dir.to_string_lossy(), "/var/log/below");
    assert_eq!(
        below_config.store_dir.to_string_lossy(),
        "/var/log/below/store"
    );
    assert_eq!(below_config.cgroup_filter_out, String::new());
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
        cgroup_filter_out = 'user.slice'
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
    assert_eq!(below_config.cgroup_filter_out, "user.slice");
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

#[test]
fn test_viewrc_collapse_cgroups() {
    let cgroup_collapsed = |c: &mut Cursive| -> bool {
        let cgroup_view = CgroupView::get_cgroup_view(c);
        let res = cgroup_view.state.borrow_mut().collapse_all_top_level_cgroup;
        res
    };
    let mut view = FakeView::new();
    view.add_cgroup_view();

    // Test for default setup
    {
        let viewrc: ViewRc = Default::default();
        viewrc.process_collapse_cgroups(&mut view.inner);
        assert!(!cgroup_collapsed(&mut view.inner));
    }

    // Test for collapse_cgroups = false
    {
        let viewrc = ViewRc {
            collapse_cgroups: Some(false),
            ..Default::default()
        };
        viewrc.process_collapse_cgroups(&mut view.inner);
        assert!(!cgroup_collapsed(&mut view.inner));
    }

    // Test for collapse_cgroups = true
    {
        let viewrc = ViewRc {
            collapse_cgroups: Some(true),
            ..Default::default()
        };
        viewrc.process_collapse_cgroups(&mut view.inner);
        assert!(cgroup_collapsed(&mut view.inner));
    }
}

#[test]
fn test_viewrc_default_view() {
    let mut view = FakeView::new();

    let desired_state = vec![
        None,
        Some(DefaultFrontView::Cgroup),
        Some(DefaultFrontView::Process),
        Some(DefaultFrontView::System),
    ];
    let expected_state = vec![
        MainViewState::Cgroup,
        MainViewState::Cgroup,
        MainViewState::Process,
        MainViewState::Core,
    ];
    desired_state
        .into_iter()
        .zip(expected_state)
        .for_each(move |(desired, expected)| {
            let viewrc = ViewRc {
                default_view: desired,
                ..Default::default()
            };
            viewrc.process_default_view(&mut view.inner);
            let current_state = view
                .inner
                .user_data::<ViewState>()
                .expect("No data stored in Cursive object!")
                .main_view_state
                .clone();
            assert_eq!(current_state, expected);
        });
}
