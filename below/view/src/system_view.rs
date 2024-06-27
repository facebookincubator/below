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
use model::system::SystemModel;
use model::BtrfsModelFieldId;
use model::KsmModelFieldId;
use model::MemoryModelFieldId;
use model::SingleCpuModelFieldId;
use model::SingleDiskModelFieldId;
use model::SingleSlabModelFieldId;
use model::VmModelFieldId;

use crate::stats_view::ColumnTitles;
use crate::stats_view::StateCommon;
use crate::stats_view::StatsView;
use crate::stats_view::ViewBridge;
use crate::system_tabs::*;
use crate::ViewState;

pub type ViewType = StatsView<SystemView>;

use crate::system_view::default_tabs::SYSTEM_BTRFS_TAB;

// TODO(T123679020): Ideally we want to decouple states for system view tabs.
// Each system view tab really deserves its own view and state
#[derive(Default)]
pub struct SystemState {
    pub filter_info: Option<(SystemStateFieldId, String)>,
    pub collapsed_disk: HashSet<String>,
    pub model: Rc<RefCell<SystemModel>>,
    pub sort_order: Option<SystemStateFieldId>,
    pub sort_tags: HashMap<String, default_tabs::SystemTabs>,
    pub reverse: bool,
}

#[derive(PartialEq)]
pub enum SystemStateFieldId {
    Disk(SingleDiskModelFieldId),
    Btrfs(BtrfsModelFieldId),
    Cpu(SingleCpuModelFieldId),
    Mem(MemoryModelFieldId),
    Vm(VmModelFieldId),
    Slab(SingleSlabModelFieldId),
    Ksm(KsmModelFieldId),
}

impl std::fmt::Display for SystemStateFieldId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disk(field) => write!(f, "{}", field),
            Self::Btrfs(field) => write!(f, "{}", field),
            Self::Cpu(field) => write!(f, "{}", field),
            Self::Mem(field) => write!(f, "{}", field),
            Self::Vm(field) => write!(f, "{}", field),
            Self::Slab(field) => write!(f, "{}", field),
            Self::Ksm(field) => write!(f, "{}", field),
        }
    }
}

impl StateCommon for SystemState {
    type ModelType = SystemModel;
    type TagType = SystemStateFieldId;
    type KeyType = String;

    fn get_filter_info(&self) -> &Option<(Self::TagType, String)> {
        &self.filter_info
    }

    fn is_filter_supported_from_tab_idx(&self, _tab: &str, idx: usize) -> bool {
        // we only enable str filtering for first col for System View
        if idx == 0 {
            return true;
        }
        false
    }

    fn get_tag_from_tab_idx(&self, tab: &str, idx: usize) -> Self::TagType {
        match tab {
            "Btrfs" => {
                let system_tab = self
                    .sort_tags
                    .get(tab)
                    .unwrap_or_else(|| panic!("Fail to find tab: {}", tab));
                let default_tabs::SystemTabs::Btrfs(system_tab) = system_tab;
                Self::TagType::Btrfs(
                    system_tab
                        .view_items
                        .get(idx)
                        .expect("Out of title scope")
                        .field_id
                        .to_owned(),
                )
            }
            "CPU" => SystemStateFieldId::Cpu(SingleCpuModelFieldId::Idx),
            "Disk" => SystemStateFieldId::Disk(SingleDiskModelFieldId::Name),
            // tabs Mem and Vm have two columns 'Field' and 'Value'. 'Field' contains
            // a list of all the FieldIds in MemoryModel and VmModel respectively.
            // the field given to filter_info don't matter for these tabs because
            // they don't use FieldId as column titles/selected col (it isn't used to filter)
            "Mem" => SystemStateFieldId::Mem(MemoryModelFieldId::Total),
            "Vm" => SystemStateFieldId::Vm(VmModelFieldId::PgpginPerSec),
            "Slab" => SystemStateFieldId::Slab(
                enum_iterator::all::<SingleSlabModelFieldId>()
                    .nth(idx)
                    .expect("Tag out of range"),
            ),
            "Ksm" => SystemStateFieldId::Ksm(KsmModelFieldId::FullScans),
            _ => panic!("bug: got unsupported tab {}", tab),
        }
    }

