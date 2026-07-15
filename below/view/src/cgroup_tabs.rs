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

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;

use base_render::RenderConfig;
use cursive::utils::markup::StyledString;
use model::CgroupModel;
use model::CgroupModelFieldId;
use model::Queriable;
use model::SingleCgroupModel;
use model::SingleCgroupModelFieldId;
use model::SingleProcessModel;
use model::sort_queriables;

use crate::cgroup_view::CgroupState;
use crate::render::ViewItem;
use crate::stats_view::ColumnTitles;
use crate::stats_view::StateCommon;

/// Renders corresponding Fields From CgroupModel.
type CgroupViewItem = ViewItem<model::SingleCgroupModelFieldId>;

/// Renders corresponding Fields From ProcessModel.
type ProcessViewItem = ViewItem<model::SingleProcessModelFieldId>;

/// Columns shown for the processes of an expanded cgroup.
static EXPANDED_PROCS_VIEW_ITEMS: LazyLock<Vec<ProcessViewItem>> = LazyLock::new(|| {
    use model::ProcessCpuModelFieldId::UsagePct;
    use model::ProcessMemoryModelFieldId::RssBytes;
    use model::SingleProcessModelFieldId::Cmdline;
    use model::SingleProcessModelFieldId::Cpu;
    use model::SingleProcessModelFieldId::Mem;
    use model::SingleProcessModelFieldId::Pid;
    use model::SingleProcessModelFieldId::State;
    vec![
        ViewItem::from_default(Pid),
        ViewItem::from_default(State),
        ViewItem::from_default(Cpu(UsagePct)),
        ViewItem::from_default(Mem(RssBytes)),
        ViewItem::from_default(Cmdline),
    ]
});

/// Maps a cgroup sort order to the equivalent process field so that processes
/// of expanded cgroups follow the view's sort order where one exists.
fn cgroup_sort_to_process_sort(
    sort_order: &SingleCgroupModelFieldId,
) -> Option<model::SingleProcessModelFieldId> {
    use model::CgroupCpuModelFieldId as CgroupCpu;
    use model::CgroupIoModelFieldId as CgroupIo;
    use model::CgroupMemoryModelFieldId as CgroupMem;
    use model::ProcessCpuModelFieldId as ProcessCpu;
    use model::ProcessIoModelFieldId as ProcessIo;
    use model::ProcessMemoryModelFieldId as ProcessMem;
    use model::SingleCgroupModelFieldId as Cgroup;
    use model::SingleProcessModelFieldId as Process;

    match sort_order {
        Cgroup::Cpu(CgroupCpu::UsagePct) => Some(Process::Cpu(ProcessCpu::UsagePct)),
        Cgroup::Cpu(CgroupCpu::UserPct) => Some(Process::Cpu(ProcessCpu::UserPct)),
        Cgroup::Cpu(CgroupCpu::SystemPct) => Some(Process::Cpu(ProcessCpu::SystemPct)),
        Cgroup::Mem(CgroupMem::Total) => Some(Process::Mem(ProcessMem::RssBytes)),
        Cgroup::Io(CgroupIo::RbytesPerSec) => Some(Process::Io(ProcessIo::RbytesPerSec)),
        Cgroup::Io(CgroupIo::WbytesPerSec) => Some(Process::Io(ProcessIo::WbytesPerSec)),
        Cgroup::Io(CgroupIo::RwbytesPerSec) => Some(Process::Io(ProcessIo::RwbytesPerSec)),
        _ => None,
    }
}

/// A collection of CgroupViewItem.
#[derive(Clone)]
pub struct CgroupTab {
    pub view_items: Vec<CgroupViewItem>,
    cgroup_name: CgroupViewItem,
    cgroup_name_collapsed: CgroupViewItem,
    process_name: ProcessViewItem,
}

