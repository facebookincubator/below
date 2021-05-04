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
use std::time::SystemTime;

use anyhow::Result;

use below_thrift::DataFrame;
use common::util;

use crate::Direction;

/// The store trait defines how should we get a sample from the concrete impl store.
trait Store {
    // We intentionally make this trait generic which not tied to the DataFrame
    // type for ease of testing.
    type SampleType;

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
}

struct LocalStore {
    dir: PathBuf,
}

struct RemoteStore {
    store: crate::remote_store::RemoteStore,
}

impl Store for LocalStore {
    type SampleType = DataFrame;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        logger: slog::Logger,
    ) -> Result<Option<(SystemTime, Self::SampleType)>> {
        crate::read_next_sample(&self.dir, timestamp, direction, logger)
    }
}

impl Store for RemoteStore {
    type SampleType = DataFrame;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        _logger: slog::Logger,
    ) -> Result<Option<(SystemTime, Self::SampleType)>> {
        self.store
            .get_frame(util::get_unix_timestamp(timestamp), direction)
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
}
