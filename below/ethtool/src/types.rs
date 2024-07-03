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

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EthtoolStats {
    pub nic: BTreeMap<String, NicStats>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NicStats {
    pub queue: Vec<QueueStats>,
    pub tx_timeout: Option<u64>,
    pub raw_stats: BTreeMap<String, u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct QueueStats {
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
    pub rx_count: Option<u64>,
    pub tx_count: Option<u64>,
    pub tx_missed_tx: Option<u64>,
    pub tx_unmask_interrupt: Option<u64>,
    pub raw_stats: BTreeMap<String, u64>,
}
