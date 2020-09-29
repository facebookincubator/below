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

use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use cursive::utils::markup::StyledString;
use cursive::view::Identifiable;
use cursive::views::{NamedView, SelectView, ViewRef};
use cursive::Cursive;

use crate::cgroup_tabs::{
    CgroupCPU, CgroupGeneral, CgroupIO, CgroupMem, CgroupOrders, CgroupPressure, CgroupTab,
};
use crate::stats_view::{StateCommon, StatsView, ViewBridge};
use crate::ViewState;
use model::CgroupModel;

pub type ViewType = StatsView<CgroupView>;

#[derive(Default)]
pub struct CgroupState {
    // Rc::RefCell is necessaray here since we will need to change the collapsed_cgroups
    // when we traverse the cgroup tree recursively. And we can not pass the CgroupState as
    // mutable.
    pub collapsed_cgroups: Rc<RefCell<HashSet<String>>>,
    pub current_selected_cgroup: String,
    pub filter: Option<String>,
    pub sort_order: CgroupOrders,
    pub sort_tags: HashMap<String, Vec<CgroupOrders>>,
    pub reverse: bool,
    pub model: Rc<RefCell<CgroupModel>>,
    pub collapse_all_top_level_cgroup: bool,
}

impl StateCommon for CgroupState {
    type ModelType = CgroupModel;
    fn get_filter(&mut self) -> &mut Option<String> {
        &mut self.filter
    }

    fn set_sort_tag(&mut self, tab: &str, idx: usize, reverse: bool) -> bool {
        self.sort_order = self
            .sort_tags
            .get(tab)
            .unwrap_or_else(|| panic!("Fail to find tab: {}", tab))
            .get(idx)
            .expect("Out of title scope")
            .clone();
        self.reverse = reverse;
        self.sort_order != CgroupOrders::Keep
    }

    fn get_model(&self) -> Ref<Self::ModelType> {
        self.model.borrow()
    }

    fn get_model_mut(&self) -> RefMut<Self::ModelType> {
        self.model.borrow_mut()
    }

    fn new(model: Rc<RefCell<Self::ModelType>>) -> Self {
        let mut sort_tags = HashMap::new();
        sort_tags.insert("General".into(), CgroupGeneral::get_sort_tag_vec());
        sort_tags.insert("CPU".into(), CgroupCPU::get_sort_tag_vec());
        sort_tags.insert("Mem".into(), CgroupMem::get_sort_tag_vec());
        sort_tags.insert("I/O".into(), CgroupIO::get_sort_tag_vec());
        sort_tags.insert("Pressure".into(), CgroupPressure::get_sort_tag_vec());
        Self {
            collapsed_cgroups: Rc::new(RefCell::new(HashSet::new())),
            current_selected_cgroup: "<root>".into(),
            filter: None,
            sort_order: CgroupOrders::Keep,
            sort_tags,
            reverse: false,
            model,
            collapse_all_top_level_cgroup: false,
        }
    }
}

impl CgroupState {
    fn set_sort_order(&mut self, tag: CgroupOrders) {
        self.sort_order = tag;
    }

    fn set_reverse(&mut self, reverse: bool) {
        self.reverse = reverse;
    }

    fn toggle_collapse_root_flag(&mut self) {
        self.collapse_all_top_level_cgroup = !self.collapse_all_top_level_cgroup;
    }
}

pub enum CgroupView {
    General(CgroupGeneral),
    Cpu(CgroupCPU),
    Mem(CgroupMem),
    Io(CgroupIO),
    Pressure(CgroupPressure),
}

