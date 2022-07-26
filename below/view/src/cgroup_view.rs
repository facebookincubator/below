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

use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use cursive::utils::markup::StyledString;
use cursive::view::Nameable;
use cursive::views::NamedView;
use cursive::views::SelectView;
use cursive::views::ViewRef;
use cursive::Cursive;

use crate::cgroup_tabs::default_tabs::CGROUP_CPU_TAB;
use crate::cgroup_tabs::default_tabs::CGROUP_GENERAL_TAB;
use crate::cgroup_tabs::default_tabs::CGROUP_IO_TAB;
use crate::cgroup_tabs::default_tabs::CGROUP_MEM_TAB;
use crate::cgroup_tabs::default_tabs::CGROUP_PRESSURE_TAB;
use crate::cgroup_tabs::CgroupTab;
use crate::stats_view::ColumnTitles;
use crate::stats_view::StateCommon;
use crate::stats_view::StatsView;
use crate::stats_view::ViewBridge;
use crate::ViewState;
use model::CgroupCpuModelFieldId;
use model::CgroupIoModelFieldId;
use model::CgroupMemoryModelFieldId;
use model::CgroupModel;
use model::SingleCgroupModelFieldId;

pub type ViewType = StatsView<CgroupView>;

#[derive(Default)]
pub struct CgroupState {
    // Rc::RefCell is necessaray here since we will need to change the collapsed_cgroups
    // when we traverse the cgroup tree recursively. And we can not pass the CgroupState as
    // mutable.
    pub collapsed_cgroups: Rc<RefCell<HashSet<String>>>,
    pub current_selected_cgroup: String,
    // cgroup row to move focus on. If set, on next refresh, selector will be
    // moved to the cgroup
    pub cgroup_to_focus: Option<String>,
    pub filter: Option<String>,
    pub sort_order: Option<SingleCgroupModelFieldId>,
    pub sort_tags: HashMap<String, &'static CgroupTab>,
    pub reverse: bool,
    pub model: Rc<RefCell<CgroupModel>>,
    pub collapse_all_top_level_cgroup: bool,
}

impl StateCommon for CgroupState {
    type ModelType = CgroupModel;
    type TagType = SingleCgroupModelFieldId;
    type KeyType = String;

    fn get_filter(&mut self) -> &mut Option<String> {
        &mut self.filter
    }

    fn set_sort_tag(&mut self, sort_order: Self::TagType, reverse: &mut bool) -> bool {
        let sort_order = Some(sort_order);
        if self.sort_order == sort_order {
            *reverse = !*reverse;
        } else {
            *reverse = true;
            self.sort_order = sort_order;
        }
        self.reverse = *reverse;
        true
    }

    fn set_sort_tag_from_tab_idx(&mut self, tab: &str, idx: usize, reverse: &mut bool) -> bool {
        let sort_order = match idx {
            0 => Self::TagType::Name,
            _ => self
                .sort_tags
                .get(tab)
                .unwrap_or_else(|| panic!("Fail to find tab: {}", tab))
                .view_items
                .get(idx - 1)
                .expect("Out of title scope")
                .field_id
                .to_owned(),
        };

        self.set_sort_tag(sort_order, reverse)
    }

    fn set_sort_string(&mut self, selection: &str, reverse: &mut bool) -> bool {
        use std::str::FromStr;
        match Self::TagType::from_str(selection) {
            Ok(field_id) => self.set_sort_tag(field_id, reverse),
            Err(_) => false,
        }
    }

    fn get_model(&self) -> Ref<Self::ModelType> {
        self.model.borrow()
    }

    fn get_model_mut(&self) -> RefMut<Self::ModelType> {
        self.model.borrow_mut()
    }

    fn new(model: Rc<RefCell<Self::ModelType>>) -> Self {
        let mut sort_tags = HashMap::new();
        sort_tags.insert("General".into(), &*CGROUP_GENERAL_TAB);
        sort_tags.insert("CPU".into(), &*CGROUP_CPU_TAB);
        sort_tags.insert("Mem".into(), &*CGROUP_MEM_TAB);
        sort_tags.insert("I/O".into(), &*CGROUP_IO_TAB);
        sort_tags.insert("Pressure".into(), &*CGROUP_PRESSURE_TAB);
        Self {
            collapsed_cgroups: Rc::new(RefCell::new(HashSet::new())),
            current_selected_cgroup: "<root>".into(),
            cgroup_to_focus: None,
            filter: None,
            sort_order: None,
            sort_tags,
            reverse: false,
            model,
            collapse_all_top_level_cgroup: false,
        }
    }
}