    fn set_filter_from_tab_idx(&mut self, tab: &str, idx: usize, filter: Option<String>) -> bool {
        if !self.is_filter_supported_from_tab_idx(tab, idx) {
            return false;
        }

        if let Some(filter_text) = filter {
            let title = self.get_tag_from_tab_idx(tab, idx);
            self.filter_info = Some((title, filter_text));
        } else {
            self.filter_info = None;
        }
        true
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
        match tab {
            "Btrfs" | "Slab" => {
                let sort_order = self.get_tag_from_tab_idx(tab, idx);
                self.set_sort_tag(sort_order, reverse)
            }
            // This is to notify that tab is not currently sortable
            _ => false,
        }
    }

    fn set_sort_string(&mut self, selection: &str, reverse: &mut bool) -> bool {
        use std::str::FromStr;
        match BtrfsModelFieldId::from_str(selection) {
            Ok(field_id) => self.set_sort_tag(SystemStateFieldId::Btrfs(field_id), reverse),
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
        sort_tags.insert(
            "Btrfs".into(),
            default_tabs::SystemTabs::Btrfs(&SYSTEM_BTRFS_TAB),
        );
        Self {
            sort_order: None,
            reverse: false,
            sort_tags,
            model,
            ..Default::default()
        }
    }
}

pub enum SystemView {
    Cpu(SystemCpu),
    Mem(SystemMem),
    Vm(SystemVm),
    Slab(SystemSlab),
    Ksm(SystemKsm),
    Disk(SystemDisk),
    Btrfs(SystemBtrfs),
}

impl SystemView {
    pub fn new(c: &mut Cursive) -> NamedView<ViewType> {
        let mut list = SelectView::<String>::new();
        list.set_on_submit(|c, idx: &String| {
            let mut view = SystemView::get_system_view(c);
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

        let tabs = vec![
            "CPU".into(),
            "Mem".into(),
            "Vm".into(),
            "Slab".into(),
            "Ksm".into(),
            "Disk".into(),
            "Btrfs".into(),
        ];
        let mut tabs_map: HashMap<String, SystemView> = HashMap::new();
        tabs_map.insert("CPU".into(), SystemView::Cpu(Default::default()));
        tabs_map.insert("Mem".into(), SystemView::Mem(Default::default()));
        tabs_map.insert("Vm".into(), SystemView::Vm(Default::default()));
        tabs_map.insert("Slab".into(), SystemView::Slab(Default::default()));
        tabs_map.insert("Ksm".into(), SystemView::Ksm(Default::default()));
        tabs_map.insert("Disk".into(), SystemView::Disk(Default::default()));
        tabs_map.insert("Btrfs".into(), SystemView::Btrfs(Default::default()));
        let user_data = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive Object!");
        StatsView::new(
            "system",
            tabs,
            tabs_map,
            list,
            SystemState::new(user_data.system.clone()),
            user_data.event_controllers.clone(),
            user_data.cmd_controllers.clone(),
        )
        .feed_data(c)
        .with_name(Self::get_view_name())
    }

    pub fn get_system_view(c: &mut Cursive) -> ViewRef<ViewType> {
        ViewType::get_view(c)
    }

    pub fn refresh(c: &mut Cursive) {
        Self::get_system_view(c).refresh(c);
    }

    fn get_inner(&self) -> Box<dyn SystemTab> {
        match self {
            Self::Cpu(inner) => Box::new(inner.clone()),
            Self::Mem(inner) => Box::new(inner.clone()),
            Self::Vm(inner) => Box::new(inner.clone()),
            Self::Slab(inner) => Box::new(inner.clone()),
            Self::Ksm(inner) => Box::new(inner.clone()),
            Self::Disk(inner) => Box::new(inner.clone()),
            Self::Btrfs(inner) => Box::new(inner.clone()),
        }
    }
}

impl ViewBridge for SystemView {
    type StateType = SystemState;
    fn get_view_name() -> &'static str {
        "system_view"
    }
    fn get_titles(&self) -> ColumnTitles {
        self.get_inner().get_titles()
    }

    fn get_rows(
        &mut self,
        state: &Self::StateType,
        offset: Option<usize>,
    ) -> Vec<(StyledString, String)> {
        self.get_inner().get_rows(state, offset)
    }

    fn on_select_update_cmd_palette(
        _view: &Self::StateType,
        selected_key: &String,
        _current_tab: &str,
        _selected_column: usize,
    ) -> String {
        selected_key.clone()
    }
}
