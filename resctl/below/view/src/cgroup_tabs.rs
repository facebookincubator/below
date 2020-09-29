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

use std::collections::HashSet;
use std::iter::FromIterator;

use crate::cgroup_view::CgroupState;
use crate::stats_view::StateCommon;

use below_derive::BelowDecor;
use common::util::{convert_bytes, fold_string, get_prefix};
use model::CgroupModel;

use cursive::utils::markup::StyledString;

// All available sorting tags
make_sort_order! (CgroupOrders {
    "cpu_usage": UsagePct,
    "cpu_user": UserPct,
    "cpu_sys": SysPct,
    "nr_periods": NrPeriodsPerSec,
    "nr_throttled": NrThrottledPerSec,
    "throttled": ThrottledPct,
    "mem_total": MemoryTotal,
    "swap": MemorySwap,
    "anon": Anon,
    "file": File,
    "kernel_stack": KernelStack,
    "slab": Slab,
    "sock": Sock,
    "shmem": Shmem,
    "file_mapped": FileMapped,
    "file_dirty": FileDirty,
    "file_writeback": FileWriteback,
    "anon_thp": AnonThp,
    "inactive_anon": InactiveAnon,
    "active_anon": ActiveAnon,
    "inactive_file": InactiveFile,
    "active_file": ActiveFile,
    "unevictable": Unevictable,
    "slab_reclaimable": SlabReclaimable,
    "slab_unreclaimable": SlabUnreclaimable,
    "pgfault": Pgfault,
    "pgmajfault": Pgmajfault,
    "workingset_refault": WorkingsetRefault,
    "workingset_activate": WorkingsetActivate,
    "workingset_node_reclaim": WorkingsetNodereclaim,
    "pgrefill": Pgrefill,
    "pgscan": Pgscan,
    "pgsteal": Pgsteal,
    "pgactivate": Pgactivate,
    "pgdeactivate": Pgdeactivate,
    "pglazyfree": Pglazyfree,
    "pglazyfreed": Pglazyfreed,
    "thp_fault_alloc": THPFaultAlloc,
    "thp_collapse_alloc": THPCollapseAlloc,
    "cpu_some": CpuSomePct,
    "mem_some": MemorySomePct,
    "mem_full": MemoryFullPct,
    "io_some": IoSomePct,
    "io_full": IoFullPct,
    "read_bps": RbytesPerSec,
    "write_bps": WbytesPerSec,
    "read_iops": RiosPerSec,
    "write_iops": WiosPerSec,
    "discard_bps": DbytesPerSec,
    "discard_iops": DiosPerSec,
    "rw_total": RwTotal,
});

impl Default for CgroupOrders {
    fn default() -> Self {
        CgroupOrders::Keep
    }
}

// Defines how to iterate through the cgroup and generate get_rows function for ViewBridge
pub trait CgroupTab {
    fn get_title_vec(&self, model: &CgroupModel) -> Vec<String>;
    fn depth(&mut self) -> &mut usize;
    fn collapse(&mut self) -> &mut bool;
    fn get_cgroup_field_line(&self, model: &CgroupModel, offset: Option<usize>) -> StyledString;
    fn sort_cgroup(&self, sort_order: CgroupOrders, cgroups: &mut Vec<&CgroupModel>, reverse: bool);
    fn output_cgroup(
        &mut self,
        cgroup: &CgroupModel,
        state: &CgroupState,
        filter_out_set: &Option<HashSet<String>>,
        output: &mut Vec<(StyledString, String)>,
        offset: Option<usize>,
    ) {
        if let Some(set) = &filter_out_set {
            if set.contains(&cgroup.full_path) {
                return;
            }
        }

        let collapsed = state.collapsed_cgroups.borrow().contains(&cgroup.full_path);
        *self.depth() = cgroup.depth as usize;
        *self.collapse() = collapsed;
        let row = self.get_cgroup_field_line(&cgroup, offset);
        // Each row is (label, value), where label is visible and value is used
        // as identifier to correlate the row with its state in global data.
        if cgroup.recreate_flag {
            output.push((row, format!("[RECREATED] {}", cgroup.full_path.clone())));
        } else {
            output.push((row, cgroup.full_path.clone()));
        }

        if collapsed {
            return;
        }

        let mut children = Vec::from_iter(&cgroup.children);

        // Here we map the sort order to an index (or for disk, do some custom sorting)
        self.sort_cgroup(state.sort_order, &mut children, state.reverse);

        // collapse_flag if set, we will insert all direct children to the collapsed_cgroups.
        // In that case, we will stop at next level.
        let collapse_flag =
            if state.collapsed_cgroups.borrow().is_empty() && state.collapse_all_top_level_cgroup {
                true
            } else {
                false
            };

        for child_cgroup in &children {
            if collapse_flag {
                state
                    .collapsed_cgroups
                    .borrow_mut()
                    .insert(child_cgroup.full_path.to_string());
            }
            self.output_cgroup(child_cgroup, state, filter_out_set, output, offset);
        }
    }

