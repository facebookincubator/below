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

/// This file contains various helpers
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Convert `timestamp` from `SystemTime` to seconds since epoch
pub fn get_unix_timestamp(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs()
}

/// Convert `timestamp` from seconds since epoch to `SystemTime`
pub fn get_system_time(timestamp: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(timestamp)
}
