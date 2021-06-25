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
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use cursive::utils::markup::StyledString;
use cursive::view::Identifiable;
use cursive::views::{NamedView, SelectView, ViewRef};
use cursive::Cursive;

use crate::core_tabs::*;
use crate::stats_view::{StateCommon, StatsView, ViewBridge};
use crate::ViewState;

use model::system::SystemModel;

pub type ViewType = StatsView<CoreView>;

#[derive(Default)]
pub struct CoreState {
    pub filter: Option<String>,
    pub collapsed_disk: HashSet<String>,
    pub model: Rc<RefCell<SystemModel>>,
}

pub enum CoreOrder {}

impl StateCommon for CoreState {
    type ModelType = SystemModel;
    type TagType = CoreOrder;
    fn get_filter(&mut self) -> &mut Option<String> {
        &mut self.filter
    }

    fn get_model(&self) -> Ref<Self::ModelType> {
        self.model.borrow()
    }

    fn get_model_mut(&self) -> RefMut<Self::ModelType> {
        self.model.borrow_mut()
    }

    fn new(model: Rc<RefCell<Self::ModelType>>) -> Self {
        Self {
            model,
            ..Default::default()
        }
    }
}

pub enum CoreView {
    Cpu(CoreCpu),
    Mem(CoreMem),
    Vm(CoreVm),
    Disk(CoreDisk),
}

impl CoreView {
    pub fn new(c: &mut Cursive) -> NamedView<ViewType> {
        let mut list = SelectView::<String>::new();
        list.set_on_submit(|c, idx: &String| {
            let mut view = CoreView::get_core_view(c);
            // We only care about disk not partition
            if view.get_tab_view().get_cur_selected() == "Disk" && idx.ends_with(".0") {
                if view.state.borrow_mut().collapsed_disk.contains(idx) {
                    view.state.borrow_mut().collapsed_disk.remove(idx);
                } else {
                    view.state
                        .borrow_mut()
                        .collapsed_disk
                        .insert(idx.to_string());
                }

                view.refresh(c);
            }
        });

        let tabs = vec!["CPU".into(), "Mem".into(), "Vm".into(), "Disk".into()];
        let mut tabs_map: HashMap<String, CoreView> = HashMap::new();
        tabs_map.insert("CPU".into(), CoreView::Cpu(Default::default()));
        tabs_map.insert("Mem".into(), CoreView::Mem(Default::default()));
        tabs_map.insert("Vm".into(), CoreView::Vm(Default::default()));
        tabs_map.insert("Disk".into(), CoreView::Disk(Default::default()));
        let user_data = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive Object!");
        StatsView::new(
            "core",
            tabs,
            tabs_map,
            list,
            CoreState::new(user_data.system.clone()),
            user_data.event_controllers.clone(),
            user_data.cmd_controllers.clone(),
        )
        .feed_data(c)
        .with_name(Self::get_view_name())
    }

    pub fn get_core_view(c: &mut Cursive) -> ViewRef<ViewType> {
        ViewType::get_view(c)
    }

    pub fn refresh(c: &mut Cursive) {
        Self::get_core_view(c).refresh(c);
    }

    fn get_inner(&self) -> Box<dyn CoreTab> {
        match self {
            Self::Cpu(inner) => Box::new(inner.clone()),
            Self::Mem(inner) => Box::new(inner.clone()),
            Self::Vm(inner) => Box::new(inner.clone()),
            Self::Disk(inner) => Box::new(inner.clone()),
        }
    }
}

impl ViewBridge for CoreView {
    type StateType = CoreState;
    fn get_view_name() -> &'static str {
        "core_view"
    }
    fn get_title_vec(&self) -> Vec<String> {
        self.get_inner().get_title_vec()
    }

    fn get_rows(
        &mut self,
        state: &Self::StateType,
        _offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        self.get_inner().get_rows(state)
    }
}
