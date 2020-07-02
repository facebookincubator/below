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
use crate::util::fold_string;

#[test]
fn record_replay_integration() {
    let dir = TempDir::new("below_record_replay_test").expect("tempdir failed");
    let mut store = store::StoreWriter::new(&dir).expect("Failed to create store");

    // Collect a sample
    let logger = get_logger();
    let sample =
        collect_sample(&get_dummy_exit_data(), true, &logger).expect("failed to collect sample");

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
    assert!(!hostname.is_empty());
    assert!(!proc0_cgroup.is_empty());

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
            .io_total
            .as_ref()
            .expect("missing io.stat")
            .rbytes_per_sec
            .as_ref()
            .expect("missing io stat read bytes per second")
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
    let logger = get_logger();
    let sample =
        collect_sample(&get_dummy_exit_data(), true, &logger).expect("failed to collect sample");
    for i in 0..3 {
        let df = DataFrame {
            sample: sample.clone(),
        };
        store
            .put(unix_ts + Duration::from_secs(i), &df)
            .expect("failed to store sample");
    }

    let mut advance = Advance::new(logger, dir.as_ref().to_path_buf(), unix_ts);

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
    let logger = get_logger();
    let sample =
        collect_sample(&get_dummy_exit_data(), false, &logger).expect("failed to collect sample");

    assert_eq!(sample.cgroup.io_stat, None);
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
fn calculate_cpu_usage() {
    let mut sample: Sample = Default::default();
    let mut last_sample: Sample = Default::default();
    // Actual elapse is (1 + 3 + 4 + 2) = 10s
    sample.system.stat.total_cpu = Some(procfs::CpuStat {
        user_usec: Some(1_000_000),
        nice_usec: Some(0),
        system_usec: Some(3_000_000),
        idle_usec: Some(4_000_000),
        iowait_usec: Some(2_000_000),
        irq_usec: Some(0),
        softirq_usec: Some(0),
        stolen_usec: Some(0),
        guest_usec: Some(0),
        guest_nice_usec: Some(0),
    });
    last_sample.system.stat.total_cpu = Some(procfs::CpuStat {
        user_usec: Some(0),
        nice_usec: Some(0),
        system_usec: Some(0),
        idle_usec: Some(0),
        iowait_usec: Some(0),
        irq_usec: Some(0),
        softirq_usec: Some(0),
        stolen_usec: Some(0),
        guest_usec: Some(0),
        guest_nice_usec: Some(0),
    });
    // Measure as 5s, which could happen if last sample took too long to record
    let model = Model::new(
        SystemTime::now(),
        &sample,
        Some((&last_sample, Duration::from_secs(5))),
    );
    assert_eq!(
        model.system.cpu,
        Some(CpuModel {
            usage_pct: Some(40.0),
            user_pct: Some(10.0),
            system_pct: Some(30.0)
        })
    );
}

#[test]
fn calculate_pressure() {
    let mut sample: Sample = Default::default();
    let mut last_sample: Sample = Default::default();
    // Two measurements are at least 6s apart
    let pressure = cgroupfs::PressureMetrics {
        avg10: Some(90.0),
        avg60: Some(35.0),
        avg300: Some(16.0),
        total: Some(16_000_000),
    };
    let last_pressure = cgroupfs::PressureMetrics {
        avg10: Some(80.0),
        avg60: Some(30.0),
        avg300: Some(15.0),
        total: Some(10_000_000),
    };
    sample.cgroup.pressure = Some(cgroupfs::Pressure {
        cpu: cgroupfs::CpuPressure {
            some: pressure.clone(),
        },
        io: cgroupfs::IoPressure {
            some: pressure.clone(),
            full: pressure.clone(),
        },
        memory: cgroupfs::MemoryPressure {
            some: pressure.clone(),
            full: pressure,
        },
    });
    last_sample.cgroup.pressure = Some(cgroupfs::Pressure {
        cpu: cgroupfs::CpuPressure {
            some: last_pressure.clone(),
        },
        io: cgroupfs::IoPressure {
            some: last_pressure.clone(),
            full: last_pressure.clone(),
        },
        memory: cgroupfs::MemoryPressure {
            some: last_pressure.clone(),
            full: last_pressure,
        },
    });
    // Measure as 5s, which could happen if last sample took too long to record
    let model = Model::new(
        SystemTime::now(),
        &sample,
        Some((&last_sample, Duration::from_secs(5))),
    );
    // Use avg10 of current pressure metrics and ignore last one
    assert_eq!(
        model.cgroup.pressure,
        Some(CgroupPressureModel {
            cpu_some_pct: Some(90.0),
            io_some_pct: Some(90.0),
            io_full_pct: Some(90.0),
            memory_some_pct: Some(90.0),
            memory_full_pct: Some(90.0),
        })
    );
}

#[test]
fn testing_fold_string() {
    assert_eq!(fold_string("demacia", 0, 0, |_| true), "demacia");
    assert_eq!(fold_string("demacia", 3, 0, |_| true), "demacia");
    assert_eq!(fold_string("demacia", 6, 6, |_| true), "demacia");
    assert_eq!(fold_string("demacia", 6, 20, |_| true), "demacia");

    assert_eq!(fold_string("demaciaaaaaaa", 10, 0, |_| false), "dem...aaaa");
    assert_eq!(
        fold_string("d/emaciaaaaaa", 10, 0, |c| c == '/'),
        "d/...aaaaa"
    );
}