/// Defines how to iterate through the cgroup and generate get_rows function for ViewBridge
/// First ViewItem is always Name so it's not included in the view_items Vec.
impl CgroupTab {
    pub fn new(view_items: Vec<CgroupViewItem>, cgroup_name_config: &RenderConfig) -> Self {
        use base_render::RenderConfigBuilder as Rc;
        use common::util::get_prefix;
        let cgroup_name_item = ViewItem::from_default(SingleCgroupModelFieldId::Name);
        Self {
            view_items,
            cgroup_name: cgroup_name_item
                .clone()
                .update(cgroup_name_config.clone())
                .update(Rc::new().indented_prefix(get_prefix(false))),
            cgroup_name_collapsed: cgroup_name_item
                .update(cgroup_name_config.clone())
                .update(Rc::new().indented_prefix(get_prefix(true))),
            process_name: ViewItem::from_default(model::SingleProcessModelFieldId::Comm)
                .update(cgroup_name_config.clone())
                .update(Rc::new().indented_prefix(get_prefix(false))),
        }
    }

    fn get_line(
        &self,
        model: &SingleCgroupModel,
        collapsed: bool,
        offset: Option<usize>,
        recreated: bool,
    ) -> StyledString {
        let mut line = if collapsed {
            &self.cgroup_name_collapsed
        } else {
            &self.cgroup_name
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

    fn get_process_line(
        &self,
        model: &SingleProcessModel,
        depth: usize,
        offset: Option<usize>,
    ) -> StyledString {
        let mut line = self.process_name.render_indented_depth(model, depth);
        line.append_plain(" ");

        for item in EXPANDED_PROCS_VIEW_ITEMS.iter().skip(offset.unwrap_or(0)) {
            line.append(item.render(model));
            line.append_plain(" ");
        }

        // Color process rows to distinguish them from cgroup rows
        StyledString::styled(
            line.source(),
            cursive::theme::Color::Light(cursive::theme::BaseColor::Blue),
        )
    }

    pub fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: std::iter::once(&self.cgroup_name)
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
        cgroup_to_procs: &Option<HashMap<&str, Vec<&SingleProcessModel>>>,
        output: &mut Vec<(StyledString, String)>,
        offset: Option<usize>,
    ) {
        let mut cgroup_stack = vec![cgroup];
        while let Some(cgroup) = cgroup_stack.pop() {
            if let Some(set) = &filtered_set
                && !set.contains(&cgroup.data.full_path)
            {
                continue;
            }

            let collapsed = state
                .collapsed_cgroups
                .lock()
                .unwrap()
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

            // Show member processes of an expanded cgroup as child rows. They
            // carry the cgroup's full_path as key so that path-based handlers
            // act on the enclosing cgroup when a process row is selected.
            if let Some(procs_map) = cgroup_to_procs
                && let Some(procs) = procs_map.get(cgroup.data.full_path.as_str())
            {
                let depth = cgroup.data.depth as usize + 1;
                for process in procs {
                    output.push((
                        self.get_process_line(process, depth, offset),
                        cgroup.data.full_path.clone(),
                    ));
                }
            }

            let mut children = Vec::from_iter(&cgroup.children);
            if let Some(sort_order) = state.sort_order.as_ref() {
                // field_id that query its own data
                let field_id = CgroupModelFieldId::new(
                    Some(model::CgroupPath { path: vec![] }),
                    sort_order.clone(),
                );
                sort_queriables(&mut children, &field_id, state.reverse);
            }

            // Stop at next level (one below <root>)
            if state.collapse_all_top_level_cgroup {
                for child_cgroup in &children {
                    state
                        .collapsed_cgroups
                        .lock()
                        .unwrap()
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
        // Index processes of expanded cgroups by cgroup path. Only processes
        // directly in an expanded cgroup are included, not those in descendant
        // cgroups, so a process shows up only under its own cgroup.
        let process_model = state.process_model.lock().unwrap();
        let cgroup_to_procs: Option<HashMap<&str, Vec<&SingleProcessModel>>> =
            if state.expanded_procs_cgroups.is_empty() {
                None
            } else {
                let mut map: HashMap<&str, Vec<&SingleProcessModel>> = HashMap::new();
                for spm in process_model.processes.values() {
                    // Processes in the root cgroup report "/" while the root
                    // cgroup's full_path is ""
                    let cgroup = match spm.cgroup.as_deref() {
                        Some("/") => "",
                        Some(cgroup) => cgroup,
                        None => continue,
                    };
                    if state.expanded_procs_cgroups.contains(cgroup) {
                        map.entry(cgroup).or_default().push(spm);
                    }
                }
                // Follow the view's sort order when it maps to a process
                // field. Otherwise keep pid order from BTreeMap iteration.
                if let Some(process_sort) = state
                    .sort_order
                    .as_ref()
                    .and_then(cgroup_sort_to_process_sort)
                {
                    for procs in map.values_mut() {
                        sort_queriables(procs, &process_sort, state.reverse);
                    }
                }
                Some(map)
            };
        let mut rows = Vec::new();
        self.output_cgroup(
            &state.get_model(),
            state,
            &filtered_set,
            &cgroup_to_procs,
            &mut rows,
            offset,
        );
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
    use model::CgroupMemoryModelFieldId::EventsSockThrottled;
    use model::CgroupMemoryModelFieldId::File;
    use model::CgroupMemoryModelFieldId::FileDirty;
    use model::CgroupMemoryModelFieldId::FileMapped;
    use model::CgroupMemoryModelFieldId::FileThp;
    use model::CgroupMemoryModelFieldId::FileWriteback;
    use model::CgroupMemoryModelFieldId::Hugetlb;
    use model::CgroupMemoryModelFieldId::InactiveAnon;
    use model::CgroupMemoryModelFieldId::InactiveFile;
    use model::CgroupMemoryModelFieldId::Kernel;
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
    use model::CgroupMemoryModelFieldId::ShmemThp;
    use model::CgroupMemoryModelFieldId::Slab;
    use model::CgroupMemoryModelFieldId::SlabReclaimable;
    use model::CgroupMemoryModelFieldId::SlabUnreclaimable;
    use model::CgroupMemoryModelFieldId::Sock;
    use model::CgroupMemoryModelFieldId::Swap;
    use model::CgroupMemoryModelFieldId::Swapcached;
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
    use model::CgroupMemoryModelFieldId::Zswap;
    use model::CgroupMemoryModelFieldId::Zswapped;
    use model::CgroupNetworkModelFieldId::RxBytesPerSec;
    use model::CgroupNetworkModelFieldId::RxPacketsPerSec;
    use model::CgroupNetworkModelFieldId::TxBytesPerSec;
    use model::CgroupNetworkModelFieldId::TxPacketsPerSec;
    use model::CgroupPidsModelFieldId::TidsCurrent;
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
    use model::CgroupPropertiesFieldId::MemoryMin;
    use model::CgroupPropertiesFieldId::MemoryOomGroup;
    use model::CgroupPropertiesFieldId::MemorySwapMax;
    use model::CgroupPropertiesFieldId::MemoryZswapMax;
    use model::CgroupPropertiesFieldId::MemoryZswapWriteback;
    use model::CgroupPropertiesFieldId::TidsMax;
    use model::CgroupStatModelFieldId::NrDescendants;
    use model::CgroupStatModelFieldId::NrDyingDescendants;
    use model::SingleCgroupModelFieldId::CgroupStat;
    use model::SingleCgroupModelFieldId::Cpu;
    use model::SingleCgroupModelFieldId::Io;
    use model::SingleCgroupModelFieldId::Mem;
    use model::SingleCgroupModelFieldId::Net;
    use model::SingleCgroupModelFieldId::Pids;
    use model::SingleCgroupModelFieldId::Pressure;
    use model::SingleCgroupModelFieldId::Props;

    use super::*;

    pub fn get_general_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        vec![
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
            ViewItem::from_default(Pids(TidsCurrent)),
        ]
    }

    pub fn get_cpu_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        vec![
            ViewItem::from_default(Cpu(UsagePct)),
            ViewItem::from_default(Cpu(UserPct)),
            ViewItem::from_default(Cpu(SystemPct)),
            ViewItem::from_default(Cpu(NrPeriodsPerSec)),
            ViewItem::from_default(Cpu(NrThrottledPerSec)),
            ViewItem::from_default(Cpu(ThrottledPct)),
        ]
    }

    pub fn get_mem_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        vec![
            ViewItem::from_default(Mem(Total)),
            ViewItem::from_default(Mem(Swap)),
            ViewItem::from_default(Mem(Anon)),
            ViewItem::from_default(Mem(File)),
            ViewItem::from_default(Mem(Kernel)),
            ViewItem::from_default(Mem(KernelStack)),
            ViewItem::from_default(Mem(Slab)),
            ViewItem::from_default(Mem(Sock)),
            ViewItem::from_default(Mem(Shmem)),
            ViewItem::from_default(Mem(Zswap)),
            ViewItem::from_default(Mem(Zswapped)),
            ViewItem::from_default(Mem(FileMapped)),
            ViewItem::from_default(Mem(FileDirty)),
            ViewItem::from_default(Mem(FileWriteback)),
            ViewItem::from_default(Mem(Swapcached)),
            ViewItem::from_default(Mem(FileThp)),
            ViewItem::from_default(Mem(AnonThp)),
            ViewItem::from_default(Mem(ShmemThp)),
            ViewItem::from_default(Mem(InactiveAnon)),
            ViewItem::from_default(Mem(ActiveAnon)),
            ViewItem::from_default(Mem(InactiveFile)),
            ViewItem::from_default(Mem(ActiveFile)),
            ViewItem::from_default(Mem(Unevictable)),
            ViewItem::from_default(Mem(SlabReclaimable)),
            ViewItem::from_default(Mem(SlabUnreclaimable)),
            ViewItem::from_default(Mem(Hugetlb)),
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
            ViewItem::from_default(Mem(EventsSockThrottled)),
        ]
    }

    pub fn get_io_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        vec![
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
        ]
    }

    pub fn get_pressure_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        vec![
            ViewItem::from_default(Pressure(CpuSomePct)),
            ViewItem::from_default(Pressure(CpuFullPct)),
            ViewItem::from_default(Pressure(MemorySomePct)),
            ViewItem::from_default(Pressure(MemoryFullPct)),
            ViewItem::from_default(Pressure(IoSomePct)),
            ViewItem::from_default(Pressure(IoFullPct)),
        ]
    }

