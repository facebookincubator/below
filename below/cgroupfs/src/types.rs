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

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CpuStat {
    pub usage_usec: Option<u64>,
    pub user_usec: Option<u64>,
    pub system_usec: Option<u64>,
    pub nr_periods: Option<u64>,
    pub nr_throttled: Option<u64>,
    pub throttled_usec: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct IoStat {
    pub rbytes: Option<u64>,
    pub wbytes: Option<u64>,
    pub rios: Option<u64>,
    pub wios: Option<u64>,
    pub dbytes: Option<u64>,
    pub dios: Option<u64>,
    pub cost_usage: Option<u64>,
    pub cost_wait: Option<u64>,
    pub cost_indebt: Option<u64>,
    pub cost_indelay: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MemoryStat {
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub slab: Option<u64>,
    pub sock: Option<u64>,
    pub shmem: Option<u64>,
    pub file_mapped: Option<u64>,
    pub file_dirty: Option<u64>,
    pub file_writeback: Option<u64>,
    pub anon_thp: Option<u64>,
    pub inactive_anon: Option<u64>,
    pub active_anon: Option<u64>,
    pub inactive_file: Option<u64>,
    pub active_file: Option<u64>,
    pub unevictable: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    pub pgfault: Option<u64>,
    pub pgmajfault: Option<u64>,
    pub workingset_refault: Option<u64>,
    pub workingset_activate: Option<u64>,
    pub workingset_nodereclaim: Option<u64>,
    pub pgrefill: Option<u64>,
    pub pgscan: Option<u64>,
    pub pgsteal: Option<u64>,
    pub pgactivate: Option<u64>,
    pub pgdeactivate: Option<u64>,
    pub pglazyfree: Option<u64>,
    pub pglazyfreed: Option<u64>,
    pub thp_fault_alloc: Option<u64>,
    pub thp_collapse_alloc: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PressureMetrics {
    pub avg10: Option<f64>,
    pub avg60: Option<f64>,
    pub avg300: Option<f64>,
    pub total: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CpuPressure {
    pub some: PressureMetrics,
    pub full: Option<PressureMetrics>,
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

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MemoryEvents {
    pub low: Option<u64>,
    pub high: Option<u64>,
    pub max: Option<u64>,
    pub oom: Option<u64>,
    pub oom_kill: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CgroupStat {
    pub nr_descendants: Option<u32>,
    pub nr_dying_descendants: Option<u32>,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MemoryNumaStat {
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub pagetables: Option<u64>,
    pub shmem: Option<u64>,
    pub file_mapped: Option<u64>,
    pub file_dirty: Option<u64>,
    pub file_writeback: Option<u64>,
    pub swapcached: Option<u64>,
    pub anon_thp: Option<u64>,
    pub file_thp: Option<u64>,
    pub shmem_thp: Option<u64>,
    pub inactive_anon: Option<u64>,
    pub active_anon: Option<u64>,
    pub inactive_file: Option<u64>,
    pub active_file: Option<u64>,
    pub unevictable: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    pub workingset_refault_anon: Option<u64>,
    pub workingset_refault_file: Option<u64>,
    pub workingset_activate_anon: Option<u64>,
    pub workingset_activate_file: Option<u64>,
    pub workingset_restore_anon: Option<u64>,
    pub workingset_restore_file: Option<u64>,
    pub workingset_nodereclaim: Option<u64>,
}
