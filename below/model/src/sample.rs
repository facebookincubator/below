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

use super::*;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub cgroup: CgroupSample,
    pub processes: procfs::PidMap,
    pub system: SystemSample,
    pub netstats: procfs::NetStat,
    pub gpus: Option<gpu_stats::GpuSample>,
    pub tc: tc::TcStats,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CgroupSample {
    pub cpu_stat: Option<cgroupfs::CpuStat>,
    pub io_stat: Option<BTreeMap<String, cgroupfs::IoStat>>,
    pub memory_current: Option<i64>,
    pub memory_stat: Option<cgroupfs::MemoryStat>,
    pub pressure: Option<cgroupfs::Pressure>,
    pub children: Option<BTreeMap<String, CgroupSample>>,
    pub memory_swap_current: Option<i64>,
    pub memory_zswap_current: Option<i64>,
    pub memory_min: Option<i64>,
    pub memory_low: Option<i64>,
    pub memory_high: Option<i64>,
    pub memory_max: Option<i64>,
    pub memory_swap_max: Option<i64>,
    pub memory_zswap_max: Option<i64>,
    pub memory_events: Option<cgroupfs::MemoryEvents>,
    pub inode_number: Option<i64>,
    pub cgroup_stat: Option<cgroupfs::CgroupStat>,
    pub memory_numa_stat: Option<BTreeMap<u32, cgroupfs::MemoryNumaStat>>,
    pub cpuset_cpus: Option<cgroupfs::Cpuset>,
    pub cpuset_cpus_effective: Option<cgroupfs::Cpuset>,
    pub cpuset_mems: Option<cgroupfs::MemNodes>,
    pub cpuset_mems_effective: Option<cgroupfs::MemNodes>,
    pub cpu_weight: Option<u32>,
    pub cpu_max: Option<cgroupfs::CpuMax>,
    pub cgroup_controllers: Option<BTreeSet<String>>,
    pub cgroup_subtree_control: Option<BTreeSet<String>>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SystemSample {
    pub stat: procfs::Stat,
    pub meminfo: procfs::MemInfo,
    pub vmstat: procfs::VmStat,
    pub hostname: String,
    pub disks: procfs::DiskMap,
    pub btrfs: Option<btrfs::BtrfsMap>,
    pub kernel_version: Option<String>,
    pub os_release: Option<String>,
}
