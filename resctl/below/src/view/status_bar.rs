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

use chrono::{DateTime, Local};
use cursive::utils::markup::StyledString;
use cursive::view::{Identifiable, View};
use cursive::views::TextView;
use cursive::Cursive;

use crate::view::ViewState;

use crate::version::get_version_str;

fn get_content(c: &mut Cursive) -> impl Into<StyledString> {
    let model = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .model;
    let datetime = DateTime::<Local>::from(model.timestamp);
    let mut header_str = datetime.format("%m/%d/%Y %H:%M:%S").to_string();
    header_str += format!("      {}", &model.system.hostname).as_str();
    header_str += get_version_str().as_str();
    header_str
}

pub fn refresh(c: &mut Cursive) {
    let content = get_content(c);
    let mut v = c
        .find_name::<TextView>("status_bar")
        .expect("No status_bar view found!");
    v.set_content(content);
}

pub fn new(c: &mut Cursive) -> impl View {
    TextView::new(get_content(c)).with_name("status_bar")
}
