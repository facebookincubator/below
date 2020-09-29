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
use cursive::views::ViewRef;
use cursive::Cursive;

use super::*;
use view::{
    cgroup_view::CgroupView, command_palette::CommandPalette, stats_view::StatsView, MainViewState,
    ViewMode, ViewState,
};

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct FakeView {
    pub inner: Cursive,
}

// TODO Add view and controller related tests (T76419919)
impl FakeView {
    pub fn new() -> Self {
        let time = SystemTime::now();
        let logger = get_logger();
        let advance = Advance::new(logger.clone(), PathBuf::new(), time);
        let mut collector = Collector::new(get_dummy_exit_data());
        let model = collector.update_model(&logger).expect("Fail to get model");

        let mut inner = Cursive::dummy();
        inner.set_user_data(ViewState::new_with_advance(
            MainViewState::Cgroup,
            model,
            ViewMode::Live(Rc::new(RefCell::new(advance))),
        ));

        Self { inner }
    }

    pub fn add_cgroup_view(&mut self) {
        let cgroup_view = CgroupView::new(&mut self.inner);
        self.inner.add_layer(cgroup_view);
    }

    pub fn get_cmd_palette(&mut self, name: &str) -> ViewRef<CommandPalette> {
        self.inner
            .find_name::<StatsView<CgroupView>>(name)
            .expect("Failed to dereference command palette")
            .get_cmd_palette()
    }
}
