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
use crate::render::ViewItem;
use crate::stats_view::StateCommon;

use model::{sort_queriables, CgroupModel};

use cursive::utils::markup::StyledString;

/// Renders corresponding Fields From CgroupModel.
type CgroupViewItem = ViewItem<model::CgroupModelFieldId>;

/// A collection of CgroupViewItem.
#[derive(Clone)]
pub struct CgroupTab {
    pub view_items: Vec<CgroupViewItem>,
}

/// Defines how to iterate through the cgroup and generate get_rows function for ViewBridge
/// First ViewItem is always Name so it's not included in the view_items Vec.
impl CgroupTab {
    fn new(view_items: Vec<CgroupViewItem>) -> Self {
        Self { view_items }
    }

    fn get_line(
        &self,
        model: &CgroupModel,
        collapsed: bool,
        offset: Option<usize>,
    ) -> StyledString {
        let mut line = if collapsed {
            &*default_tabs::CGROUP_NAME_ITEM_COLLAPSED
        } else {
            &*default_tabs::CGROUP_NAME_ITEM
        }
        .render_indented(model);
        line.append_plain(" ");

        for item in self.view_items.iter().skip(offset.unwrap_or(0)) {
            line.append(item.render(model));
            line.append_plain(" ");
        }

        if model.recreate_flag {
            line = StyledString::styled(
                line.source(),
                cursive::theme::Color::Light(cursive::theme::BaseColor::Green),
            );
        }

        line
    }

    pub fn get_title_vec(&self) -> Vec<String> {
        std::iter::once(&*default_tabs::CGROUP_NAME_ITEM)
            .chain(self.view_items.iter())
            .map(|item| item.config.render_title())
            .collect()
    }

    fn output_cgroup(
        &self,
        cgroup: &CgroupModel,
        state: &CgroupState,
        filter_out_set: &Option<HashSet<String>>,
        output: &mut Vec<(StyledString, String)>,
        offset: Option<usize>,
    ) {
        let mut cgroup_stack = vec![cgroup];
        while let Some(cgroup) = cgroup_stack.pop() {
            if let Some(set) = &filter_out_set {
                if set.contains(&cgroup.full_path) {
                    continue;
                }
            }

            let collapsed = state.collapsed_cgroups.borrow().contains(&cgroup.full_path);
            let row = self.get_line(&cgroup, collapsed, offset);
            // Each row is (label, value), where label is visible and value is used
            // as identifier to correlate the row with its state in global data.
            if cgroup.recreate_flag {
                output.push((row, format!("[RECREATED] {}", &cgroup.full_path)));
            } else {
                output.push((row, cgroup.full_path.clone()));
            }

            if collapsed {
                continue;
            }

            let mut children = Vec::from_iter(&cgroup.children);
            if let Some(sort_order) = state.sort_order.as_ref() {
                sort_queriables(&mut children, sort_order, state.reverse);
            }

            // Stop at next level (one below <root>)
            if state.collapse_all_top_level_cgroup {
                for child_cgroup in &children {
                    state
                        .collapsed_cgroups
                        .borrow_mut()
                        .insert(child_cgroup.full_path.clone());
                }
            }
            // Push children in reverse order so the first one will be pop first
            while let Some(child) = children.pop() {
                cgroup_stack.push(child);
            }
        }
    }

