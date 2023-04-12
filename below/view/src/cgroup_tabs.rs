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

use cursive::utils::markup::StyledString;
use model::sort_queriables;
use model::CgroupModel;
use model::Queriable;
use model::SingleCgroupModel;
use model::SingleCgroupModelFieldId;

use crate::cgroup_view::CgroupState;
use crate::render::ViewItem;
use crate::stats_view::ColumnTitles;
use crate::stats_view::StateCommon;

/// Renders corresponding Fields From CgroupModel.
type CgroupViewItem = ViewItem<model::SingleCgroupModelFieldId>;

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
        model: &SingleCgroupModel,
        collapsed: bool,
        offset: Option<usize>,
        recreated: bool,
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

        if recreated {
            line = StyledString::styled(
                line.source(),
                cursive::theme::Color::Light(cursive::theme::BaseColor::Green),
            );
        }

        line
    }

    pub fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: std::iter::once(&*default_tabs::CGROUP_NAME_ITEM)
                .chain(self.view_items.iter())
                .map(|item| item.config.render_title())
                .collect(),
            pinned_titles: 1,
        }
    }

    fn output_cgroup(
        &self,
        cgroup: &CgroupModel,
        state: &CgroupState,
        filtered_set: &Option<HashSet<String>>,
        output: &mut Vec<(StyledString, String)>,
        offset: Option<usize>,
    ) {
        let mut cgroup_stack = vec![cgroup];
        while let Some(cgroup) = cgroup_stack.pop() {
            if let Some(set) = &filtered_set {
                if !set.contains(&cgroup.data.full_path) {
                    continue;
                }
            }

            let collapsed = state
                .collapsed_cgroups
                .borrow()
                .contains(&cgroup.data.full_path);
            let row = self.get_line(&cgroup.data, collapsed, offset, cgroup.recreate_flag);
            // Each row is (label, value), where label is visible and value is used
            // as identifier to correlate the row with its state in global data.
            if cgroup.recreate_flag {
                output.push((row, format!("[RECREATED] {}", &cgroup.data.full_path)));
            } else {
                output.push((row, cgroup.data.full_path.clone()));
            }

            if collapsed {
                continue;
            }

            let mut children = Vec::from_iter(&cgroup.children);
            if let Some(sort_order) = state.sort_order.as_ref() {
                sort_queriables(&mut children, &sort_order.to_owned().into(), state.reverse);
            }

            // Stop at next level (one below <root>)
            if state.collapse_all_top_level_cgroup {
                for child_cgroup in &children {
                    state
                        .collapsed_cgroups
                        .borrow_mut()
                        .insert(child_cgroup.data.full_path.clone());
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
        let filtered_set = if let Some((field_id, filter)) = &state.filter_info {
            Some(calculate_filtered_set(&state.get_model(), field_id, filter))
        } else {
            None
        };
        let mut rows = Vec::new();
        self.output_cgroup(&state.get_model(), state, &filtered_set, &mut rows, offset);
        rows
    }
}

/// Returns a set of full cgroup paths that should be filtered by the filter string.
///
/// Note that this algorithm recursively whitelists parents of cgroups that are
/// whitelisted. The reason for this is because cgroups are inherently tree-like
/// and displaying a lone cgroup without its ancestors doesn't make much sense.
pub fn calculate_filtered_set(
    cgroup: &CgroupModel,
    field_id: &SingleCgroupModelFieldId,
    filter: &str,
) -> HashSet<String> {
    fn field_val_matches_filter(
        cgroup: &CgroupModel,
        field_id: &SingleCgroupModelFieldId,
        filter: &str,
    ) -> bool {
        match cgroup.data.query(field_id) {
            None => false,
            Some(value) => value.to_string().contains(filter),
        }
    }

    // insert all descendents of cgroup into set
    fn insert_cgroup_and_descendents(set: &mut HashSet<String>, cgroup: &CgroupModel) {
        set.insert(cgroup.data.full_path.clone());
        for child in &cgroup.children {
            insert_cgroup_and_descendents(set, child)
        }
    }

    fn should_keep(
        cgroup: &CgroupModel,
        field_id: &SingleCgroupModelFieldId,
        filter: &str,
        set: &mut HashSet<String>,
    ) -> bool {
        let match_filter = field_val_matches_filter(cgroup, field_id, filter);
        if match_filter {
            insert_cgroup_and_descendents(set, cgroup);
            return match_filter;
        }

        let mut keep_cgroup = false;
        for child in &cgroup.children {
            // keep children that match filter and children of cgroups that match filter
            if should_keep(child, field_id, filter, set) {
                // keep parent cgroup if child isn't filtered out
                keep_cgroup = true;
            }
        }

        if keep_cgroup {
            set.insert(cgroup.data.full_path.clone());
        }
        keep_cgroup
    }
    let mut set = HashSet::new();
    should_keep(cgroup, field_id, filter, &mut set);
    set
}

pub mod default_tabs {
    use base_render::RenderConfigBuilder as Rc;
    use common::util::get_prefix;
    use model::CgroupCpuModelFieldId::NrPeriodsPerSec;
    use model::CgroupCpuModelFieldId::NrThrottledPerSec;
    use model::CgroupCpuModelFieldId::SystemPct;
    use model::CgroupCpuModelFieldId::ThrottledPct;
    use model::CgroupCpuModelFieldId::UsagePct;
    use model::CgroupCpuModelFieldId::UserPct;
    use model::CgroupIoModelFieldId::CostIndebtPct;
    use model::CgroupIoModelFieldId::CostIndelayPct;
    use model::CgroupIoModelFieldId::CostUsagePct;
    use model::CgroupIoModelFieldId::CostWaitPct;
    use model::CgroupIoModelFieldId::DbytesPerSec;
    use model::CgroupIoModelFieldId::DiosPerSec;
    use model::CgroupIoModelFieldId::RbytesPerSec;
    use model::CgroupIoModelFieldId::RiosPerSec;
    use model::CgroupIoModelFieldId::RwbytesPerSec;
    use model::CgroupIoModelFieldId::WbytesPerSec;
    use model::CgroupIoModelFieldId::WiosPerSec;
    use model::CgroupMemoryModelFieldId::ActiveAnon;
    use model::CgroupMemoryModelFieldId::ActiveFile;
    use model::CgroupMemoryModelFieldId::Anon;
    use model::CgroupMemoryModelFieldId::AnonThp;
    use model::CgroupMemoryModelFieldId::EventsHigh;
    use model::CgroupMemoryModelFieldId::EventsLow;
    use model::CgroupMemoryModelFieldId::EventsMax;
    use model::CgroupMemoryModelFieldId::EventsOom;
    use model::CgroupMemoryModelFieldId::EventsOomKill;
    use model::CgroupMemoryModelFieldId::File;
    use model::CgroupMemoryModelFieldId::FileDirty;
    use model::CgroupMemoryModelFieldId::FileMapped;
    use model::CgroupMemoryModelFieldId::FileWriteback;
    use model::CgroupMemoryModelFieldId::InactiveAnon;
    use model::CgroupMemoryModelFieldId::InactiveFile;
    use model::CgroupMemoryModelFieldId::KernelStack;
    use model::CgroupMemoryModelFieldId::Pgactivate;
    use model::CgroupMemoryModelFieldId::Pgdeactivate;
    use model::CgroupMemoryModelFieldId::Pgfault;
    use model::CgroupMemoryModelFieldId::Pglazyfree;
    use model::CgroupMemoryModelFieldId::Pglazyfreed;
    use model::CgroupMemoryModelFieldId::Pgmajfault;
    use model::CgroupMemoryModelFieldId::Pgrefill;
    use model::CgroupMemoryModelFieldId::Pgscan;
    use model::CgroupMemoryModelFieldId::Pgsteal;
    use model::CgroupMemoryModelFieldId::Shmem;
    use model::CgroupMemoryModelFieldId::Slab;
    use model::CgroupMemoryModelFieldId::SlabReclaimable;
    use model::CgroupMemoryModelFieldId::SlabUnreclaimable;
    use model::CgroupMemoryModelFieldId::Sock;
    use model::CgroupMemoryModelFieldId::Swap;
    use model::CgroupMemoryModelFieldId::ThpCollapseAlloc;
    use model::CgroupMemoryModelFieldId::ThpFaultAlloc;
    use model::CgroupMemoryModelFieldId::Total;
    use model::CgroupMemoryModelFieldId::Unevictable;
    use model::CgroupMemoryModelFieldId::WorkingsetActivateAnon;
    use model::CgroupMemoryModelFieldId::WorkingsetActivateFile;
    use model::CgroupMemoryModelFieldId::WorkingsetNodereclaim;
    use model::CgroupMemoryModelFieldId::WorkingsetRefaultAnon;
    use model::CgroupMemoryModelFieldId::WorkingsetRefaultFile;
    use model::CgroupMemoryModelFieldId::WorkingsetRestoreAnon;
    use model::CgroupMemoryModelFieldId::WorkingsetRestoreFile;
    use model::CgroupPressureModelFieldId::CpuFullPct;
    use model::CgroupPressureModelFieldId::CpuSomePct;
    use model::CgroupPressureModelFieldId::IoFullPct;
    use model::CgroupPressureModelFieldId::IoSomePct;
    use model::CgroupPressureModelFieldId::MemoryFullPct;
    use model::CgroupPressureModelFieldId::MemorySomePct;
    use model::CgroupPropertiesFieldId::CgroupControllers;
    use model::CgroupPropertiesFieldId::CpuMaxPeriodUsec;
    use model::CgroupPropertiesFieldId::CpuMaxUsec;
    use model::CgroupPropertiesFieldId::CpuWeight;
    use model::CgroupPropertiesFieldId::CpusetCpus;
    use model::CgroupPropertiesFieldId::CpusetCpusEffective;
    use model::CgroupPropertiesFieldId::MemoryHigh;
    use model::CgroupPropertiesFieldId::MemoryLow;
    use model::CgroupPropertiesFieldId::MemoryMax;
    use model::CgroupPropertiesFieldId::MemorySwapMax;
    use model::CgroupPropertiesFieldId::MemoryZswapMax;
    use model::CgroupStatModelFieldId::NrDescendants;
    use model::CgroupStatModelFieldId::NrDyingDescendants;
    use model::SingleCgroupModelFieldId::CgroupStat;
    use model::SingleCgroupModelFieldId::Cpu;
    use model::SingleCgroupModelFieldId::Io;
    use model::SingleCgroupModelFieldId::Mem;
    use model::SingleCgroupModelFieldId::Name;
    use model::SingleCgroupModelFieldId::Pressure;
    use model::SingleCgroupModelFieldId::Props;
    use once_cell::sync::Lazy;

    use super::*;

    pub static CGROUP_NAME_ITEM: Lazy<CgroupViewItem> = Lazy::new(|| {
        ViewItem::from_default(Name).update(Rc::new().indented_prefix(get_prefix(false)))
    });
    pub static CGROUP_NAME_ITEM_COLLAPSED: Lazy<CgroupViewItem> = Lazy::new(|| {
        ViewItem::from_default(Name).update(Rc::new().indented_prefix(get_prefix(true)))
    });

    pub static CGROUP_GENERAL_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Cpu(UsagePct)).update(Rc::new().title("CPU")),
            ViewItem::from_default(Mem(Total)),
            ViewItem::from_default(Pressure(CpuFullPct)),
            ViewItem::from_default(Pressure(MemoryFullPct)),
            ViewItem::from_default(Pressure(IoFullPct)),
            ViewItem::from_default(Io(RbytesPerSec)),
            ViewItem::from_default(Io(WbytesPerSec)),
            ViewItem::from_default(Io(RwbytesPerSec)),
            ViewItem::from_default(CgroupStat(NrDescendants)),
            ViewItem::from_default(CgroupStat(NrDyingDescendants)),
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
            ViewItem::from_default(Mem(WorkingsetRefaultAnon)),
            ViewItem::from_default(Mem(WorkingsetRefaultFile)),
            ViewItem::from_default(Mem(WorkingsetActivateAnon)),
            ViewItem::from_default(Mem(WorkingsetActivateFile)),
            ViewItem::from_default(Mem(WorkingsetRestoreAnon)),
            ViewItem::from_default(Mem(WorkingsetRestoreFile)),
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
            ViewItem::from_default(Io(CostUsagePct)),
            ViewItem::from_default(Io(CostWaitPct)),
            ViewItem::from_default(Io(CostIndebtPct)),
            ViewItem::from_default(Io(CostIndelayPct)),
        ])
    });

    pub static CGROUP_PRESSURE_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Pressure(CpuSomePct)),
            ViewItem::from_default(Pressure(CpuFullPct)),
            ViewItem::from_default(Pressure(MemorySomePct)),
            ViewItem::from_default(Pressure(MemoryFullPct)),
            ViewItem::from_default(Pressure(IoSomePct)),
            ViewItem::from_default(Pressure(IoFullPct)),
        ])
    });

    pub static CGROUP_PROPERTIES_TAB: Lazy<CgroupTab> = Lazy::new(|| {
        CgroupTab::new(vec![
            ViewItem::from_default(Props(MemoryLow)),
            ViewItem::from_default(Props(MemoryHigh)),
            ViewItem::from_default(Props(MemoryMax)),
            ViewItem::from_default(Props(MemorySwapMax)),
            ViewItem::from_default(Props(MemoryZswapMax)),
            ViewItem::from_default(Props(CpuMaxUsec)),
            ViewItem::from_default(Props(CpuMaxPeriodUsec)),
            ViewItem::from_default(Props(CpuWeight)),
            ViewItem::from_default(Props(CpusetCpus)),
            ViewItem::from_default(Props(CpusetCpusEffective)),
            ViewItem::from_default(Props(CgroupControllers)),
        ])
    });
}