    fn get_rows(
        &mut self,
        state: &CgroupState,
        offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        let filter_out_set = if let Some(f) = &state.filter {
            Some(calculate_filter_out_set(&state.get_model(), &f))
        } else {
            None
        };

        let mut rows = Vec::new();
        self.output_cgroup(
            &state.get_model(),
            state,
            &filter_out_set,
            &mut rows,
            offset,
        );
        rows
    }
}

/// Returns a set of full cgroup paths that should be filtered out.
///
/// Note that this algorithm recursively whitelists parents of cgroups that are
/// whitelisted. The reason for this is because cgroups are inherently tree-like
/// and displaying a lone cgroup without its ancestors doesn't make much sense.
pub fn calculate_filter_out_set(cgroup: &CgroupModel, filter: &str) -> HashSet<String> {
    fn should_filter_out(cgroup: &CgroupModel, filter: &str, set: &mut HashSet<String>) -> bool {
        // No children
        if cgroup.count == 1 {
            if !cgroup.full_path.contains(filter) {
                set.insert(cgroup.full_path.clone());
                return true;
            }
            return false;
        }

        let mut filter_cgroup = true;
        for child in &cgroup.children {
            if should_filter_out(&child, &filter, set) {
                set.insert(child.full_path.clone());
            } else {
                // We found a child that's not filtered out. That means
                // we have to keep this (the parent cgroup) too.
                filter_cgroup = false;
            }
        }

        if filter_cgroup {
            set.insert(cgroup.full_path.clone());
        }

        filter_cgroup
    }

    let mut set = HashSet::new();
    should_filter_out(&cgroup, &filter, &mut set);
    set
}

// macro defines common implementation of CgroupTab.
macro_rules! impl_cgroup_tab {
    ($name:ident) => {
        impl CgroupTab for $name {
            fn get_title_vec(&self, model: &CgroupModel) -> Vec<String> {
                let mut res: Vec<String> = self
                    .get_title_pipe(&model)
                    .trim()
                    .split("|")
                    .map(|s| s.to_string())
                    .collect();
                res.pop();
                res
            }

            fn get_cgroup_field_line(
                &self,
                model: &CgroupModel,
                offset: Option<usize>,
            ) -> StyledString {
                let mut res = match offset {
                    Some(offset) => {
                        let mut field_iter = self.get_field_vec(&model).into_iter();
                        let mut res = StyledString::new();
                        if let Some(name) = field_iter.next() {
                            res.append(name);
                            res.append_plain(" ")
                        };

                        field_iter.skip(offset).for_each(|item| {
                            res.append(item);
                            res.append_plain(" ")
                        });
                        res
                    }
                    _ => self.get_field_line(&model),
                };

                if model.recreate_flag {
                    res = StyledString::styled(
                        res.source(),
                        cursive::theme::Color::Light(cursive::theme::BaseColor::Green),
                    );
                }

                res
            }

            fn sort_cgroup(
                &self,
                sort_order: CgroupOrders,
                cgroups: &mut Vec<&CgroupModel>,
                reverse: bool,
            ) {
                self.sort(sort_order, cgroups, reverse)
            }

            fn depth(&mut self) -> &mut usize {
                &mut self.depth
            }

            fn collapse(&mut self) -> &mut bool {
                &mut self.collapse
            }
        }
    };
}

#[derive(BelowDecor, Default, Clone)]
pub struct CgroupGeneral {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)",
        decorator = "fold_string(&$, 50 - self.depth.clone() * 3, 0, |c: char| !char::is_alphanumeric(c))"
    )]
    pub name: String,
    #[blink("CgroupModel$cpu?.get_usage_pct")]
    #[bttr(title = "CPU", sort_tag = "CgroupOrders::UsagePct")]
    pub cpu_usage_pct: Option<f64>,
    #[blink("CgroupModel$memory?.get_total")]
    #[bttr(sort_tag = "CgroupOrders::MemoryTotal")]
    pub memory_total: Option<u64>,
    #[blink("CgroupModel$pressure?.get_cpu_some_pct")]
    #[bttr(sort_tag = "CgroupOrders::CpuSomePct")]
    pub pressure_cpu_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_full_pct")]
    #[bttr(sort_tag = "CgroupOrders::MemoryFullPct")]
    pub pressure_memory_full_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_full_pct")]
    #[bttr(sort_tag = "CgroupOrders::IoFullPct")]
    pub pressure_io_full_pct: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::RbytesPerSec")]
    pub io_total_rbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::WbytesPerSec")]
    pub io_total_wbytes_per_sec: Option<f64>,
    #[bttr(
        title = "RW Total",
        width = 10,
        sort_tag = "CgroupOrders::RwTotal",
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    pub disk: Option<f64>,
    depth: usize,
    collapse: bool,
}