    pub fn get_rows(
        &self,
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

pub mod default_tabs {
    use super::*;

    use base_render::render_config as rc;
    use common::util::get_prefix;
    use model::CgroupCpuModelFieldId::{
        NrPeriodsPerSec, NrThrottledPerSec, SystemPct, ThrottledPct, UsagePct, UserPct,
    };
    use model::CgroupIoModelFieldId::{
        DbytesPerSec, DiosPerSec, RbytesPerSec, RiosPerSec, RwbytesPerSec, WbytesPerSec, WiosPerSec,
    };
    use model::CgroupMemoryModelFieldId::{
        ActiveAnon, ActiveFile, Anon, AnonThp, EventsHigh, EventsLow, EventsMax, EventsOom,
        EventsOomKill, File, FileDirty, FileMapped, FileWriteback, InactiveAnon, InactiveFile,
        KernelStack, Pgactivate, Pgdeactivate, Pgfault, Pglazyfree, Pglazyfreed, Pgmajfault,
        Pgrefill, Pgscan, Pgsteal, Shmem, Slab, SlabReclaimable, SlabUnreclaimable, Sock, Swap,
        ThpCollapseAlloc, ThpFaultAlloc, Total, Unevictable, WorkingsetActivate,
        WorkingsetNodereclaim, WorkingsetRefault,
    };
    use model::CgroupModelFieldId::{Cpu, Io, Mem, Name, Pressure};
    use model::CgroupPressureModelFieldId::{
        CpuSomePct, IoFullPct, IoSomePct, MemoryFullPct, MemorySomePct,
    };

    use once_cell::sync::Lazy;

    pub static CGROUP_NAME_ITEM: Lazy<CgroupViewItem> =
        Lazy::new(|| ViewItem::from_default(Name).update(rc!(indented_prefix(get_prefix(false)))));
    pub static CGROUP_NAME_ITEM_COLLAPSED: Lazy<CgroupViewItem> =
        Lazy::new(|| ViewItem::from_default(Name).update(rc!(indented_prefix(get_prefix(true)))));

    pub static CGROUP_GENERAL_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Cpu(UsagePct)).update(rc!(title("CPU"))),
            ViewItem::from_default(Mem(Total)),
            ViewItem::from_default(Pressure(CpuSomePct)),
            ViewItem::from_default(Pressure(MemoryFullPct)),
            ViewItem::from_default(Pressure(IoFullPct)),
            ViewItem::from_default(Io(RbytesPerSec)),
            ViewItem::from_default(Io(WbytesPerSec)),
            ViewItem::from_default(Io(RwbytesPerSec)),
        ])
    });

    pub static CGROUP_CPU_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Cpu(UsagePct)),
            ViewItem::from_default(Cpu(UserPct)),
            ViewItem::from_default(Cpu(SystemPct)),
            ViewItem::from_default(Cpu(NrPeriodsPerSec)),
            ViewItem::from_default(Cpu(NrThrottledPerSec)),
            ViewItem::from_default(Cpu(ThrottledPct)),
        ])
    });

    pub static CGROUP_MEM_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Mem(Total)),
            ViewItem::from_default(Mem(Swap)),
            ViewItem::from_default(Mem(Anon)),
            ViewItem::from_default(Mem(File)),
            ViewItem::from_default(Mem(KernelStack)),
            ViewItem::from_default(Mem(Slab)),
            ViewItem::from_default(Mem(Sock)),
            ViewItem::from_default(Mem(Shmem)),
            ViewItem::from_default(Mem(FileMapped)),
            ViewItem::from_default(Mem(FileDirty)),
            ViewItem::from_default(Mem(FileWriteback)),
            ViewItem::from_default(Mem(AnonThp)),
            ViewItem::from_default(Mem(InactiveAnon)),
            ViewItem::from_default(Mem(ActiveAnon)),
            ViewItem::from_default(Mem(InactiveFile)),
            ViewItem::from_default(Mem(ActiveFile)),
            ViewItem::from_default(Mem(Unevictable)),
            ViewItem::from_default(Mem(SlabReclaimable)),
            ViewItem::from_default(Mem(SlabUnreclaimable)),
            ViewItem::from_default(Mem(Pgfault)),
            ViewItem::from_default(Mem(Pgmajfault)),
            ViewItem::from_default(Mem(WorkingsetRefault)),
            ViewItem::from_default(Mem(WorkingsetActivate)),
            ViewItem::from_default(Mem(WorkingsetNodereclaim)),
            ViewItem::from_default(Mem(Pgrefill)),
            ViewItem::from_default(Mem(Pgscan)),
            ViewItem::from_default(Mem(Pgsteal)),
            ViewItem::from_default(Mem(Pgactivate)),
            ViewItem::from_default(Mem(Pgdeactivate)),
            ViewItem::from_default(Mem(Pglazyfree)),
            ViewItem::from_default(Mem(Pglazyfreed)),
            ViewItem::from_default(Mem(ThpFaultAlloc)),
            ViewItem::from_default(Mem(ThpCollapseAlloc)),
            ViewItem::from_default(Mem(EventsLow)),
            ViewItem::from_default(Mem(EventsHigh)),
            ViewItem::from_default(Mem(EventsMax)),
            ViewItem::from_default(Mem(EventsOom)),
            ViewItem::from_default(Mem(EventsOomKill)),
        ])
    });

    pub static CGROUP_IO_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Io(RbytesPerSec)),
            ViewItem::from_default(Io(WbytesPerSec)),
            ViewItem::from_default(Io(DbytesPerSec)),
            ViewItem::from_default(Io(RiosPerSec)),
            ViewItem::from_default(Io(WiosPerSec)),
            ViewItem::from_default(Io(DiosPerSec)),
            ViewItem::from_default(Io(RwbytesPerSec)),
        ])
    });

    pub static CGROUP_PRESSURE_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Pressure(CpuSomePct)),
            ViewItem::from_default(Pressure(MemorySomePct)),
            ViewItem::from_default(Pressure(MemoryFullPct)),
            ViewItem::from_default(Pressure(IoSomePct)),
            ViewItem::from_default(Pressure(IoFullPct)),
        ])
    });
}
