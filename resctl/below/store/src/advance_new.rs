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

use std::boxed::Box;
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

impl<SampleType> SamplePackage<SampleType> {
    fn new(
        older_sample: Option<SampleType>,
        older_timestamp: SystemTime,
        newer_sample: SampleType,
        newer_timestamp: SystemTime,
    ) -> Self {
        Self {
            older_sample,
            newer_sample,
            timestamp: newer_timestamp,
            duration: newer_timestamp
                .duration_since(older_timestamp)
                .expect("time went backwards"),
        }
    }
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

/// The Advance data structure will be used as an operational
/// bridge between controller and store.
pub struct Advance<FrameType, MType> {
    logger: slog::Logger,
    store: Box<dyn Store<SampleType = FrameType, ModelType = MType>>,
    // below needs two adajcent sample to calculate a model. So we will
    // need to cache one of them while are moving forward or backward
    // continuously to avoid double query.
    // * While the current moving direction is forward, we will cache
    //   the newer_sample.
    // * Otherwise, we will cache the older_sample.
    cached_sample: Option<FrameType>,
    // When we are not moving, target_timestamp means the timestamp of the
    // cached_sample. When we are about to move, the target_timestamp denotes
    // the timestamp we want to move.
    target_timestamp: SystemTime,
    current_direction: Direction,
}

impl<FrameType, ModelType> Advance<FrameType, ModelType> {
    /// Initialize the current advance module.
    // Base on the target_timestamp, we will go forward to find the first
    // available sample. Once we find a sample, we will update the
    // cached_sample and target_timestamp. This function will throw on
    // double initialize.
    pub fn initialize(&mut self) {
        assert!(self.cached_sample.is_none());

        if let Some((timestamp, sample)) = self.store.extract_sample_and_log(
            self.target_timestamp,
            Direction::Forward,
            &self.logger,
        ) {
            self.cached_sample = Some(sample);
            self.target_timestamp = timestamp;
        }
    }

    /// Generate the next Model base on the moving direction.
    //
    // For all of the comment below, we will use the following example samples:
    // [1, 2, 4, 8, 16, 32, 64]
    //
    // Object Saving base on direction:
    // * tl;dr: We always save the newly generated sample regardless of the direction.
    //
    // * Forward: We always save the newer sample and its timestamp.
    //            While we are displaying {8}, we will also need {4} to generate
    //            a model. The moving direction is forward, so we will save {8}.
    //            When next move forward command comes, we will display {16}. So
    //            that we can use the saved {8} to generate a new model. And the
    //            {16} will be saved after generate the model.
    //
    // * Reverse: We always save the older sample and its timestamp.
    //            While we are displaying {8}, we will also need {4} to generate
    //            a model. The moving direction is reverse, so we will save {4}.
    //            When next move reverse command comes, we will display {4}. In
    //            that case, we can just query {2} to generate a new model. And
    //            the {2} will be saved after generate the model.
    //
    // Corner cases:
    // * When direction changes, we will advance two times in the changing direction.
    //   Let's say the current direction is forward, and we are displaying {8}. So
    //   the current cached_sample is {8}. We received a command to move reverse.
    //   So that we are expected to display {4}, which will require sample {2}. When
    //   the first time we call advance(Reverse), we are still displaying {8}, but
    //   the cached_sample becomes {4}. We call advance(Reverse) again, we are will
    //   display {4} and saved {2}, which is what we expected.
    //
    // * When reach either end, we don't move forward. Return None in other words,
    //   and save nothing.
    //
    // * When direction change meet reach the end. We don't need any speical handling
    //   Let's say we are displaying {2} and current direction is forward. We get
    //   a command to move reverse, so we will call advance(reverse) twice. The
    //   first time, we are displaying {2} and save {1}. And the second will return
    //   None. So we changed the direction, but didn't change the display since we
    //   already reached the end.
    pub fn advance(&mut self, direction: Direction) -> Option<ModelType> {
        let target_timestamp = match direction {
            Direction::Forward => self.target_timestamp + Duration::from_secs(1),
            Direction::Reverse => self.target_timestamp - Duration::from_secs(1),
        };

        let (next_timestamp, next_sample) =
            self.store
                .extract_sample_and_log(target_timestamp, direction, &self.logger)?;

        // If we detect a direction change, no need to generate a model, we can
        // just save and move to the next round.
        if direction != self.current_direction {
            self.current_direction = direction;
            self.cached_sample = Some(next_sample);
            self.target_timestamp = next_timestamp;

            return self.advance(direction);
        }

        match direction {
            Direction::Forward => {
                let sample_package = SamplePackage::<FrameType>::new(
                    self.cached_sample.take(),
                    self.target_timestamp,
                    next_sample,
                    next_timestamp,
                );
                let model = self.store.to_model(&sample_package);
                self.cached_sample = Some(sample_package.newer_sample);
                self.target_timestamp = next_timestamp;
                model
            }
            Direction::Reverse => {
                let sample_package = SamplePackage::<FrameType>::new(
                    Some(next_sample),
                    next_timestamp,
                    self.cached_sample.take().expect(
                        "No cached sample avaialbe, the Advance module may not be initialized",
                    ),
                    self.target_timestamp,
                );
                let model = self.store.to_model(&sample_package);
                self.cached_sample = sample_package.older_sample;
                self.target_timestamp = next_timestamp;
                model
            }
        }
    }