impl_cgroup_tab!(CgroupGeneral);

#[derive(BelowDecor, Default, Clone)]
pub struct CgroupCPU {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)",
        decorator = "fold_string(&$, 50 - self.depth * 3, 0, |c: char| !char::is_alphanumeric(c))"
    )]
    pub name: String,
    #[blink("CgroupModel$cpu?.get_usage_pct")]
    #[bttr(sort_tag = "CgroupOrders::UsagePct")]
    pub usage_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_user_pct")]
    #[bttr(sort_tag = "CgroupOrders::UserPct")]
    pub user_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_system_pct")]
    #[bttr(sort_tag = "CgroupOrders::SysPct")]
    pub system_pct: Option<f64>,
    #[blink("CgroupModel$cpu?.get_nr_periods_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::NrPeriodsPerSec")]
    pub nr_periods_per_sec: Option<f64>,
    #[blink("CgroupModel$cpu?.get_nr_throttled_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::NrThrottledPerSec")]
    pub nr_throttled_per_sec: Option<f64>,
    #[blink("CgroupModel$cpu?.get_throttled_pct")]
    #[bttr(sort_tag = "CgroupOrders::ThrottledPct")]
    pub throttled_pct: Option<f64>,
    depth: usize,
    collapse: bool,
}

impl_cgroup_tab!(CgroupCPU);

#[derive(BelowDecor, Default, Clone)]
pub struct CgroupMem {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)",
        decorator = "fold_string(&$, 50 - self.depth * 3, 0, |c: char| !char::is_alphanumeric(c))"
    )]
    pub name: String,
    #[blink("CgroupModel$memory?.get_total")]
    #[bttr(sort_tag = "CgroupOrders::MemoryTotal")]
    pub memory_total: Option<u64>,
    #[blink("CgroupModel$memory?.get_swap")]
    #[bttr(sort_tag = "CgroupOrders::MemorySwap")]
    pub memory_swap: Option<u64>,
    #[blink("CgroupModel$memory?.get_anon")]
    #[bttr(sort_tag = "CgroupOrders::Anon")]
    pub anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_file")]
    #[bttr(sort_tag = "CgroupOrders::File")]
    pub file: Option<u64>,
    #[blink("CgroupModel$memory?.get_kernel_stack")]
    #[bttr(sort_tag = "CgroupOrders::KernelStack")]
    pub kernel_stack: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab")]
    #[bttr(sort_tag = "CgroupOrders::Slab")]
    pub slab: Option<u64>,
    #[blink("CgroupModel$memory?.get_sock")]
    #[bttr(sort_tag = "CgroupOrders::Sock")]
    pub sock: Option<u64>,
    #[blink("CgroupModel$memory?.get_shmem")]
    #[bttr(sort_tag = "CgroupOrders::Shmem")]
    pub shmem: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_mapped")]
    #[bttr(sort_tag = "CgroupOrders::FileMapped")]
    pub file_mapped: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_dirty")]
    #[bttr(sort_tag = "CgroupOrders::FileDirty")]
    pub file_dirty: Option<u64>,
    #[blink("CgroupModel$memory?.get_file_writeback")]
    #[bttr(sort_tag = "CgroupOrders::FileWriteback")]
    pub file_writeback: Option<u64>,
    #[blink("CgroupModel$memory?.get_anon_thp")]
    #[bttr(sort_tag = "CgroupOrders::AnonThp")]
    pub anon_thp: Option<u64>,
    #[blink("CgroupModel$memory?.get_inactive_anon")]
    #[bttr(sort_tag = "CgroupOrders::InactiveAnon")]
    pub inactive_anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_active_anon")]
    #[bttr(sort_tag = "CgroupOrders::ActiveAnon")]
    pub active_anon: Option<u64>,
    #[blink("CgroupModel$memory?.get_inactive_file")]
    #[bttr(sort_tag = "CgroupOrders::InactiveFile")]
    pub inactive_file: Option<u64>,
    #[blink("CgroupModel$memory?.get_active_file")]
    #[bttr(sort_tag = "CgroupOrders::ActiveFile")]
    pub active_file: Option<u64>,
    #[blink("CgroupModel$memory?.get_unevictable")]
    #[bttr(sort_tag = "CgroupOrders::Unevictable")]
    pub unevictable: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab_reclaimable")]
    #[bttr(sort_tag = "CgroupOrders::SlabReclaimable")]
    pub slab_reclaimable: Option<u64>,
    #[blink("CgroupModel$memory?.get_slab_unreclaimable")]
    #[bttr(sort_tag = "CgroupOrders::SlabUnreclaimable")]
    pub slab_unreclaimable: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgfault")]
    #[bttr(sort_tag = "CgroupOrders::Pgfault")]
    pub pgfault: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgmajfault")]
    #[bttr(sort_tag = "CgroupOrders::Pgmajfault")]
    pub pgmajfault: Option<u64>,
    #[blink("CgroupModel$memory?.get_workingset_refault")]
    #[bttr(sort_tag = "CgroupOrders::WorkingsetRefault")]
    pub workingset_refault: Option<u64>,
    #[blink("CgroupModel$memory?.get_workingset_activate")]
    #[bttr(sort_tag = "CgroupOrders::WorkingsetActivate")]
    pub workingset_activate: Option<u64>,
    #[blink("CgroupModel$memory?.get_workingset_nodereclaim")]
    #[bttr(sort_tag = "CgroupOrders::WorkingsetNodereclaim")]
    pub workingset_nodereclaim: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgrefill")]
    #[bttr(sort_tag = "CgroupOrders::Pgrefill")]
    pub pgrefill: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgscan")]
    #[bttr(sort_tag = "CgroupOrders::Pgscan")]
    pub pgscan: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgsteal")]
    #[bttr(sort_tag = "CgroupOrders::Pgsteal")]
    pub pgsteal: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgactivate")]
    #[bttr(sort_tag = "CgroupOrders::Pgactivate")]
    pub pgactivate: Option<u64>,
    #[blink("CgroupModel$memory?.get_pgdeactivate")]
    #[bttr(sort_tag = "CgroupOrders::Pgdeactivate")]
    pub pgdeactivate: Option<u64>,
    #[blink("CgroupModel$memory?.get_pglazyfree")]
    #[bttr(sort_tag = "CgroupOrders::Pglazyfree")]
    pub pglazyfree: Option<u64>,
    #[blink("CgroupModel$memory?.get_pglazyfreed")]
    #[bttr(sort_tag = "CgroupOrders::Pglazyfreed")]
    pub pglazyfreed: Option<u64>,
    #[blink("CgroupModel$memory?.get_thp_fault_alloc")]
    #[bttr(sort_tag = "CgroupOrders::THPFaultAlloc")]
    pub thp_fault_alloc: Option<u64>,
    #[blink("CgroupModel$memory?.get_thp_collapse_alloc")]
    #[bttr(sort_tag = "CgroupOrders::THPCollapseAlloc")]
    pub thp_collapse_alloc: Option<u64>,
    depth: usize,
    collapse: bool,
}

