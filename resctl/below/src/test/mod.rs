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
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use slog::{self, error, o, Drain};
use tempdir::TempDir;

use crate::below_config::BelowConfig;
use crate::logutil;
use crate::model::{collect_sample, CgroupModel, CgroupPressureModel, Collector, CpuModel, Model};
use crate::store;
use crate::Advance;

use below_derive::BelowDecor;
use below_thrift::types::Sample;
use below_thrift::DataFrame;

mod test_config;
mod test_decorators;
mod test_dump;
mod test_general;

pub fn get_logger() -> slog::Logger {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
    slog::Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!())
}

fn get_dummy_exit_data() -> Arc<Mutex<procfs::PidMap>> {
    Arc::new(Mutex::new(procfs::PidMap::default()))
}