    /// jump to the sample at timestamp.
    // We will always use forward jump to make sure we can get two samples before(at)
    // and after the timestamp. One exception here is the timestamp is in future, so
    // if we get None, we will try search backward
    pub fn jump_sample_to(&mut self, timestamp: SystemTime) -> Option<ModelType> {
        let mut sample_package = self.store.get_adjacent_sample_at_timestamp(
            timestamp,
            Direction::Forward,
            &self.logger,
        );

        // timestamp is in future, find the latest sample
        if sample_package.is_none() {
            sample_package = self.store.get_adjacent_sample_at_timestamp(
                timestamp,
                Direction::Reverse,
                &self.logger,
            );
        }

        let sample_package = sample_package?;
        let model = self.store.to_model(&sample_package);
        // We will always set direction to Forward after jump to ease of caching
        self.current_direction = Direction::Forward;
        self.cached_sample = Some(sample_package.newer_sample);
        self.target_timestamp = sample_package.timestamp;

        model
    }

    /// Syntactic sugar for getting lastest sample
    pub fn get_latest_sample(&mut self) -> Option<ModelType> {
        self.jump_sample_to(SystemTime::now())
    }

    /// Syntactic sugar for jump sample forward
    pub fn jump_sample_forward(&mut self, duration: humantime::Duration) -> Option<ModelType> {
        self.jump_sample_to(self.target_timestamp + Duration::from_secs(duration.as_secs()))
    }

    /// Syntactic sugar for jump sample backward
    pub fn jump_sample_backward(&mut self, duration: humantime::Duration) -> Option<ModelType> {
        let gap = Duration::from_secs(duration.as_secs());
        if util::get_unix_timestamp(self.target_timestamp) < gap.as_secs() {
            return None;
        }

        self.jump_sample_to(self.target_timestamp - gap)
    }

