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

use cursive::view::Identifiable;
use cursive::views::{NamedView, SelectView, ViewRef};
use cursive::Cursive;

use crate::model::SingleProcessModel;
use crate::view::process_tabs::{
    ProcessCPU, ProcessGeneral, ProcessIO, ProcessMem, ProcessOrders, ProcessTab,
};
use crate::view::stats_view::{StateCommon, StatsView, ViewBridge};
use crate::view::ViewState;

pub type ViewType = StatsView<ProcessView>;

pub struct ProcessState {
    pub filter: Option<String>,
    pub cgroup_filter: Option<String>,
    pub sort_order: ProcessOrders,
    pub sort_tags: HashMap<String, Vec<ProcessOrders>>,
    pub reverse: bool,
}

impl StateCommon for ProcessState {
    fn get_filter(&mut self) -> &mut Option<String> {
        &mut self.filter
    }
    fn set_sort_tag(&mut self, tab: &str, idx: usize, reverse: bool) {
        self.sort_order = self
            .sort_tags
            .get(tab)
            .unwrap_or_else(|| panic!("Fail to find tab: {}", tab))
            .get(idx)
            .expect("Out of title scope")
            .clone();
        self.reverse = reverse;
    }
}

impl ProcessState {
    fn set_sort_order(&mut self, tag: ProcessOrders) {
        self.sort_order = tag;
    }

    fn set_reverse(&mut self, reverse: bool) {
        self.reverse = reverse;
    }
}

impl Default for ProcessState {
    fn default() -> Self {
        let mut sort_tags = HashMap::new();
        sort_tags.insert("General".into(), ProcessGeneral::get_sort_tag_vec());
        sort_tags.insert("CPU".into(), ProcessCPU::get_sort_tag_vec());
        sort_tags.insert("Mem".into(), ProcessMem::get_sort_tag_vec());
        sort_tags.insert("I/O".into(), ProcessIO::get_sort_tag_vec());
        Self {
            cgroup_filter: None,
            filter: None,
            sort_order: ProcessOrders::Keep,
            sort_tags,
            reverse: false,
        }
    }
}

pub enum ProcessView {
    General(ProcessGeneral),
    Cpu(ProcessCPU),
    Mem(ProcessMem),
    Io(ProcessIO),
}

impl ProcessView {
    pub fn new(c: &mut Cursive) -> NamedView<ViewType> {
        let mut list = SelectView::<String>::new();
        list.set_on_select(|c, pid: &String| {
            let view_state = &c
                .user_data::<ViewState>()
                .expect("No data stored in Cursive object!");

            let cgroup = view_state
                .model
                .process
                .processes
                .get(&pid.parse::<i32>().unwrap_or(0))
                .map_or("?".to_string(), |spm| {
                    spm.cgroup.clone().unwrap_or_else(|| "?".to_string())
                });

            c.call_on_name(Self::get_view_name(), |view: &mut ViewType| {
                view.get_cmd_palette().set_info(cgroup);
            });
        });

        let tabs = vec!["General".into(), "CPU".into(), "Mem".into(), "I/O".into()];
        let mut tabs_map: HashMap<String, ProcessView> = HashMap::new();
        tabs_map.insert("General".into(), ProcessView::General(Default::default()));
        tabs_map.insert("CPU".into(), ProcessView::Cpu(Default::default()));
        tabs_map.insert("Mem".into(), ProcessView::Mem(Default::default()));
        tabs_map.insert("I/O".into(), ProcessView::Io(Default::default()));
        StatsView::new("process", tabs, tabs_map, list)
            .feed_data(c)
            .on_event('P', |c| {
                let mut view = Self::get_process_view(c);
                view.state.borrow_mut().set_sort_order(ProcessOrders::Pid);
                view.state.borrow_mut().set_reverse(false);
                view.refresh(c)
            })
            .on_event('C', |c| {
                let mut view = Self::get_process_view(c);
                view.state
                    .borrow_mut()
                    .set_sort_order(ProcessOrders::CpuTotal);
                view.state.borrow_mut().set_reverse(true);
                view.refresh(c)
            })
            .on_event('N', |c| {
                let mut view = Self::get_process_view(c);
                view.state.borrow_mut().set_sort_order(ProcessOrders::Comm);
                view.state.borrow_mut().set_reverse(false);
                view.refresh(c)
            })
            .on_event('M', |c| {
                let mut view = Self::get_process_view(c);
                view.state.borrow_mut().set_sort_order(ProcessOrders::Rss);
                view.state.borrow_mut().set_reverse(true);
                view.refresh(c)
            })
            .on_event('D', |c| {
                let mut view = Self::get_process_view(c);
                view.state
                    .borrow_mut()
                    .set_sort_order(ProcessOrders::IoTotal);
                view.state.borrow_mut().set_reverse(true);
                view.refresh(c)
            })
            .with_name(Self::get_view_name())
    }

    pub fn get_process_view(c: &mut Cursive) -> ViewRef<ViewType> {
        c.find_name::<ViewType>(Self::get_view_name())
            .expect("Fail to find process_view by its name")
    }

    pub fn refresh(c: &mut Cursive) {
        Self::get_process_view(c).refresh(c);
    }

    fn get_inner(&self) -> Box<dyn ProcessTab> {
        match self {
            Self::General(inner) => Box::new(inner.clone()),
            Self::Cpu(inner) => Box::new(inner.clone()),
            Self::Mem(inner) => Box::new(inner.clone()),
            Self::Io(inner) => Box::new(inner.clone()),
        }
    }
}

impl ViewBridge for ProcessView {
    type StateType = ProcessState;
    fn get_view_name() -> &'static str {
        "process_view"
    }
    fn get_title_vec(&self) -> Vec<String> {
        let model: SingleProcessModel = Default::default();
        self.get_inner().get_title_vec(&model)
    }

    fn get_rows(
        &mut self,
        view_state: &mut ViewState,
        state: &Self::StateType,
    ) -> Vec<(String, String)> {
        self.get_inner().get_rows(view_state, state)
    }
}