impl_cgroup_tab!(CgroupMem);

#[derive(BelowDecor, Default, Clone)]
pub struct CgroupIO {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)",
        decorator = "fold_string(&$, 50 - self.depth * 3, 0, |c: char| !char::is_alphanumeric(c))"
    )]
    pub name: String,
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::RbytesPerSec")]
    pub rbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::WbytesPerSec")]
    pub wbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_dbytes_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::DbytesPerSec")]
    pub dbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rios_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::RiosPerSec")]
    pub rios_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wios_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::WiosPerSec")]
    pub wios_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_dios_per_sec")]
    #[bttr(sort_tag = "CgroupOrders::DiosPerSec")]
    pub dios_per_sec: Option<f64>,
    #[bttr(
        width = 16,
        unit = "/s",
        decorator = "convert_bytes($ as f64)",
        title = "Read/Write Total",
        sort_tag = "CgroupOrders::RwTotal"
    )]
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    pub rw_total: Option<f64>,
    depth: usize,
    collapse: bool,
}

impl_cgroup_tab!(CgroupIO);

#[derive(BelowDecor, Default, Clone)]
pub struct CgroupPressure {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)",
        decorator = "fold_string(&$, 50 - self.depth * 3, 0, |c: char| !char::is_alphanumeric(c))"
    )]
    pub name: String,
    #[blink("CgroupModel$pressure?.get_cpu_some_pct")]
    #[bttr(sort_tag = "CgroupOrders::CpuSomePct")]
    pub cpu_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_some_pct")]
    #[bttr(sort_tag = "CgroupOrders::MemorySomePct")]
    pub pressure_memory_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_full_pct")]
    #[bttr(sort_tag = "CgroupOrders::MemoryFullPct")]
    pub pressure_memory_full_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_some_pct")]
    #[bttr(sort_tag = "CgroupOrders::IoSomePct")]
    pub pressure_io_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_full_pct")]
    #[bttr(sort_tag = "CgroupOrders::IoFullPct")]
    pub pressure_io_full_pct: Option<f64>,
    pub rw_total: Option<f64>,
    depth: usize,
    collapse: bool,
}

impl_cgroup_tab!(CgroupPressure);