    // Convenience function will be used by dump and scuba dump
    pub fn get_next_ts(&self) -> SystemTime {
        // timestamp for initial advance if initialize didn't setup cached_sample
        if self.cached_sample.is_none() {
            return self.target_timestamp;
        }

        match self.current_direction {
            Direction::Forward => self.target_timestamp + Duration::from_secs(1),
            Direction::Reverse => self.target_timestamp - Duration::from_secs(1),
        }
    }
}

/// Construct a new Advance object with local store
pub fn new_advance_local(
    logger: slog::Logger,
    store_dir: PathBuf,
    timestamp: SystemTime,
) -> Advance<DataFrame, Model> {
    let store = Box::new(LocalStore { dir: store_dir });
    Advance {
        logger,
        store,
        cached_sample: None,
        target_timestamp: timestamp,
        current_direction: Direction::Forward,
    }
}

/// Construct a new Advance object with remote store
pub fn new_advance_remote(
    logger: slog::Logger,
    host: String,
    port: Option<u16>,
    timestamp: SystemTime,
) -> Result<Advance<DataFrame, Model>> {
    let store = Box::new(RemoteStore {
        store: crate::remote_store::RemoteStore::new(host, port)?,
    });

    Ok(Advance {
        logger,
        store,
        cached_sample: None,
        target_timestamp: timestamp,
        current_direction: Direction::Forward,
    })
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

    fn get_advance_with_fake_store(timestamp: u64) -> Advance<u64, String> {
        Advance::<u64, String> {
            logger: get_logger(),
            store: Box::new(FakeStore::new()),
            cached_sample: None,
            target_timestamp: util::get_system_time(timestamp),
            current_direction: Direction::Forward,
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

    #[test]
    fn advance_test_initialize() {
        macro_rules! check_advance {
            ($init_time:tt, $expected:expr) => {
                let mut advance = get_advance_with_fake_store($init_time);
                advance.initialize();
                assert_eq!(advance.cached_sample, $expected);

                // When we successfully init the cached_sample, the
                // target_timestamp should be updated accordingly. Otherwise
                // we should not change the target_timestamp
                if advance.cached_sample.is_some() {
                    assert_eq!(
                        advance.target_timestamp,
                        util::get_system_time($expected.expect("Didn't init"))
                    );
                } else {
                    assert_eq!(advance.target_timestamp, util::get_system_time($init_time));
                }
            };
        }
        // Samples: [3, 10, 20, 50]
        // case 1: timestamp at the sample time
        check_advance!(10 /*init_time*/, Some(10) /*expected*/);

        // case 2: timstamp between samples
        check_advance!(4 /*init_time*/, Some(10) /*expected*/);

        // case 3: timestamp earlier than first sample
        check_advance!(2 /*init_time*/, Some(3) /*expected*/);

        // case 4: timestamp later than last sample
        check_advance!(60 /*init_time*/, None /*expected*/);
    }

    macro_rules! advance {
        ($adv:expr, $direction:expr, $expected_cache:expr, $model:expr) => {
            let res = $adv.advance($direction);
            assert_eq!(res, $model);
            assert_eq!($adv.cached_sample, Some($expected_cache));
            assert_eq!(
                $adv.target_timestamp,
                util::get_system_time($expected_cache)
            );
            assert_eq!($adv.current_direction, $direction);
        };
    }

    #[test]
    fn advance_test_advance_continous_move() {
        // Samples: [3, 10, 20, 50]
        let mut advance = get_advance_with_fake_store(3);
        advance.initialize();
        for (old, new) in [(3, 10), (10, 20), (20, 50)].iter() {
            advance!(
                advance,
                Direction::Forward,
                *new, /*expected_cache*/
                Some(format!("{}_{}_{}_{}", old, new, new, new - old))
            );
        }

        // Continuous move forward should return nothing
        for _ in 0..5 {
            advance!(
                advance,
                Direction::Forward,
                50, /*expected_cache*/
                None
            );
        }

        // Reverse
        for (old, new) in [(10, 20), (3, 10)].iter() {
            advance!(
                advance,
                Direction::Reverse,
                *old, /*expected_cache*/
                Some(format!("{}_{}_{}_{}", old, new, new, new - old))
            );
        }

        // Continuous move backward should return nothing
        for _ in 0..5 {
            advance!(advance, Direction::Reverse, 3 /*expected_cache*/, None);
        }
    }

    #[test]
    fn advance_test_advance_direction_change() {
        // Samples: [3, 10, 20, 50]
        let mut advance = get_advance_with_fake_store(10);
        advance.initialize();
        // Displaying 20
        advance!(
            advance,
            Direction::Forward,
            20,                         /*expected_cache*/
            Some("10_20_20_10".into())  /*old_new_ts_duration*/
        );
        // Displaying 10
        advance!(
            advance,
            Direction::Reverse,
            3,                        /*expected_cache*/
            Some("3_10_10_7".into())  /*old_new_ts_duration*/
        );

        // Displaying 10 but direction and cached_sample
        let mut advance = get_advance_with_fake_store(10);
        advance.initialize();
        advance!(
            advance,
            Direction::Reverse,
            3,    /*expected_cache*/
            None  /*old_new_ts_duration*/
        );
    }

    #[test]
    fn advance_test_jump_sample_to() {
        // Samples: [3, 10, 20, 50]
        let mut advance = get_advance_with_fake_store(3);
        advance.initialize();

        macro_rules! check_jump {
            ($query:tt, $expected_cache:expr, $expected_sample:expr) => {
                let timestamp = util::get_system_time($query);
                let res = advance.jump_sample_to(timestamp);
                assert_eq!(res.expect("Failed to get sample"), $expected_sample);
                assert_eq!(advance.current_direction, Direction::Forward);
                assert_eq!(
                    advance.target_timestamp,
                    util::get_system_time($expected_cache)
                );
                assert_eq!(advance.cached_sample, Some($expected_cache));
            };
        }

        // Case 1: Jump to exact timestamp
        check_jump!(
            20,            /*query*/
            20,            /*expected_cache*/
            "10_20_20_10"  /*old_new_ts_dur*/
        );

        // Case 2: Jump to between timestamp
        check_jump!(
            15,            /*query*/
            20,            /*expected_cache*/
            "10_20_20_10"  /*old_new_ts_dur*/
        );

        // Case 3: Jump to future timestamp
        check_jump!(
            60,            /*query*/
            50,            /*expected_cache*/
            "20_50_50_30"  /*old_new_ts_dur*/
        );

        // Case 4: Jump to timestamp that is older than first sample
        check_jump!(
            1,     /*query*/
            3,     /*expected_cache*/
            "3_3"  /*new_ts*/
        );
    }

    #[test]
    fn advance_test_jump_util() {
        // Samples: [3, 10, 20, 50]
        let mut advance = get_advance_with_fake_store(3);
        advance.initialize();

        assert_eq!(
            advance
                .jump_sample_forward(Duration::from_secs(10).into())
                .expect("Failed to jump sample forward"),
            "10_20_20_10" /*old_new_ts_dur*/
        );

        assert_eq!(
            advance
                .jump_sample_backward(Duration::from_secs(10).into())
                .expect("Failed to jump sample backward"),
            "3_10_10_7" /*old_new_ts_dur*/
        );

        assert_eq!(
            advance
                .get_latest_sample()
                .expect("Failed to get lastest sample"),
            "20_50_50_30" /*old_new_ts_dur*/
        );
    }

    #[test]
    fn advance_test_get_next_ts() {
        // Samples: [3, 10, 20, 50]
        let mut advance = get_advance_with_fake_store(3);
        assert_eq!(advance.get_next_ts(), util::get_system_time(3));
        advance.initialize();
        assert_eq!(advance.get_next_ts(), util::get_system_time(4));
        advance.advance(Direction::Forward);
        advance.advance(Direction::Reverse);
        assert_eq!(advance.get_next_ts(), util::get_system_time(2));
    }
}
