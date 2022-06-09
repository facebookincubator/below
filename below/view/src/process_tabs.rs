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

use crate::process_view::ProcessState;
use crate::render::ViewItem;
use crate::stats_view::{ColumnTitles, StateCommon};
use model::SingleProcessModel;

use cursive::utils::markup::StyledString;
use itertools::Itertools;

/// Renders corresponding Fields From ProcessModel.
type ProcessViewItem = ViewItem<model::SingleProcessModelFieldId>;

/// A collection of ProcessViewItem.
#[derive(Clone)]
pub struct ProcessTab {
    pub view_items: Vec<ProcessViewItem>,
}

// Defines how to iterate through the process stats and generate get_rows for ViewBridge
impl ProcessTab {
    fn new(view_items: Vec<ProcessViewItem>) -> Self {
        Self { view_items }
    }

    fn get_process_field_line(
        &self,
        model: &SingleProcessModel,
        offset: Option<usize>,
    ) -> StyledString {
        let mut line = StyledString::new();
        line.append(default_tabs::COMM_VIEW_ITEM.render(model));
        line.append_plain(" ");

        for item in std::iter::once(&*default_tabs::CGROUP_VIEW_ITEM)
            .chain(self.view_items.iter())
            .skip(offset.unwrap_or(0))
        {
            line.append(item.render(model));
            line.append_plain(" ");
        }

        line
    }

    pub fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: std::iter::once(&*default_tabs::COMM_VIEW_ITEM)
                .chain(std::iter::once(&*default_tabs::CGROUP_VIEW_ITEM))
                .chain(self.view_items.iter())
                .map(|item| item.config.render_title())
                .collect(),
            pinned_titles: 1,
        }
    }

    pub fn get_rows(
        &self,
        state: &ProcessState,
        offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        let unknown = "?".to_string();
        let unknown_pid: i32 = -1;
        let process_model = state.get_model();
        let mut processes: Vec<&SingleProcessModel> =
            process_model.processes.iter().map(|(_, spm)| spm).collect();

        if let Some(sort_order) = state.sort_order.as_ref() {
            model::sort_queriables(&mut processes, sort_order, state.reverse);
        }
        processes
            .iter()
            .filter(|spm| {
                // If we're in zoomed pids mode, only show processes belonging
                // to set of pids
                if let Some(f) = &state.pids_filter {
                    f.contains(&spm.pid.unwrap_or(unknown_pid))
                } else {
                    true
                }
            })
            .filter(|spm| {
                // If we're in zoomed cgroup mode, only show processes belonging to
                // our zoomed cgroup
                if let Some(f) = &state.cgroup_filter {
                    spm.cgroup.as_ref().unwrap_or(&unknown).starts_with(f)
                } else {
                    true
                }
            })
            .filter(|spm| {
                // If we're filtering by name, only show processes who pass the filter
                if let Some(f) = &state.filter {
                    spm.comm.as_ref().unwrap_or(&unknown).contains(f)
                } else {
                    true
                }
            })
            .copied()
            // Abuse batching() to conditionally fold iter
            .batching(|it| {
                if state.fold {
                    if let Some(first) = it.next() {
                        Some(it.fold(first.clone(), |acc, spm| {
                            SingleProcessModel::fold(&acc, spm)
                        }))
                    } else {
                        None
                    }
                } else {
                    it.next().cloned()
                }
            })
            .map(|spm| {
                (
                    self.get_process_field_line(&spm, offset),
                    spm.pid.unwrap_or(0).to_string(),
                )
            })
            .collect()
    }
}

pub mod default_tabs {
    use super::*;

    use model::ProcessCpuModelFieldId::{NumThreads, SystemPct, UsagePct, UserPct};
    use model::ProcessIoModelFieldId::{RbytesPerSec, RwbytesPerSec, WbytesPerSec};
    use model::ProcessMemoryModelFieldId::{
        Anon, File, HugeTlb, Lock, MajorfaultsPerSec, MinorfaultsPerSec, Pin, Pte, RssBytes, Shmem,
        Swap, VmSize,
    };
    use model::SingleProcessModelFieldId::{
        Cgroup, Cmdline, Comm, Cpu, Io, Mem, Pid, Ppid, State, UptimeSecs,
    };

    use once_cell::sync::Lazy;

    pub static COMM_VIEW_ITEM: Lazy<ProcessViewItem> = Lazy::new(|| ViewItem::from_default(Comm));
    pub static CGROUP_VIEW_ITEM: Lazy<ProcessViewItem> =
        Lazy::new(|| ViewItem::from_default(Cgroup));

    pub static PROCESS_GENERAL_TAB: Lazy<ProcessTab> = Lazy::new(|| {
        ProcessTab::new(vec![
            ViewItem::from_default(Pid),
            ViewItem::from_default(Ppid),
            ViewItem::from_default(State),
            ViewItem::from_default(Cpu(UsagePct)),
            ViewItem::from_default(Cpu(UserPct)),
            ViewItem::from_default(Cpu(SystemPct)),
            ViewItem::from_default(Mem(RssBytes)),
            ViewItem::from_default(Mem(MinorfaultsPerSec)),
            ViewItem::from_default(Mem(MajorfaultsPerSec)),
            ViewItem::from_default(Io(RbytesPerSec)),
            ViewItem::from_default(Io(WbytesPerSec)),
            ViewItem::from_default(UptimeSecs),
            ViewItem::from_default(Cpu(NumThreads)),
            ViewItem::from_default(Io(RwbytesPerSec)),
            ViewItem::from_default(Cmdline),
        ])
    });

    pub static PROCESS_CPU_TAB: Lazy<ProcessTab> = Lazy::new(|| {
        ProcessTab::new(vec![
            ViewItem::from_default(Cpu(UserPct)),
            ViewItem::from_default(Cpu(SystemPct)),
            ViewItem::from_default(Cpu(NumThreads)),
            ViewItem::from_default(Cpu(UsagePct)),
        ])
    });

    pub static PROCESS_MEM_TAB: Lazy<ProcessTab> = Lazy::new(|| {
        ProcessTab::new(vec![
            ViewItem::from_default(Mem(RssBytes)),
            ViewItem::from_default(Mem(VmSize)),
            ViewItem::from_default(Mem(Swap)),
            ViewItem::from_default(Mem(Anon)),
            ViewItem::from_default(Mem(File)),
            ViewItem::from_default(Mem(Shmem)),
            ViewItem::from_default(Mem(Pte)),
            ViewItem::from_default(Mem(Lock)),
            ViewItem::from_default(Mem(Pin)),
            ViewItem::from_default(Mem(HugeTlb)),
            ViewItem::from_default(Mem(MinorfaultsPerSec)),
            ViewItem::from_default(Mem(MajorfaultsPerSec)),
        ])
    });

    pub static PROCESS_IO_TAB: Lazy<ProcessTab> = Lazy::new(|| {
        ProcessTab::new(vec![
            ViewItem::from_default(Io(RbytesPerSec)),
            ViewItem::from_default(Io(WbytesPerSec)),
            ViewItem::from_default(Io(RwbytesPerSec)),
        ])
    });
}
