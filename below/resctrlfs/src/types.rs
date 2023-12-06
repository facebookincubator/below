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
use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

/// Describes a set of CPUs of a resctrl group
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Cpuset {
    pub cpus: BTreeSet<u32>,
}

/// Represents the "mode" of a CTRL_MON group
#[derive(Clone, PartialEq, Eq, PartialOrd, Debug, Serialize, Deserialize)]
pub enum GroupMode {
    /// A shareable group allows sharing of its allocations
    Shareable,
    /// An exclusive group does not allow sharing of its allocations
    Exclusive,
}

/// Internal representation of the value read from monitoring data. This is a
/// wrapper around u64 that is used to handle the "Unavailable" state that can
/// be returned by resctrlfs.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RmidBytes {
    Bytes(u64),
    Unavailable,
}

/// Represents the stats for a single L3 within a group. There will be N of
/// these for each group, one for each `mon_XX` directory in `mon_data`.
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct L3MonStat {
    pub llc_occupancy_bytes: Option<RmidBytes>,
    pub mbm_total_bytes: Option<RmidBytes>,
    pub mbm_local_bytes: Option<RmidBytes>,
}

/// Represents the stats for a single group. This corresponds to information in
/// the `mon_data` directory of a CTRL_MON, MON or root group.
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MonStat {
    pub l3_mon_stat: Option<BTreeMap<u64, L3MonStat>>,
}

/// Information about a CTRL_MON group. See
/// https://www.kernel.org/doc/html/v6.4/arch/x86/resctrl.html
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CtrlMonGroupStat {
    pub inode_number: Option<u64>,
    pub mode: Option<GroupMode>,
    pub cpuset: Option<Cpuset>,
    pub mon_stat: Option<MonStat>,
    pub mon_groups: Option<BTreeMap<String, MonGroupStat>>,
}

/// Information about a MON group. See
/// https://www.kernel.org/doc/html/v6.4/arch/x86/resctrl.html
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MonGroupStat {
    pub inode_number: Option<u64>,
    pub cpuset: Option<Cpuset>,
    pub mon_stat: Option<MonStat>,
}

/// Represents the entire resctrlfs state including information for each child group.
/// See https://www.kernel.org/doc/html/v6.4/arch/x86/resctrl.html
#[derive(Default, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ResctrlSample {
    pub mode: Option<GroupMode>,
    pub cpuset: Option<Cpuset>,
    pub mon_stat: Option<MonStat>,
    pub ctrl_mon_groups: Option<BTreeMap<String, CtrlMonGroupStat>>,
    pub mon_groups: Option<BTreeMap<String, MonGroupStat>>,
}
