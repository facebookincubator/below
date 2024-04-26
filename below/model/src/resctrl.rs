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

#[::below_derive::queriable_derives]
pub struct ResctrlL3MonModel {
    pub llc_occupancy_bytes: Option<u64>,
    pub mbm_total_bytes_per_sec: Option<u64>,
    pub mbm_local_bytes_per_sec: Option<u64>,
}

/// Model for mon data
#[::below_derive::queriable_derives]
pub struct ResctrlMonModel {
    #[queriable(subquery)]
    pub total: ResctrlL3MonModel,
    #[queriable(subquery)]
    pub per_l3: BTreeMap<u64, ResctrlL3MonModel>,
}

/// Collection of all data about a single MON group
#[::below_derive::queriable_derives]
pub struct ResctrlMonGroupModel {
    pub name: String,
    pub full_path: String,
    #[queriable(subquery)]
    pub mon: ResctrlMonModel,
}

/// Collection of all data about a single CTRL_MON group and descendents
#[::below_derive::queriable_derives]
pub struct ResctrlCtrlMonGroupModel {
    pub name: String,
    pub full_path: String,
    pub cpuset: Option<resctrlfs::Cpuset>,
    pub mode: Option<resctrlfs::GroupMode>,
    #[queriable(subquery)]
    pub mon: ResctrlMonModel,
    #[queriable(subquery)]
    pub mon_groups: BTreeMap<String, ResctrlMonGroupModel>,
}

/// All data about the entire resctrl filesystem
#[::below_derive::queriable_derives]
pub struct ResctrlModel {
    pub cpuset: Option<resctrlfs::Cpuset>,
    pub mode: Option<resctrlfs::GroupMode>,
    #[queriable(subquery)]
    pub mon: Option<ResctrlMonModel>,
    #[queriable(subquery)]
    pub mon_groups: BTreeMap<String, ResctrlMonGroupModel>,
    #[queriable(subquery)]
    pub ctrl_mon_groups: BTreeMap<String, ResctrlCtrlMonGroupModel>,
}

fn rmid_bytes_to_opt(rmid_bytes: &Option<resctrlfs::RmidBytes>) -> Option<u64> {
    match rmid_bytes {
        Some(resctrlfs::RmidBytes::Bytes(b)) => Some(*b),
        Some(resctrlfs::RmidBytes::Unavailable) => None,
        None => None,
    }
}

impl std::ops::Add<&ResctrlL3MonModel> for ResctrlL3MonModel {
    type Output = Self;

    fn add(self, other: &Self) -> Self {
        Self {
            llc_occupancy_bytes: opt_add(self.llc_occupancy_bytes, other.llc_occupancy_bytes),
            mbm_total_bytes_per_sec: opt_add(
                self.mbm_total_bytes_per_sec,
                other.mbm_total_bytes_per_sec,
            ),
            mbm_local_bytes_per_sec: opt_add(
                self.mbm_local_bytes_per_sec,
                other.mbm_local_bytes_per_sec,
            ),
        }
    }
}

impl ResctrlModel {
    pub fn new(
        sample: &resctrlfs::ResctrlSample,
        last: Option<(&resctrlfs::ResctrlSample, Duration)>,
    ) -> ResctrlModel {
        ResctrlModel {
            cpuset: sample.cpuset.clone(),
            mode: sample.mode.clone(),
            mon: sample.mon_stat.as_ref().map(|mon_stat| {
                ResctrlMonModel::new(
                    mon_stat,
                    last.and_then(|(s, d)| s.mon_stat.as_ref().map(|v| (v, d))),
                )
            }),
            mon_groups: sample
                .mon_groups
                .as_ref()
                .unwrap_or(&Default::default())
                .iter()
                .map(|(name, stat)| {
                    (
                        name.clone(),
                        ResctrlMonGroupModel::new(
                            name.clone(),
                            name.to_string(),
                            stat,
                            last.and_then(|(s, d)| {
                                s.mon_groups
                                    .as_ref()
                                    .and_then(|v| v.get(name))
                                    .map(|v| (v, d))
                            }),
                        ),
                    )
                })
                .collect(),
            ctrl_mon_groups: sample
                .ctrl_mon_groups
                .as_ref()
                .unwrap_or(&Default::default())
                .iter()
                .map(|(name, stat)| {
                    (
                        name.clone(),
                        ResctrlCtrlMonGroupModel::new(
                            name.clone(),
                            name.to_string(),
                            stat,
                            last.and_then(|(s, d)| {
                                s.ctrl_mon_groups
                                    .as_ref()
                                    .and_then(|v| v.get(name))
                                    .map(|v| (v, d))
                            }),
                        ),
                    )
                })
                .collect(),
        }
    }
}

