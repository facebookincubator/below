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

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use slog::{self, error};

use below_thrift::DataFrame;
use common::util;
use model::Model;

use crate::Direction;

/// A SamplePackage consists of enough information to construct a Model.
// A SamplePackage consists of the sample(newer_sample) at target timestamp
// and a sample before it. This is useful since we will need at least two
// sample to calculate a Model.
struct SamplePackage<SampleType> {
    // The sample before the sample at target timestamp
    older_sample: Option<SampleType>,
    // The sample at target timestamp
    newer_sample: SampleType,
    // The target timetstamp
    timestamp: SystemTime,
    // Duration between two samples
    duration: Duration,
}

impl SamplePackage<DataFrame> {
    pub fn to_model(&self) -> Model {
        // When older_sample is None, we don't provide older_sample to the model
        if let Some(older_sample) = self.older_sample.as_ref() {
            Model::new(
                self.timestamp,
                &self.newer_sample.sample,
                Some((&older_sample.sample, self.duration)),
            )
        } else {
            Model::new(self.timestamp, &self.newer_sample.sample, None)
        }
    }
}

/// The store trait defines how should we get a sample from the concrete impl store.
trait Store {
    // We intentionally make this trait generic which not tied to the DataFrame and Model
    // type for ease of testing.
    // For LocalStore and RemoteStore, SampleType will be DataFrame
    // For FakeStore, SampleType will be u64
    type SampleType;
    // For LocalStore and RemoteStore, ModelType will be Model
    // For FakeStore, ModelType will be string
    type ModelType;

    /// Return the sample time and data frame. Needs to be implemented by
    /// all stores.
    // This function should return the data sample at the provided timestamp.
    // If no sample available at the given timestamp, it will return the
    // first sample after the timestamp if the direction is forward. Otherwise
    // it will return the last sample before the timestamp. This function should
    // return None in the following situation:
    // * reverse search a target that has timestamp earlier than the first recorded
    //   sample
    // * forward search a target that has timestamp later than the last recorded
    //   sample
    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        logger: slog::Logger,
    ) -> Result<Option<(SystemTime, Self::SampleType)>>;

    /// Defines how should we generate a ModelType to a SamplePackage.
    fn to_model(&self, sample_package: &SamplePackage<Self::SampleType>)
        -> Option<Self::ModelType>;

    /// Syntactic sugar to extract the value from the store return and log on error
    fn extract_sample_and_log(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        logger: &slog::Logger,
    ) -> Option<(SystemTime, Self::SampleType)> {
        match self.get_sample_at_timestamp(timestamp, direction, logger.clone()) {
            Ok(None) => None,
            Ok(val) => val,
            Err(e) => {
                error!(logger, "{:#}", e.context("Failed to load from store"));
                None
            }
        }
    }

    /// Return a SamplePackage in order to construct a Model.
    fn get_adjacent_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        logger: &slog::Logger,
    ) -> Option<SamplePackage<Self::SampleType>> {
        // Get and process the target sample
        // Return None if forward find future sample or reverse
        // find the sample older than the first sample
        let (target_ts, target_sample) =
            self.extract_sample_and_log(timestamp, direction, logger)?;

        let mut res_package = SamplePackage {
            older_sample: None,
            newer_sample: target_sample,
            timestamp: target_ts,
            duration: Duration::from_secs(0),
        };

        // Get and process the sample before target sample
        if let Some((older_ts, older_sample)) = self.extract_sample_and_log(
            res_package.timestamp - Duration::from_secs(1),
            Direction::Reverse,
            logger,
        ) {
            res_package.older_sample = Some(older_sample);
            res_package.duration = res_package
                .timestamp
                .duration_since(older_ts)
                .expect("time went backwards");
        }

        Some(res_package)
    }
}

struct LocalStore {
    dir: PathBuf,
}

struct RemoteStore {
    store: crate::remote_store::RemoteStore,
}

impl Store for LocalStore {
    type SampleType = DataFrame;
    type ModelType = Model;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        logger: slog::Logger,
    ) -> Result<Option<(SystemTime, Self::SampleType)>> {
        crate::read_next_sample(&self.dir, timestamp, direction, logger)
    }

    fn to_model(&self, sample_package: &SamplePackage<DataFrame>) -> Option<Model> {
        Some(sample_package.to_model())
    }
}

