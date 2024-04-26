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

use serde::Deserialize;

use super::get_belowrc_filename;
use super::get_belowrc_view_section_key;

/// Enum of supported front view.
// We didn't re-use the MainViewState because we don't want to
// expose internal state like Process(ProcessZoomState::Cgroup)
#[derive(Clone, Deserialize)]
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
    // Overrides cgroup name column width.
    pub cgroup_name_width: Option<usize>,
}

impl ViewRc {
    /// Create a new ViewRc object base on the content in
    /// $HOME/.config/below/belowrc. Will return default ViewRc if the belowrc
    /// file is missing or view section does not exists. Optionally return a
    /// parse error string.
    pub fn new() -> (ViewRc, Option<String>) {
        match std::fs::read_to_string(get_belowrc_filename()) {
            Ok(belowrc_str) => match belowrc_str.parse::<toml::value::Value>() {
                // We get the belowrc file, parsing the [view] section
                Ok(belowrc_val) => {
                    if let Some(viewrc_val) = belowrc_val.get(get_belowrc_view_section_key()) {
                        // Got the [view] section, let's see if we can deserialize it to ViewRc
                        match viewrc_val.to_owned().try_into::<ViewRc>() {
                            Ok(viewrc) => (viewrc, None),
                            Err(e) => (
                                Default::default(),
                                Some(format!(
                                    "Failed to parse belowrc::{}: {}",
                                    get_belowrc_view_section_key(),
                                    e
                                )),
                            ),
                        }
                    } else {
                        Default::default()
                    }
                }
                Err(e) => (
                    Default::default(),
                    Some(format!("Failed to parse belowrc: {}", e)),
                ),
            },
            _ => (Default::default(), None),
        }
    }
}
