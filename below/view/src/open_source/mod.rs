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

use crate::controllers::Controllers;
use crate::help_menu::ControllerHelper;

pub fn get_version_str() -> String {
    let version = option_env!("CARGO_PKG_VERSION");
    match version {
        None => String::from(""),
        Some(version_str) => String::from(version_str),
    }
}

pub fn get_internal_controller_str(
    _cmd_map: &HashMap<Controllers, ControllerHelper>,
) -> Vec<String> {
    return vec![];
}
