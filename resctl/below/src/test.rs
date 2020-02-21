use std::time::{Duration, UNIX_EPOCH};

use slog::{self, Drain};
use tempdir::TempDir;

use crate::model::collect_sample;
use crate::store;
use crate::Advance;

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
