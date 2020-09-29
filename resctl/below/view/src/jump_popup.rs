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

use std::cell::RefCell;
use std::rc::Rc;

use cursive::event::Key;
use cursive::view::{Identifiable, View};
use cursive::views::{Dialog, EditView, LinearLayout, OnEventView, TextView};
use cursive::Cursive;

use common::dateutil;
use store::{advance, Direction};

use crate::ViewState;

pub fn advance_helper(
    adv: &Rc<RefCell<advance::Advance>>,
    direction: Direction,
    c: &mut Cursive,
    input: &str,
) {
    // Raise warning when input start with 0;
    if input.trim().starts_with('0') {
        view_warn!(c, "Time value should not start with 0");
        return;
    }

    // Jump for duration
    match (input.parse::<humantime::Duration>(), direction) {
        (Ok(d), Direction::Forward) => match adv.borrow_mut().jump_sample_forward(d) {
            Some(data) => c
                .user_data::<ViewState>()
                .expect("No user data set")
                .update(data),
            // This will be unlikely to happen: Only if there's no recorded data.
            // But when execution reaches here, there should be at least one sample. So
            // silently doing nothing.
            None => {}
        },
        (Ok(d), Direction::Reverse) => match adv.borrow_mut().jump_sample_backward(d) {
            Some(data) => c
                .user_data::<ViewState>()
                .expect("No user data set")
                .update(data),
            // This will be unlikely to happen: Only if there's no recorded data.
            // But when execution reaches here, there should be at least one sample. So
            // silently doing nothing.
            None => {}
        },
        _ => match dateutil::HgTime::parse(input) {
            // Jump for absolute time
            Some(pt) => {
                // For forward jumping: we will find the next available sample of the input time forward
                // For backward jumping: we will find the next available sample of the input time backward
                let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(pt.unixtime);
                match adv.borrow_mut().jump_sample_to(timestamp, direction) {
                    Some(data) => c
                        .user_data::<ViewState>()
                        .expect("No user data set")
                        .update(data),
                    None => view_warn!(c, "Cannot find available data sample"),
                }
            }
            None => {
                view_warn!(c, "Failed to parse time value: {}", input);
                return ();
            }
        },
    };
}

pub fn new(adv: Rc<RefCell<advance::Advance>>, direction: Direction) -> impl View {
    let title = match direction {
        Direction::Forward => "How far forward should we advance?",
        Direction::Reverse => "How far backward should we advance?",
    };
    OnEventView::new(
        Dialog::new()
            .title(title)
            .padding_lrtb(1, 1, 1, 0)
            .content(
                LinearLayout::vertical()
                    .child(
                        EditView::new()
                            .on_submit(move |c, input| {
                                advance_helper(&adv, direction, c, &input);
                                c.pop_layer();
                            })
                            .with_name("jump_popup"),
                    )
                    .child(TextView::new("e.g:"))
                    .child(TextView::new("  Relative Time: 10s or 3h5m or 2d"))
                    .child(TextView::new("  Absulute time: 10:00am")),
            )
            .dismiss_button("Close"),
    )
    .on_event(Key::Esc, |s| {
        s.pop_layer();
    })
}