impl CgroupView {
    pub fn new(c: &mut Cursive) -> NamedView<ViewType> {
        let mut list = SelectView::new();
        list.set_on_submit(|c, cgroup: &String| {
            let mut view = CgroupView::get_cgroup_view(c);

            // Select root will collapse or uncollapse all top level cgroup
            if cgroup.is_empty() {
                view.state.borrow_mut().toggle_collapse_root_flag();
                view.state
                    .borrow_mut()
                    .collapsed_cgroups
                    .borrow_mut()
                    .clear();
                return view.refresh(c);
            } else if view.state.borrow().collapse_all_top_level_cgroup {
                view.state.borrow_mut().toggle_collapse_root_flag();
            }

            if view
                .state
                .borrow()
                .collapsed_cgroups
                .borrow()
                .contains(cgroup)
            {
                view.state
                    .borrow_mut()
                    .collapsed_cgroups
                    .borrow_mut()
                    .remove(cgroup);
            } else {
                view.state
                    .borrow_mut()
                    .collapsed_cgroups
                    .borrow_mut()
                    .insert(cgroup.to_string());
            }

            view.refresh(c);
        });

        list.set_on_select(|c, cgroup: &String| {
            c.call_on_name(Self::get_view_name(), |view: &mut ViewType| {
                view.state.borrow_mut().current_selected_cgroup = cgroup.clone();
                view.get_cmd_palette().set_info(cgroup);
            });
        });

        let tabs = vec![
            "General".into(),
            "CPU".into(),
            "Mem".into(),
            "I/O".into(),
            "Pressure".into(),
        ];
        let mut tabs_map: HashMap<String, CgroupView> = HashMap::new();
        tabs_map.insert("General".into(), CgroupView::General(Default::default()));
        tabs_map.insert("CPU".into(), CgroupView::Cpu(Default::default()));
        tabs_map.insert("Mem".into(), CgroupView::Mem(Default::default()));
        tabs_map.insert("I/O".into(), CgroupView::Io(Default::default()));
        tabs_map.insert("Pressure".into(), CgroupView::Pressure(Default::default()));
        let user_data = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive Object!");
        StatsView::new(
            "Cgroup",
            tabs,
            tabs_map,
            list,
            CgroupState::new(user_data.cgroup.clone()),
            user_data.event_controllers.clone(),
            user_data.cmd_controllers.clone(),
        )
        .feed_data(c)
        .on_event('C', |c| {
            let mut view = Self::get_cgroup_view(c);
            view.state
                .borrow_mut()
                .set_sort_order(CgroupOrders::UsagePct);
            view.state.borrow_mut().set_reverse(true);
            view.refresh(c)
        })
        .on_event('M', |c| {
            let mut view = Self::get_cgroup_view(c);
            view.state
                .borrow_mut()
                .set_sort_order(CgroupOrders::MemoryTotal);
            view.state.borrow_mut().set_reverse(true);
            view.refresh(c)
        })
        .on_event('D', |c| {
            let mut view = Self::get_cgroup_view(c);
            view.state
                .borrow_mut()
                .set_sort_order(CgroupOrders::RwTotal);
            view.state.borrow_mut().set_reverse(true);
            view.refresh(c)
        })
        .with_name(Self::get_view_name())
    }

    pub fn get_cgroup_view(c: &mut Cursive) -> ViewRef<ViewType> {
        ViewType::get_view(c)
    }

    pub fn refresh(c: &mut Cursive) {
        let mut view = Self::get_cgroup_view(c);
        view.refresh(c);
        let mut cmd_palette = view.get_cmd_palette();
        // We should not override alert on refresh. Only selection should override alert.
        match (
            cmd_palette.is_alerting(),
            view.get_detail_view().selection(),
        ) {
            (false, Some(selection)) => cmd_palette.set_info(selection.to_string()),
            _ => {}
        }
    }

    fn get_inner(&self) -> Box<dyn CgroupTab> {
        match self {
            Self::General(inner) => Box::new(inner.clone()),
            Self::Cpu(inner) => Box::new(inner.clone()),
            Self::Mem(inner) => Box::new(inner.clone()),
            Self::Io(inner) => Box::new(inner.clone()),
            Self::Pressure(inner) => Box::new(inner.clone()),
        }
    }
}

impl ViewBridge for CgroupView {
    type StateType = CgroupState;

    fn get_view_name() -> &'static str {
        "cgroup_view"
    }
    fn get_title_vec(&self) -> Vec<String> {
        let model: CgroupModel = Default::default();
        self.get_inner().get_title_vec(&model)
    }

    fn get_rows(
        &mut self,
        state: &Self::StateType,
        offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        self.get_inner().get_rows(state, offset)
    }
}
