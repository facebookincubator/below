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

use std::collections::BTreeMap;
use std::io;
use std::io::prelude::*;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use slog::{self, error, o, Drain};
use tempdir::TempDir;

use crate::below_config::BelowConfig;
use crate::logutil;
use crate::model::{collect_sample, Model};
use crate::store;
use crate::Advance;

use below_thrift::types::Sample;
use below_thrift::DataFrame;

pub fn get_logger() -> slog::Logger {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
    slog::Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!())
}

#[test]
fn record_replay_integration() {
    let dir = TempDir::new("below_record_replay_test").expect("tempdir failed");
    let mut store = store::StoreWriter::new(&dir).expect("Failed to create store");

    // Collect a sample
    let sample = collect_sample(true).expect("failed to collect sample");

    // Validate some data in the sample
    assert!(
        sample
            .cgroup
            .pressure
            .as_ref()
            .expect("missing memory.pressure")
            .memory
            .full
            .total
            .as_ref()
            .expect("missing memory.pressure.total")
            > &0
    );
    let nr_procs = sample.processes.len();
    let hostname = sample.system.hostname.clone();
    let proc0_cgroup = sample
        .processes
        .iter()
        .next()
        .expect("unable to iter")
        .1
        .cgroup
        .clone();
    assert!(nr_procs > 0);
    assert!(hostname.len() > 0);
    assert!(proc0_cgroup.len() > 0);

    // Store two copies of the same sample so the model can generate
    // all the delta fields. Use the same sample twice so we can predict
    // what the deltas will be (0).
    let timestamp = 554433;
    let unix_ts = UNIX_EPOCH + Duration::from_secs(timestamp);
    let df = DataFrame { sample };
    store.put(unix_ts, &df).expect("failed to store sample");
    store
        .put(unix_ts + Duration::from_secs(1), &df)
        .expect("Failed to store second sample");

    // Restore the first sample
    let mut advance = Advance::new(get_logger(), dir.as_ref().to_path_buf(), unix_ts);
    // First sample has incomplete delta data so throw it away
    advance
        .advance(store::Direction::Forward)
        .expect("failed to get advanced data");
    let restored_sample = advance
        .advance(store::Direction::Forward)
        .expect("failed to get advanced data");

    // Validate some values in restored sample
    assert!(
        *restored_sample
            .cgroup
            .pressure
            .as_ref()
            .expect("missing memory.pressure")
            .memory_full_pct
            .as_ref()
            .expect("missing memory pressure full pct")
            == 0.0
    );
    assert!(restored_sample.process.processes.len() == nr_procs);
    assert!(restored_sample.system.hostname == hostname);
    assert!(
        restored_sample
            .process
            .processes
            .iter()
            .next()
            .expect("unable to iter")
            .1
            .cgroup
            .as_ref()
            .expect("missing process cgroup")
            == &proc0_cgroup
    );
}

#[test]
fn advance_forward_and_reverse() {
    let dir = TempDir::new("below_record_replay_test").expect("tempdir failed");
    let mut store = store::StoreWriter::new(&dir).expect("Failed to create store");

    // Collect and store the same sample 3 times
    let timestamp = 554433;
    let unix_ts = UNIX_EPOCH + Duration::from_secs(timestamp);
    let sample = collect_sample(true).expect("failed to collect sample");
    for i in 0..3 {
        let df = DataFrame {
            sample: sample.clone(),
        };
        store
            .put(unix_ts + Duration::from_secs(i), &df)
            .expect("failed to store sample");
    }

    let mut advance = Advance::new(get_logger(), dir.as_ref().to_path_buf(), unix_ts);

    // Basic sanity check that backstep then forward step time works
    for i in 0..3 {
        let sample = advance
            .advance(store::Direction::Forward)
            .expect("failed to get forward data");
        assert_eq!(
            sample
                .timestamp
                .duration_since(UNIX_EPOCH)
                .expect("time before UNIX EPOCH")
                .as_secs(),
            timestamp + i
        );
    }
    let direction_sample = advance
        .advance(store::Direction::Reverse)
        .expect("failed to get reverse data");

    // We advanced forward 3 times and reversed once. That means we should
    // expect to see the 2nd sample (at timestamp + 1).
    assert_eq!(
        direction_sample
            .timestamp
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_secs(),
        timestamp + 1
    );
}

#[test]
fn disable_io_stat() {
    let sample = collect_sample(false).expect("Failed to collect sample");

    assert_eq!(sample.cgroup.io_stat, None);
}

