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

use build_info::BuildInfo;

use crate::controllers::Controllers;
use crate::help_menu::ControllerHelper;

pub mod fb_default_styles;
pub mod gpu_tabs;
pub mod gpu_view;

pub fn get_version_str() -> String {
    let mut vers_str = String::from(" (");
    let mut name = BuildInfo::get_package_name();
    if name.is_empty() {
        name = "below";
    }
    let mut vers = BuildInfo::get_package_version();
    if vers.is_empty() {
        vers = "dev";
    }

    vers_str += format!("{}-{}", name, vers).as_str();
    let rel = BuildInfo::get_package_release();
    if !rel.is_empty() {
        vers_str += format!("-{}", rel).as_str();
    }

    vers_str += ")";
    vers_str
}

pub fn get_extra_controller_str(cmd_map: &HashMap<Controllers, ControllerHelper>) -> Vec<String> {
    vec![
        cmd_map.get(&Controllers::Gpu).unwrap().to_string(),
        cmd_map.get(&Controllers::GpuProcess).unwrap().to_string(),
        cmd_map.get(&Controllers::GpuZoom).unwrap().to_string(),
    ]
}
