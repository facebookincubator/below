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

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CpuStat {
    pub usage_usec: Option<i64>,
    pub user_usec: Option<i64>,
    pub system_usec: Option<i64>,
    pub nr_periods: Option<i64>,
    pub nr_throttled: Option<i64>,
    pub throttled_usec: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IoStat {
    pub rbytes: Option<i64>,
    pub wbytes: Option<i64>,
    pub rios: Option<i64>,
    pub wios: Option<i64>,
    pub dbytes: Option<i64>,
    pub dios: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MemoryStat {
    pub anon: Option<i64>,
    pub file: Option<i64>,
    pub kernel_stack: Option<i64>,
    pub slab: Option<i64>,
    pub sock: Option<i64>,
    pub shmem: Option<i64>,
    pub file_mapped: Option<i64>,
    pub file_dirty: Option<i64>,
    pub file_writeback: Option<i64>,
    pub anon_thp: Option<i64>,
    pub inactive_anon: Option<i64>,
    pub active_anon: Option<i64>,
    pub inactive_file: Option<i64>,
    pub active_file: Option<i64>,
    pub unevictable: Option<i64>,
    pub slab_reclaimable: Option<i64>,
    pub slab_unreclaimable: Option<i64>,
    pub pgfault: Option<i64>,
    pub pgmajfault: Option<i64>,
    pub workingset_refault: Option<i64>,
    pub workingset_activate: Option<i64>,
    pub workingset_nodereclaim: Option<i64>,
    pub pgrefill: Option<i64>,
    pub pgscan: Option<i64>,
    pub pgsteal: Option<i64>,
    pub pgactivate: Option<i64>,
    pub pgdeactivate: Option<i64>,
    pub pglazyfree: Option<i64>,
    pub pglazyfreed: Option<i64>,
    pub thp_fault_alloc: Option<i64>,
    pub thp_collapse_alloc: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PressureMetrics {
    pub avg10: Option<f64>,
    pub avg60: Option<f64>,
    pub avg300: Option<f64>,
    pub total: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CpuPressure {
    pub some: PressureMetrics,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IoPressure {
    pub some: PressureMetrics,
    pub full: PressureMetrics,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MemoryPressure {
    pub some: PressureMetrics,
    pub full: PressureMetrics,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pressure {
    pub cpu: CpuPressure,
    pub io: IoPressure,
    pub memory: MemoryPressure,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MemoryEvents {
    pub low: Option<i64>,
    pub high: Option<i64>,
    pub max: Option<i64>,
    pub oom: Option<i64>,
    pub oom_kill: Option<i64>,
}
