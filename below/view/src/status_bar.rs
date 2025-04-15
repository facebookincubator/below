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

use chrono::DateTime;
use chrono::Local;
use cursive::Cursive;
use cursive::utils::markup::StyledString;
use cursive::view::Nameable;
use cursive::view::View;
use cursive::views::TextView;

use crate::ViewState;

fn get_spacing() -> &'static str {
    "     "
}

fn get_content(c: &mut Cursive) -> impl Into<StyledString> {
    let view_state = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");
    let datetime = DateTime::<Local>::from(view_state.timestamp);
    let mut header_str = StyledString::plain(format!(
        "{}{}",
        datetime.format("%m/%d/%Y %H:%M:%S UTC%:z"),
        get_spacing()
    ));

    header_str.append_plain("Elapsed: ");
    let elapsed_rendered = format!("{}s", view_state.time_elapsed.as_secs(),);
    let lowest = view_state.lowest_time_elapsed.as_secs();
    let this = view_state.time_elapsed.as_secs();
    // 1 second jitter happens pretty often due to integer rounding
    if lowest != 0 && this >= (lowest + 2) {
        header_str.append_styled(
            elapsed_rendered,
            cursive::theme::Color::Light(cursive::theme::BaseColor::Red),
        );
    } else {
        header_str.append_plain(elapsed_rendered);
    }

    header_str.append_plain(format!(
        "{}{}{}",
        get_spacing(),
        &view_state.system.borrow().hostname,
        get_spacing(),
    ));

    header_str.append_plain(crate::get_version_str());
    header_str.append_plain(get_spacing());
    header_str.append_plain(view_state.view_mode_str());

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