#[test]
fn compound_decorator() {
    static FIO: Lazy<Arc<RwLock<String>>> = Lazy::new(|| Arc::new(RwLock::new(String::new())));
    static TIO: Lazy<Arc<RwLock<String>>> = Lazy::new(|| Arc::new(RwLock::new(String::new())));

    struct FakeFileIO(Sender<bool>, Sender<bool>);
    impl io::Write for FakeFileIO {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.1.send(true).unwrap();
            let mut file_io = FIO.write().unwrap();
            let content = String::from_utf8(buf.to_vec()).unwrap();
            let content_size = content.len();
            *file_io += &content;
            // Depend on the ending char to sendout notification.
            if content.chars().last().unwrap() == '\n' {
                self.0.send(true).unwrap();
            }
            Ok(content_size)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    };

    struct FakeTermIO(Sender<bool>);
    impl io::Write for FakeTermIO {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut term_io = TIO.write().unwrap();
            let content = String::from_utf8(buf.to_vec()).unwrap();
            let content_size = content.len();
            *term_io += &content;
            // Depend on the ending char to sendout notification.
            if content.chars().last().unwrap() == '\n' {
                self.0.send(true).unwrap();
            }
            Ok(content_size)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    };

    let (ftx, frx) = channel::<bool>();
    let (ttx, trx) = channel::<bool>();
    let (rtx, rrx) = channel::<bool>();

    let decorator = logutil::CompoundDecorator::new(FakeFileIO(ftx, rtx), FakeTermIO(ttx));
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, o!());

    error!(logger, "Go both");
    let timeout = Duration::from_secs(3);
    frx.recv_timeout(timeout).expect("failed in file logging.");
    trx.recv_timeout(timeout).expect("failed in term logging.");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go both\n");
        assert_eq!(&term[term.len() - 8..], "Go both\n");
    }

    logutil::set_current_log_target(logutil::TargetLog::File);

    error!(logger, "Go file");
    frx.recv_timeout(timeout).expect("failed in file logging");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go file\n");
        assert_eq!(&term[term.len() - 8..], "Go both\n");
    }

    logutil::set_current_log_target(logutil::TargetLog::Term);

    error!(logger, "Go term");
    trx.recv_timeout(timeout).expect("failed in term logging");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go file\n");
        assert_eq!(&term[term.len() - 8..], "Go term\n");
    }

    logutil::set_current_log_target(logutil::TargetLog::All);

    error!(logger, "Go both");
    frx.recv_timeout(timeout).expect("failed in file logging.");
    trx.recv_timeout(timeout).expect("failed in term logging.");
    {
        let file = FIO.read().unwrap();
        let term = TIO.read().unwrap();
        assert_eq!(&file[file.len() - 8..], "Go both\n");
        assert_eq!(&term[term.len() - 8..], "Go both\n");
    }
    rrx.try_iter().count();

    // Testing race condition during change target and flush
    logutil::set_current_log_target(logutil::TargetLog::File);
    error!(
        logger,
        "Something really long that will take multiple writes"
    );
    rrx.recv_timeout(timeout)
        .expect("Race logger initial wait failed.");
    logutil::set_current_log_target(logutil::TargetLog::Term);
    frx.recv_timeout(timeout)
        .expect("file logger raced with term logger");
    if let Ok(_) = trx.recv_timeout(timeout) {
        panic!("Term logger raced with file logger");
    }
}

#[test]
/// For cgroup io stat that's empty, make sure we report zero's instead of None
fn default_cgroup_io_model() {
    let mut sample: Sample = Default::default();
    let mut last_sample: Sample = Default::default();
    sample.cgroup.io_stat = Some(BTreeMap::new());
    last_sample.cgroup.io_stat = Some(BTreeMap::new());
    let duration = Duration::from_secs(5);

    let model = Model::new(SystemTime::now(), &sample, Some((&last_sample, duration)));
    assert!(model.cgroup.io_total.is_some());
    let io_total = model.cgroup.io_total.unwrap();
    assert_eq!(io_total.rbytes_per_sec, Some(0.0));
    assert_eq!(io_total.wbytes_per_sec, Some(0.0));
    assert_eq!(io_total.rios_per_sec, Some(0.0));
    assert_eq!(io_total.wios_per_sec, Some(0.0));
    assert_eq!(io_total.dbytes_per_sec, Some(0.0));
    assert_eq!(io_total.dios_per_sec, Some(0.0));
}

#[test]
/// When at least one of IO stat sample is None, the IO model should also be.
fn no_cgroup_io_model() {
    let mut sample: Sample = Default::default();
    let mut last_sample: Sample = Default::default();
    for (io_stat, last_io_stat) in &[
        (None, None),
        (None, Some(BTreeMap::new())),
        (Some(BTreeMap::new()), None),
    ] {
        sample.cgroup.io_stat = io_stat.clone();
        last_sample.cgroup.io_stat = last_io_stat.clone();
        let duration = Duration::from_secs(5);

        let model = Model::new(SystemTime::now(), &sample, Some((&last_sample, duration)));
        assert!(model.cgroup.io_total.is_none());
    }
}

#[test]
fn test_config_default() {
    let below_config: BelowConfig = Default::default();
    assert_eq!(below_config.log_dir.to_string_lossy(), "/var/log/below");
    assert_eq!(
        below_config.store_dir.to_string_lossy(),
        "/var/log/below/store"
    );
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
        # I'm a comment
        something_else = "demacia"
    "#;
    file.write_all(config_str.as_bytes())
        .expect("Faild to write temp conf file during testing ignore");
    file.flush().expect("Failed to flush during testing ignore");

    let below_config = match BelowConfig::load(&path) {
        Ok(b) => b,
        Err(e) => panic!("{}", e),
    };
    assert_eq!(below_config.log_dir.to_string_lossy(), "/var/log/below");
    assert_eq!(below_config.store_dir.to_string_lossy(), "/var/log/below");
}

#[test]
fn test_below_load_failed() {
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
        Err(e) => panic!("{}", e),
    };
    assert_eq!(below_config.log_dir.to_string_lossy(), "my magic string");
    assert_eq!(
        below_config.store_dir.to_string_lossy(),
        "/var/log/below/store"
    );
}
