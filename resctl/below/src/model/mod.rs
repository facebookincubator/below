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

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, Context, Result};

use crate::util::{convert_bytes, fold_string};
use below_derive::BelowDecor;
use below_thrift::types::{CgroupSample, Sample, SystemSample};

#[macro_use]
pub mod collector;
pub mod cgroup;
pub mod process;
pub mod system;

pub use cgroup::*;
pub use collector::*;
pub use process::*;
pub use system::*;

pub struct Model {
    pub time_elapsed: Duration,
    pub timestamp: SystemTime,
    pub system: SystemModel,
    pub cgroup: CgroupModel,
    pub process: ProcessModel,
}

impl Model {
    /// Construct a `Model` from a Sample and optionally, the last
    /// `CumulativeSample` as well as the `Duration` since it was
    /// collected.
    pub fn new(timestamp: SystemTime, sample: &Sample, last: Option<(&Sample, Duration)>) -> Self {
        Model {
            time_elapsed: last.map(|(_, d)| d).unwrap_or_default(),
            timestamp,
            system: SystemModel::new(
                &sample.system,
                last.map(|(s, d)| (&s.system, d)),
                &sample.processes,
                last.map(|(s, d)| (&s.processes, d)),
            ),
            cgroup: CgroupModel::new(
                "<root>".to_string(),
                String::new(),
                0,
                &sample.cgroup,
                last.map(|(s, d)| (&s.cgroup, d)),
            ),
            process: ProcessModel::new(&sample.processes, last.map(|(s, d)| (&s.processes, d))),
        }
    }
}