impl ResctrlCtrlMonGroupModel {
    pub fn new(
        name: String,
        full_path: String,
        sample: &resctrlfs::CtrlMonGroupStat,
        last: Option<(&resctrlfs::CtrlMonGroupStat, Duration)>,
    ) -> ResctrlCtrlMonGroupModel {
        let last_if_inode_matches =
            last.and_then(|(s, d)| match (s.inode_number, sample.inode_number) {
                (Some(prev_inode), Some(current_inode)) if prev_inode == current_inode => {
                    Some((s, d))
                }
                (None, None) => Some((s, d)),
                _ => None,
            });
        ResctrlCtrlMonGroupModel {
            name,
            full_path: full_path.clone(),
            cpuset: sample.cpuset.clone(),
            mode: sample.mode.clone(),
            mon: sample
                .mon_stat
                .as_ref()
                .map(|mon_stat| {
                    if let Some((last, delta)) = last_if_inode_matches {
                        if let Some(last_mon_stat) = last.mon_stat.as_ref() {
                            ResctrlMonModel::new(mon_stat, Some((last_mon_stat, delta)))
                        } else {
                            ResctrlMonModel::new(mon_stat, None)
                        }
                    } else {
                        ResctrlMonModel::new(mon_stat, None)
                    }
                })
                .unwrap_or_default(),
            mon_groups: sample
                .mon_groups
                .as_ref()
                .unwrap_or(&Default::default())
                .iter()
                .map(|(name, stat)| {
                    (
                        name.clone(),
                        ResctrlMonGroupModel::new(
                            name.clone(),
                            format!("{}/{}", full_path, name),
                            stat,
                            last_if_inode_matches.and_then(|(s, d)| {
                                s.mon_groups
                                    .as_ref()
                                    .and_then(|v| v.get(name))
                                    .map(|v| (v, d))
                            }),
                        ),
                    )
                })
                .collect(),
        }
    }
}

impl ResctrlMonGroupModel {
    pub fn new(
        name: String,
        full_path: String,
        sample: &resctrlfs::MonGroupStat,
        last: Option<(&resctrlfs::MonGroupStat, Duration)>,
    ) -> ResctrlMonGroupModel {
        let last_if_inode_matches =
            last.and_then(|(s, d)| match (s.inode_number, sample.inode_number) {
                (Some(prev_inode), Some(current_inode)) if prev_inode == current_inode => {
                    Some((s, d))
                }
                (None, None) => Some((s, d)),
                _ => None,
            });
        ResctrlMonGroupModel {
            name,
            full_path,
            mon: sample
                .mon_stat
                .as_ref()
                .map(|mon_stat| {
                    if let Some((last, delta)) = last_if_inode_matches {
                        if let Some(last_mon_stat) = last.mon_stat.as_ref() {
                            ResctrlMonModel::new(mon_stat, Some((last_mon_stat, delta)))
                        } else {
                            ResctrlMonModel::new(mon_stat, None)
                        }
                    } else {
                        ResctrlMonModel::new(mon_stat, None)
                    }
                })
                .unwrap_or_default(),
        }
    }
}

impl ResctrlMonModel {
    pub fn new(
        sample: &resctrlfs::MonStat,
        last: Option<(&resctrlfs::MonStat, Duration)>,
    ) -> ResctrlMonModel {
        let mut model = ResctrlMonModel::default();
        for (l3, end_l3_sample) in sample
            .l3_mon_stat
            .as_ref()
            .unwrap_or(&Default::default())
            .iter()
        {
            let last_l3 = if let Some((last_l3_sample, delta)) = &last {
                last_l3_sample
                    .l3_mon_stat
                    .as_ref()
                    .and_then(|v| v.get(l3))
                    .map(|v| (v, delta.clone()))
            } else {
                None
            };
            model
                .per_l3
                .insert(*l3, ResctrlL3MonModel::new(end_l3_sample, last_l3));
        }
        model.total = model
            .per_l3
            .values()
            .fold(ResctrlL3MonModel::default(), |acc, model| acc + model);
        model
    }
}

impl ResctrlL3MonModel {
    pub fn new(
        sample: &resctrlfs::L3MonStat,
        last: Option<(&resctrlfs::L3MonStat, Duration)>,
    ) -> ResctrlL3MonModel {
        if let Some((begin, delta)) = last {
            ResctrlL3MonModel {
                llc_occupancy_bytes: rmid_bytes_to_opt(&sample.llc_occupancy_bytes),
                mbm_total_bytes_per_sec: count_per_sec!(
                    rmid_bytes_to_opt(&begin.mbm_total_bytes),
                    rmid_bytes_to_opt(&sample.mbm_total_bytes),
                    delta,
                    u64
                ),
                mbm_local_bytes_per_sec: count_per_sec!(
                    rmid_bytes_to_opt(&begin.mbm_local_bytes),
                    rmid_bytes_to_opt(&sample.mbm_local_bytes),
                    delta,
                    u64
                ),
            }
        } else {
            ResctrlL3MonModel {
                llc_occupancy_bytes: rmid_bytes_to_opt(&sample.llc_occupancy_bytes),
                mbm_total_bytes_per_sec: None,
                mbm_local_bytes_per_sec: None,
            }
        }
    }
}
