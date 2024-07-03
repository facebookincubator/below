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

use common::dateutil;
use cursive::event::Key;
use cursive::view::Nameable;
use cursive::view::View;
use cursive::views::Dialog;
use cursive::views::EditView;
use cursive::views::LinearLayout;
use cursive::views::OnEventView;
use cursive::views::TextView;
use cursive::Cursive;
use store::Advance;
use store::Direction;

use crate::ViewState;

pub fn advance_helper(
    adv: &Rc<RefCell<Advance>>,
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
        (Ok(d), Direction::Forward) => {
            if let Some(data) = adv.borrow_mut().jump_sample_forward(d) {
                c.user_data::<ViewState>()
                    .expect("No user data set")
                    .update(data)
            } else {
                // This will be unlikely to happen: Only if there's no recorded data.
                // But when execution reaches here, there should be at least one sample. So
                // silently doing nothing.
            }
        }
        (Ok(d), Direction::Reverse) => {
            if let Some(data) = adv.borrow_mut().jump_sample_backward(d) {
                c.user_data::<ViewState>()
                    .expect("No user data set")
                    .update(data)
            } else {
                // This will be unlikely to happen: Only if there's no recorded data.
                // But when execution reaches here, there should be at least one sample. So
                // silently doing nothing.
            }
        }
        _ => match dateutil::HgTime::parse_time_of_day(input) {
            Some(time_of_day) => {
                // If an absolute time without date is provided, the viewing date will be used
                let view_time = c
                    .user_data::<ViewState>()
                    .expect("user data not set")
                    .timestamp;

                match dateutil::HgTime::time_of_day_relative_to_system_time(view_time, time_of_day)
                {
                    Some(timestamp) => match adv.borrow_mut().jump_sample_to(timestamp) {
                        Some(data) => c
                            .user_data::<ViewState>()
                            .expect("No user data set")
                            .update(data),
                        None => view_warn!(c, "Cannot find available data sample"),
                    },
                    None => {
                        view_warn!(c, "Failed to parse time of day value: {}", input);
                        return;
                    }
                }
            }
            None => {
                match dateutil::HgTime::parse(input) {
                    // Jump for absolute time
                    Some(pt) => {
                        // For forward jumping: we will find the next available sample of the input time forward
                        // For backward jumping: we will find the next available sample of the input time backward
                        let timestamp =
                            std::time::UNIX_EPOCH + std::time::Duration::from_secs(pt.unixtime);
                        match adv.borrow_mut().jump_sample_to(timestamp) {
                            Some(data) => c
                                .user_data::<ViewState>()
                                .expect("No user data set")
                                .update(data),
                            None => view_warn!(c, "Cannot find available data sample"),
                        }
                    }
                    None => {
                        view_warn!(c, "Failed to parse time value: {}", input);
                        return;
                    }
                }
            }
        },
    };

    crate::refresh(c);
}

pub fn new(adv: Rc<RefCell<Advance>>, direction: Direction) -> impl View {
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
                                advance_helper(&adv, direction, c, input);
                                c.pop_layer();
                            })
                            .with_name("jump_popup"),
                    )
                    .child(TextView::new("e.g:"))
                    .child(TextView::new("  Relative Time: 10s or 3h5m or 2d"))
                    .child(TextView::new("  Absolute time: 01/01/1970 11:59PM"))
                    .child(TextView::new("  Time Of Day: 10:00am")),
            )
            .dismiss_button("Close"),
    )
    .on_event(Key::Esc, |s| {
        s.pop_layer();
    })
}