impl Store for RemoteStore {
    type SampleType = DataFrame;
    type ModelType = Model;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        _logger: slog::Logger,
    ) -> Result<Option<(SystemTime, Self::SampleType)>> {
        self.store
            .get_frame(util::get_unix_timestamp(timestamp), direction)
    }

    fn to_model(&self, sample_package: &SamplePackage<DataFrame>) -> Option<Model> {
        Some(sample_package.to_model())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;

    fn get_logger() -> slog::Logger {
        slog::Logger::root(slog::Discard, slog::o!())
    }

    struct FakeStore {
        sample: Vec<u64>,
        raise_error: bool,
    }

    impl FakeStore {
        fn new() -> Self {
            let mut sample = vec![3, 10, 20, 50];
            sample.sort_unstable();
            Self {
                sample,
                raise_error: false,
            }
        }

        fn raise_error(&mut self) {
            self.raise_error = true;
        }
    }

    impl Store for FakeStore {
        type SampleType = u64;
        type ModelType = String;

        fn to_model(&self, sample_package: &SamplePackage<u64>) -> Option<String> {
            // When duration is 0, we don't provide older_sample to the model
            if let Some(older_sample) = sample_package.older_sample.as_ref() {
                Some(format!(
                    "{}_{}_{}_{}",
                    older_sample,
                    sample_package.newer_sample,
                    util::get_unix_timestamp(sample_package.timestamp),
                    sample_package.duration.as_secs()
                ))
            } else {
                Some(format!(
                    "{}_{}",
                    sample_package.newer_sample,
                    util::get_unix_timestamp(sample_package.timestamp)
                ))
            }
        }

        fn get_sample_at_timestamp(
            &mut self,
            timestamp: SystemTime,
            direction: Direction,
            _logger: slog::Logger,
        ) -> Result<Option<(SystemTime, Self::SampleType)>> {
            if self.raise_error {
                bail!("error");
            }

            let timestamp = util::get_unix_timestamp(timestamp);
            // corner cases
            if self.sample.is_empty()
                || (timestamp < *self.sample.first().unwrap() && direction == Direction::Reverse)
                || (timestamp > *self.sample.last().unwrap() && direction == Direction::Forward)
            {
                return Ok(None);
            }

            match self.sample.binary_search(&timestamp) {
                Ok(_) => Ok(Some((util::get_system_time(timestamp), timestamp))),
                Err(idx) => match direction {
                    Direction::Reverse => Ok(Some((
                        util::get_system_time(self.sample[idx - 1]),
                        self.sample[idx - 1],
                    ))),
                    Direction::Forward => Ok(Some((
                        util::get_system_time(self.sample[idx]),
                        self.sample[idx],
                    ))),
                },
            }
        }
    }

    // Testing the Store trait interface and behavior correctness for
    // FakeStore.
    #[test]
    fn store_operation_test_with_fake_store() {
        let mut store = FakeStore::new();
        // We didn't use closure here to reveal line number for test failure
        macro_rules! check_sample {
            ($query:tt, $expected:tt, $direction:expr) => {
                let timestamp = util::get_system_time($query);
                let res = store.get_sample_at_timestamp(timestamp, $direction, get_logger());
                assert_eq!(
                    res.expect("Fail to get sample."),
                    Some((util::get_system_time($expected), $expected))
                );
            };
            ($query:tt, $direction:expr) => {
                let timestamp = util::get_system_time($query);
                let res = store.get_sample_at_timestamp(timestamp, $direction, get_logger());
                assert_eq!(res.expect("Fail to get sample."), None);
            };
        }

        // Exact match
        check_sample!(20 /*query*/, 20 /*expected*/, Direction::Forward);
        check_sample!(20 /*query*/, 20 /*expected*/, Direction::Reverse);

        // When query time is earlier than first sample
        // should return first sample for forward search
        check_sample!(0 /*query*/, 3 /*expected*/, Direction::Forward);
        // should return none for reverse search
        check_sample!(0 /*query*/, Direction::Reverse);

        // When query time is later than last sample
        // should return None for forward search
        check_sample!(60 /*query*/, Direction::Forward);
        // should return last sample for reverse search
        check_sample!(60 /*query*/, 50 /*expected*/, Direction::Reverse);

        // When query time is within the interval
        check_sample!(30 /*query*/, 50 /*expected*/, Direction::Forward);
        check_sample!(30 /*query*/, 20 /*expected*/, Direction::Reverse);

        store.raise_error();
        let res = store.get_sample_at_timestamp(
            util::get_system_time(0),
            Direction::Forward,
            get_logger(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn store_operation_test_get_adjacent_sample_at_timestamp() {
        let mut store = FakeStore::new();

        macro_rules! check_sample {
            ($query:tt, $direction:expr, $expected_sample:expr) => {
                let timestamp = util::get_system_time($query);
                let res =
                    store.get_adjacent_sample_at_timestamp(timestamp, $direction, &get_logger());
                assert_eq!(
                    store
                        .to_model(&res.expect("Failed to get sample"))
                        .expect("Failed to convert sample to model"),
                    $expected_sample
                );
            };
            ($query:tt, $direction:expr) => {
                let timestamp = util::get_system_time($query);
                let res =
                    store.get_adjacent_sample_at_timestamp(timestamp, $direction, &get_logger());
                assert!(res.is_none());
            };
        }

        // case 1: timestamp at the available sample
        for direction in [Direction::Forward, Direction::Reverse].iter() {
            // [3, 10, 20, 50]
            check_sample!(
                10, /*query*/
                *direction,
                "3_10_10_7" /*old_new_timestamp_duraion*/
            );
        }

        // case 2: timestamp between two available samples
        // [3, 10, 20, 50]
        check_sample!(
            7, /*query*/
            Direction::Forward,
            "3_10_10_7" /*old_new_timestamp_duraion*/
        );

        check_sample!(
            7, /*query*/
            Direction::Reverse,
            "3_3" /*new_timestamp*/
        );

        check_sample!(
            12, /*query*/
            Direction::Reverse,
            "3_10_10_7" /*old_new_timestamp_duraion*/
        );

        // case 3: timestamp before first sample
        // [3, 10, 20, 50]
        check_sample!(
            0, /*query*/
            Direction::Forward,
            "3_3" /*new_timestamp*/
        );

        check_sample!(0 /*query*/, Direction::Reverse);

        // case 4: timestamp after the last sample
        // [3, 10, 20, 50]
        check_sample!(
            60, /*query*/
            Direction::Reverse,
            "20_50_50_30" /*old_new_timestamp_duraion*/
        );

        check_sample!(60 /*query*/, Direction::Forward);
    }
}
