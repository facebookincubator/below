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

use super::cgroup_view::CgroupView;
use super::controllers::Controllers;
use super::{get_belowrc_filename, get_belowrc_view_section_key};

use cursive::Cursive;
use serde::Deserialize;

/// Enum of supported front view.
// We didn't re-use the MainViewState because we don't want to
// expose those internal state like ProcessZoomedIntoCgroup
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultFrontView {
    Cgroup,
    Process,
    System,
}

/// Runtime configuration on the below view.
#[derive(Default, Deserialize)]
pub struct ViewRc {
    // The default front view. If this field is not set, we will use cgroup
    // view as front view
    pub default_view: Option<DefaultFrontView>,
    // If we want to collapse all top level cgroups. If this field is not set,
    // it will be treated as false
    pub collapse_cgroups: Option<bool>,
}

impl ViewRc {
    /// Create a new ViewRc object base on the content in
    /// $HOME/.config/below/belowrc. Will return default ViewRc if the belowrc
    /// file is missing or view section does not exists. Will raise a warning
    /// in the command palette if the belowrc file is malformated.
    fn new(c: &mut Cursive) -> ViewRc {
        match std::fs::read_to_string(get_belowrc_filename()) {
            Ok(belowrc_str) => match belowrc_str.parse::<toml::value::Value>() {
                // We get the belowrc file, parsing the [view] section
                Ok(belowrc_val) => {
                    if let Some(viewrc_val) = belowrc_val.get(get_belowrc_view_section_key()) {
                        // Got the [view] section, let's see if we can deserialize it to ViewRc
                        match viewrc_val.to_owned().try_into::<ViewRc>() {
                            Ok(viewrc) => viewrc,
                            Err(e) => {
                                view_warn!(
                                    c,
                                    "Failed to parse belowrc::{}: {}",
                                    get_belowrc_view_section_key(),
                                    e
                                );
                                Default::default()
                            }
                        }
                    } else {
                        Default::default()
                    }
                }
                Err(e) => {
                    view_warn!(c, "Failed to parse belowrc: {}", e);
                    Default::default()
                }
            },
            _ => Default::default(),
        }
    }

    /// Fold the top level cgroups base on the value of collapse_cgroups.
    pub fn process_collapse_cgroups(&self, c: &mut Cursive) {
        if Some(true) == self.collapse_cgroups {
            let cgroup_view = CgroupView::get_cgroup_view(c);
            cgroup_view.state.borrow_mut().collapse_all_top_level_cgroup = true;
            cgroup_view
                .state
                .borrow_mut()
                .collapsed_cgroups
                .borrow_mut()
                .clear();
        }
    }

    /// Move the desired view to front base on the value of default_view
    pub fn process_default_view(&self, c: &mut Cursive) {
        match self.default_view {
            Some(DefaultFrontView::Cgroup) => Controllers::Cgroup.callback::<CgroupView>(c, &[]),
            Some(DefaultFrontView::Process) => Controllers::Process.callback::<CgroupView>(c, &[]),
            Some(DefaultFrontView::System) => Controllers::System.callback::<CgroupView>(c, &[]),
            None => {}
        }
    }

    /// Syntactic sugar for processing the belowrc file.
    pub fn process(c: &mut Cursive) {
        let viewrc = Self::new(c);
        viewrc.process_default_view(c);
        viewrc.process_collapse_cgroups(c);
        super::refresh(c);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::fake_view::FakeView;
    use crate::{MainViewState, ViewState};

    #[test]
    fn test_viewrc_collapse_cgroups() {
        let cgroup_collapsed = |c: &mut Cursive| -> bool {
            let cgroup_view = CgroupView::get_cgroup_view(c);
            let res = cgroup_view.state.borrow_mut().collapse_all_top_level_cgroup;
            res
        };
        let mut view = FakeView::new();
        view.add_cgroup_view();

        // Test for default setup
        {
            let viewrc: ViewRc = Default::default();
            viewrc.process_collapse_cgroups(&mut view.inner);
            assert!(!cgroup_collapsed(&mut view.inner));
        }

        // Test for collapse_cgroups = false
        {
            let viewrc = ViewRc {
                collapse_cgroups: Some(false),
                ..Default::default()
            };
            viewrc.process_collapse_cgroups(&mut view.inner);
            assert!(!cgroup_collapsed(&mut view.inner));
        }

        // Test for collapse_cgroups = true
        {
            let viewrc = ViewRc {
                collapse_cgroups: Some(true),
                ..Default::default()
            };
            viewrc.process_collapse_cgroups(&mut view.inner);
            assert!(cgroup_collapsed(&mut view.inner));
        }
    }

    #[test]
    fn test_viewrc_default_view() {
        let mut view = FakeView::new();

        let desired_state = vec![
            None,
            Some(DefaultFrontView::Cgroup),
            Some(DefaultFrontView::Process),
            Some(DefaultFrontView::System),
        ];
        let expected_state = vec![
            MainViewState::Cgroup,
            MainViewState::Cgroup,
            MainViewState::Process,
            MainViewState::Core,
        ];
        desired_state
            .into_iter()
            .zip(expected_state)
            .for_each(move |(desired, expected)| {
                let viewrc = ViewRc {
                    default_view: desired,
                    ..Default::default()
                };
                viewrc.process_default_view(&mut view.inner);
                let current_state = view
                    .inner
                    .user_data::<ViewState>()
                    .expect("No data stored in Cursive object!")
                    .main_view_state
                    .clone();
                assert_eq!(current_state, expected);
            });
    }
}
