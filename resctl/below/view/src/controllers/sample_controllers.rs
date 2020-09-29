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

use super::*;

use crate::{jump_popup, ViewMode};
use store::Direction;

// Jump forward
make_event_controller!(
    JumpForward,
    "jump_forward",
    Event::Char('j'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let mode = c
            .user_data::<ViewState>()
            .expect("user data not set")
            .mode
            .clone();
        match mode {
            ViewMode::Pause(adv) | ViewMode::Replay(adv) => {
                c.add_layer(jump_popup::new(adv, Direction::Forward));
            }
            _ => {}
        }
    }
);

// Jump backward
make_event_controller!(
    JumpBackward,
    "jump_backward",
    Event::Char('J'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let mode = c
            .user_data::<ViewState>()
            .expect("user data not set")
            .mode
            .clone();
        match mode {
            ViewMode::Pause(adv) | ViewMode::Replay(adv) => {
                c.add_layer(jump_popup::new(adv, Direction::Reverse));
            }
            _ => {}
        }
    }
);

// Next sample
make_event_controller!(
    NextSample,
    "next_sample",
    Event::Char('t'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let mode = c
            .user_data::<ViewState>()
            .expect("user data not set")
            .mode
            .clone();
        match mode {
            ViewMode::Pause(adv) | ViewMode::Replay(adv) => {
                let mut adv = adv.borrow_mut();
                advance!(c, adv, Direction::Forward);
            }
            _ => {}
        };
        crate::status_bar::refresh(c);
        StatsView::<T>::refresh_myself(c);
    }
);

// Previous sample
make_event_controller!(
    PrevSample,
    "prev_sample",
    Event::Char('T'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let mode = c
            .user_data::<ViewState>()
            .expect("user data not set")
            .mode
            .clone();
        match mode {
            ViewMode::Pause(adv) | ViewMode::Replay(adv) => {
                let mut adv = adv.borrow_mut();
                advance!(c, adv, Direction::Reverse);
            }
            _ => {}
        }
        crate::status_bar::refresh(c);
        StatsView::<T>::refresh_myself(c);
    }
);

// Pause
make_event_controller!(
    PauseImpl,
    "pause_resume",
    Event::Char(' '),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        {
            let mut view_state = c.user_data::<ViewState>().expect("user data not set");

            match &view_state.mode {
                ViewMode::Pause(adv) => {
                    // On resume, we need to jump back to latest sample
                    adv.borrow_mut().get_latest_sample();
                    view_state.mode = ViewMode::Live(adv.clone());
                }
                ViewMode::Live(adv) => {
                    // If it's live local, we need to jump to the lastest sample
                    adv.borrow_mut().get_latest_sample();
                    view_state.mode = ViewMode::Pause(adv.clone());
                }
                _ => {}
            };
        }
        crate::status_bar::refresh(c);
    }
);