impl CgroupState {
    fn set_sort_order(&mut self, tag: SingleCgroupModelFieldId) {
        self.sort_order = Some(tag);
    }

    fn set_reverse(&mut self, reverse: bool) {
        self.reverse = reverse;
    }

    fn toggle_collapse_root_flag(&mut self) {
        self.collapse_all_top_level_cgroup = !self.collapse_all_top_level_cgroup;
    }

    // Recursively fold open to given cgroup
    fn uncollapse_cgroup(&mut self, cgroup: &str) {
        // Root is always uncollapsed
        if cgroup.is_empty() {
            return;
        }
        self.collapse_all_top_level_cgroup = false;
        let mut sub_cgroup = Some(cgroup);
        while let Some(c) = sub_cgroup {
            self.collapsed_cgroups.borrow_mut().remove(c);
            sub_cgroup = c.rsplit_once('/').map(|(s, _)| s);
        }
    }

    pub fn handle_state_for_entering_focus(&mut self, cgroup: String) {
        self.uncollapse_cgroup(cgroup.as_str());
        self.cgroup_to_focus = Some(cgroup);
    }
}

// TODO: Make CgroupView a collection of CgroupTab
pub struct CgroupView {
    tab: &'static CgroupTab,
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

        let tabs = vec![
            "General".into(),
            "CPU".into(),
            "Mem".into(),
            "I/O".into(),
            "Pressure".into(),
        ];
        let mut tabs_map: HashMap<String, CgroupView> = HashMap::new();
        tabs_map.insert(
            "General".into(),
            CgroupView {
                tab: &*CGROUP_GENERAL_TAB,
            },
        );
        tabs_map.insert(
            "CPU".into(),
            CgroupView {
                tab: &*CGROUP_CPU_TAB,
            },
        );
        tabs_map.insert(
            "Mem".into(),
            CgroupView {
                tab: &*CGROUP_MEM_TAB,
            },
        );
        tabs_map.insert(
            "I/O".into(),
            CgroupView {
                tab: &*CGROUP_IO_TAB,
            },
        );
        tabs_map.insert(
            "Pressure".into(),
            CgroupView {
                tab: &*CGROUP_PRESSURE_TAB,
            },
        );
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
                .set_sort_order(SingleCgroupModelFieldId::Cpu(
                    CgroupCpuModelFieldId::UsagePct,
                ));
            view.state.borrow_mut().set_reverse(true);
            view.refresh(c)
        })
        .on_event('M', |c| {
            let mut view = Self::get_cgroup_view(c);
            view.state
                .borrow_mut()
                .set_sort_order(SingleCgroupModelFieldId::Mem(
                    CgroupMemoryModelFieldId::Total,
                ));
            view.state.borrow_mut().set_reverse(true);
            view.refresh(c)
        })
        .on_event('D', |c| {
            let mut view = Self::get_cgroup_view(c);
            view.state
                .borrow_mut()
                .set_sort_order(SingleCgroupModelFieldId::Io(
                    CgroupIoModelFieldId::RwbytesPerSec,
                ));
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
        let cgroup_to_focus = view.state.borrow_mut().cgroup_to_focus.take();
        if let Some(cgroup) = &cgroup_to_focus {
            // Refresh before getting position to ensure cgroup is expanded
            view.refresh(c);
            let pos = view
                .get_detail_view()
                .iter()
                .position(|(_row, key)| key == cgroup);
            if let Some(pos) = pos {
                view.get_detail_view().set_selection(pos)(c);
            }
        }
        view.refresh(c);
    }
}

impl ViewBridge for CgroupView {
    type StateType = CgroupState;

    fn get_view_name() -> &'static str {
        "cgroup_view"
    }
    fn get_titles(&self) -> ColumnTitles {
        self.tab.get_titles()
    }

    fn get_rows(
        &mut self,
        state: &Self::StateType,
        offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        self.tab.get_rows(state, offset)
    }

    fn on_select_update_state(state: &mut Self::StateType, selected_key: Option<&String>) {
        state.current_selected_cgroup = selected_key.cloned().unwrap_or_default();
    }

    fn on_select_update_cmd_palette(_view: &Self::StateType, selected_key: &String) -> String {
        selected_key.clone()
    }
}