    pub fn get_properties_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        vec![
            ViewItem::from_default(Props(MemoryMin)),
            ViewItem::from_default(Props(MemoryLow)),
            ViewItem::from_default(Props(MemoryHigh)),
            ViewItem::from_default(Props(MemoryMax)),
            ViewItem::from_default(Props(MemorySwapMax)),
            ViewItem::from_default(Props(MemoryZswapMax)),
            ViewItem::from_default(Props(MemoryZswapWriteback)),
            ViewItem::from_default(Props(MemoryOomGroup)),
            ViewItem::from_default(Props(CpuMaxUsec)),
            ViewItem::from_default(Props(CpuMaxPeriodUsec)),
            ViewItem::from_default(Props(CpuWeight)),
            ViewItem::from_default(Props(CpusetCpus)),
            ViewItem::from_default(Props(CpusetCpusEffective)),
            ViewItem::from_default(Props(TidsMax)),
            ViewItem::from_default(Props(CgroupControllers)),
        ]
    }

    pub fn get_network_items() -> Vec<ViewItem<SingleCgroupModelFieldId>> {
        // Placeholder network items - will return "?" for anything without a value
        vec![
            ViewItem::from_default(Net(RxBytesPerSec)),
            ViewItem::from_default(Net(TxBytesPerSec)),
            ViewItem::from_default(Net(RxPacketsPerSec)),
            ViewItem::from_default(Net(TxPacketsPerSec)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::Mutex;

    use model::ProcessModel;
    use model::SingleProcessModel;

    use super::*;
    use crate::cgroup_view::CgroupState;

    fn cgroup_node(
        name: &str,
        full_path: &str,
        depth: u32,
        children: Vec<CgroupModel>,
    ) -> CgroupModel {
        CgroupModel {
            data: SingleCgroupModel {
                name: name.to_owned(),
                full_path: full_path.to_owned(),
                depth,
                ..Default::default()
            },
            children: children.into_iter().collect(),
            count: 1,
            recreate_flag: false,
        }
    }

    fn fake_cgroup_model() -> CgroupModel {
        cgroup_node(
            "<root>",
            "",
            0,
            vec![
                cgroup_node(
                    "child1",
                    "/child1",
                    1,
                    vec![cgroup_node("nested", "/child1/nested", 2, vec![])],
                ),
                cgroup_node("other", "/other", 1, vec![]),
            ],
        )
    }

    fn fake_process(pid: i32, comm: &str, cmdline: &str, cgroup: &str) -> SingleProcessModel {
        SingleProcessModel {
            pid: Some(pid),
            comm: Some(comm.to_owned()),
            cmdline: Some(cmdline.to_owned()),
            cgroup: Some(cgroup.to_owned()),
            ..Default::default()
        }
    }

    fn fake_process_model() -> ProcessModel {
        let mut processes = BTreeMap::new();
        processes.insert(
            100,
            fake_process(100, "proc_a", "/bin/proc_a --flag", "/child1"),
        );
        processes.insert(
            200,
            fake_process(200, "proc_b", "/bin/proc_b", "/child1/nested"),
        );
        processes.insert(300, fake_process(300, "proc_root", "/bin/proc_root", "/"));
        ProcessModel { processes }
    }

    fn fake_state() -> CgroupState {
        CgroupState {
            model: Arc::new(Mutex::new(fake_cgroup_model())),
            process_model: Arc::new(Mutex::new(fake_process_model())),
            ..Default::default()
        }
    }

    fn tab() -> CgroupTab {
        CgroupTab::new(default_tabs::get_general_items(), &RenderConfig::default())
    }

    fn keys(rows: &[(StyledString, String)]) -> Vec<&str> {
        rows.iter().map(|(_row, key)| key.as_str()).collect()
    }

    #[test]
    fn test_get_rows_no_expansion() {
        let rows = tab().get_rows(&fake_state(), None);
        assert_eq!(keys(&rows), vec!["", "/child1", "/child1/nested", "/other"]);
        assert!(
            rows.iter()
                .all(|(row, _key)| !row.source().contains("proc_")),
            "no process rows should render without expansion"
        );
    }

    #[test]
    fn test_get_rows_expanded_procs() {
        let mut state = fake_state();
        state.expanded_procs_cgroups.insert("/child1".to_owned());
        let rows = tab().get_rows(&state, None);

        // Process row shows up directly under its cgroup, keyed by the
        // cgroup's full path
        assert_eq!(
            keys(&rows),
            vec!["", "/child1", "/child1", "/child1/nested", "/other"]
        );
        let proc_row = &rows[2].0;
        let source = proc_row.source();
        assert!(source.contains("proc_a"), "got: {}", source);
        assert!(source.contains("100"), "got: {}", source);
        assert!(source.contains("--flag"), "got: {}", source);
        // Processes of the nested cgroup are not pulled into the parent
        assert!(
            !rows
                .iter()
                .any(|(row, _key)| row.source().contains("proc_b")),
            "expansion should only show the cgroup's own processes"
        );
        assert!(
            proc_row.spans().all(|span| span.attr.color.front
                == cursive::theme::ColorType::Color(cursive::theme::Color::Light(
                    cursive::theme::BaseColor::Blue
                ))),
            "process rows should render in blue"
        );
    }

    #[test]
    fn test_expanded_procs_collapse_wins() {
        let mut state = fake_state();
        state.expanded_procs_cgroups.insert("/child1".to_owned());
        state
            .collapsed_cgroups
            .lock()
            .unwrap()
            .insert("/child1".to_owned());
        let rows = tab().get_rows(&state, None);
        assert_eq!(keys(&rows), vec!["", "/child1", "/other"]);
        assert!(
            !rows
                .iter()
                .any(|(row, _key)| row.source().contains("proc_a")),
            "collapsed cgroups should not show process rows"
        );
    }

    #[test]
    fn test_expanded_procs_root_normalization() {
        let mut state = fake_state();
        // Root's full_path is "" while its processes report cgroup "/"
        state.expanded_procs_cgroups.insert("".to_owned());
        let rows = tab().get_rows(&state, None);
        assert_eq!(
            keys(&rows),
            vec!["", "", "/child1", "/child1/nested", "/other"]
        );
        assert!(rows[1].0.source().contains("proc_root"));
    }

    #[test]
    fn test_toggle_procs_for_selected_cgroup() {
        let mut state = fake_state();
        state.current_selected_cgroup = "/child1".to_owned();
        state.toggle_procs_for_selected_cgroup();
        assert!(state.expanded_procs_cgroups.contains("/child1"));
        state.toggle_procs_for_selected_cgroup();
        assert!(state.expanded_procs_cgroups.is_empty());

        // Keys that don't resolve to a live cgroup are ignored
        for key in ["[RECREATED] /child1", "/nonexistent"] {
            state.current_selected_cgroup = key.to_owned();
            state.toggle_procs_for_selected_cgroup();
            assert!(state.expanded_procs_cgroups.is_empty());
        }
    }

    #[test]
    fn test_expanded_procs_sort_mapping() {
        let mut state = fake_state();
        {
            let mut process_model = state.process_model.lock().unwrap();
            for (pid, usage_pct) in [(100, 5.0), (101, 90.0)] {
                let mut process =
                    fake_process(pid, &format!("proc_{}", pid), "/bin/proc", "/child1");
                process.cpu = Some(model::ProcessCpuModel {
                    usage_pct: Some(usage_pct),
                    ..Default::default()
                });
                process_model.processes.insert(pid, process);
            }
        }
        state.expanded_procs_cgroups.insert("/child1".to_owned());
        let pos = |rows: &[(StyledString, String)], needle: &str| {
            rows.iter()
                .position(|(row, _key)| row.source().contains(needle))
                .unwrap_or_else(|| panic!("row {} not found", needle))
        };

        // Without a sort order, process rows follow pid order
        let rows = tab().get_rows(&state, None);
        assert!(pos(&rows, "proc_100") < pos(&rows, "proc_101"));

        // A cgroup sort with a process equivalent applies to process rows
        state.sort_order = Some(SingleCgroupModelFieldId::Cpu(
            model::CgroupCpuModelFieldId::UsagePct,
        ));
        state.reverse = true;
        let rows = tab().get_rows(&state, None);
        assert!(pos(&rows, "proc_101") < pos(&rows, "proc_100"));

        // A sort with no process equivalent falls back to pid order
        state.sort_order = Some(SingleCgroupModelFieldId::Name);
        let rows = tab().get_rows(&state, None);
        assert!(pos(&rows, "proc_100") < pos(&rows, "proc_101"));
    }

    #[test]
    fn test_expanded_procs_with_filter() {
        let mut state = fake_state();
        state.expanded_procs_cgroups.insert("/child1".to_owned());
        state.filter_info = Some((SingleCgroupModelFieldId::Name, "other".to_owned()));
        let rows = tab().get_rows(&state, None);
        assert_eq!(keys(&rows), vec!["", "/other"]);
        assert!(
            !rows
                .iter()
                .any(|(row, _key)| row.source().contains("proc_a")),
            "filtered-out cgroups should not show process rows"
        );
    }
}
