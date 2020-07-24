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

use anyhow::{bail, Context, Result};
use slog::{self, error};

use crate::model::{self, Model};
use crate::remote_store;
use crate::store;
use crate::util;
use below_thrift::{DataFrame, Sample};

enum AdvanceStore {
    Local(PathBuf),
    Remote(remote_store::RemoteStore),
}

// Object to manage advancing through the store and constructing the
// data model. Separated out as its own struct so we can replace
// Advance with this whole sale
pub struct Advance {
    logger: slog::Logger,
    store: AdvanceStore,
    direction: store::Direction,
    last_sample: Option<Sample>,
    last_sample_time: SystemTime,
}

impl Advance {
    pub fn new(logger: slog::Logger, store_dir: PathBuf, timestamp: SystemTime) -> Advance {
        Advance {
            logger,
            store: AdvanceStore::Local(store_dir),
            direction: store::Direction::Forward,
            last_sample: None,
            last_sample_time: timestamp,
        }
    }

    pub fn new_with_remote(
        logger: slog::Logger,
        host: String,
        port: Option<u16>,
        timestamp: SystemTime,
    ) -> Result<Advance> {
        let store = remote_store::RemoteStore::new(host, port)?;

        Ok(Advance {
            logger,
            store: AdvanceStore::Remote(store),
            direction: store::Direction::Forward,
            last_sample: None,
            last_sample_time: timestamp,
        })
    }

    // Sets up last_sample and time if any, otherwise nothing is changed
    pub fn initialize(&mut self) {
        // Only initialize once
        assert!(self.last_sample == None);

        match self.get_next_sample(self.last_sample_time, store::Direction::Reverse) {
            Ok(Some((timestamp, dataframe))) => {
                self.last_sample = Some(dataframe.sample);
                self.last_sample_time = timestamp;
            }
            Ok(None) => (),
            Err(e) => {
                error!(self.logger, "{}", e.context("Failed to load from store"));
            }
        }
    }

    pub fn get_next_ts(&self) -> SystemTime {
        // timestamp for initial advance if initialize didn't setup last_sample
        if self.last_sample == None {
            return self.last_sample_time;
        }
        // store::read_next_sample gives >= or <= so we add or
        // subtract one to get the next sample
        match self.direction {
            store::Direction::Forward => self.last_sample_time + Duration::from_secs(1),
            store::Direction::Reverse => self.last_sample_time - Duration::from_secs(1),
        }
    }

    pub fn get_latest_sample(&mut self) -> Option<model::Model> {
        // Try to get sample that just updated, we minus 1 second here is because
        // advance will increase sample time by 1 second.
        self.last_sample_time = SystemTime::now() - Duration::from_secs(1);
        match self.advance(store::Direction::Forward) {
            Some(model) => return Some(model),
            None => (),
        }
        // Otherwise, we get the previous sample.
        self.last_sample_time = SystemTime::now();
        self.advance(store::Direction::Reverse)
    }

    pub fn jump_sample_forward(&mut self, duration: humantime::Duration) -> Option<model::Model> {
        self.last_sample_time += duration.into();
        match self.advance(store::Direction::Forward) {
            Some(model) => return Some(model),
            None => (),
        }

        // If sample is not available, get the latest sample
        self.last_sample_time = SystemTime::now();
        self.advance(store::Direction::Reverse)
    }

    pub fn jump_sample_backward(&mut self, duration: humantime::Duration) -> Option<model::Model> {
        self.last_sample_time -= duration.into();
        match self.advance(store::Direction::Reverse) {
            Some(model) => return Some(model),
            None => (),
        }

        // If sample is not available, get the earlist sample
        self.advance(store::Direction::Forward)
    }

    fn get_next_sample(
        &mut self,
        timestamp: SystemTime,
        direction: store::Direction,
    ) -> Result<Option<(SystemTime, DataFrame)>> {
        match &mut self.store {
            AdvanceStore::Local(dir) => {
                store::read_next_sample(dir, timestamp, direction, self.logger.clone())
            }
            AdvanceStore::Remote(ref mut store) => {
                store.get_frame(util::get_unix_timestamp(timestamp), direction)
            }
        }
    }

    fn handle_direction_switch(&mut self, direction: store::Direction) -> anyhow::Result<()> {
        if direction == self.direction {
            return Ok(());
        }

        self.direction = direction;
        let ts = self.get_next_ts();

        match self.get_next_sample(ts, self.direction) {
            Ok(Some((timestamp, dataframe))) => {
                self.last_sample = Some(dataframe.sample);
                self.last_sample_time = timestamp;
            }
            Ok(None) => {
                bail!(
                    "Failed to find sample with ts: {:?}, on direction switch to {:?}",
                    ts,
                    self.direction
                );
            }
            Err(e) => {
                return Err(e).context(format!(
                    "Failed to find sample with ts: {:?}, on direction switch to {:?}",
                    ts, self.direction
                ));
            }
        };

        Ok(())
    }

    pub fn advance(&mut self, direction: store::Direction) -> Option<model::Model> {
        if let Err(e) = self.handle_direction_switch(direction) {
            error!(self.logger, "Failed to switch iterator direction: {}", e);
            return None;
        }

        match self.get_next_sample(self.get_next_ts(), self.direction) {
            Ok(Some((timestamp, dataframe))) => {
                // `newer_sample` refers to the chronologically newer sample.
                // `last_sample` refers to the chronologically older sample.
                let newer_sample;
                let older_sample;

                match self.direction {
                    store::Direction::Forward => {
                        newer_sample = (dataframe.sample, timestamp);
                        older_sample = self.last_sample.take().map(|s| (s, self.last_sample_time));
                    }
                    store::Direction::Reverse => {
                        if self.last_sample.is_none() {
                            return None;
                        }
                        newer_sample = (
                            self.last_sample.take().expect("last_sample is none!"),
                            self.last_sample_time,
                        );
                        older_sample = Some((dataframe.sample, timestamp));
                    }
                };

                let data = Some(Model::new(
                    newer_sample.1,
                    &newer_sample.0,
                    older_sample.as_ref().map(|(s, i)| {
                        (
                            s,
                            newer_sample
                                .1
                                .duration_since(*i)
                                .expect("time went backwards"),
                        )
                    }),
                ));

                match self.direction {
                    store::Direction::Forward => {
                        self.last_sample = Some(newer_sample.0);
                        self.last_sample_time = newer_sample.1;
                    }
                    store::Direction::Reverse => {
                        let older = older_sample.expect("older_sample is none!");
                        self.last_sample = Some(older.0);
                        self.last_sample_time = older.1;
                    }
                };

                data
            }
            Ok(None) => None,
            Err(e) => {
                error!(self.logger, "{}", e.context("Failed to load from store"));
                None
            }
        }
    }
}
