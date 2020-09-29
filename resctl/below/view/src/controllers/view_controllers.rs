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

use crate::MainViewState;
use cursive::views::{NamedView, OnEventView, ResizedView, StackView};

// Invoke command palette
make_event_controller!(
    InvokeCmdPalette,
    "invoke_cmd_palette",
    Event::Char(':'),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        view.get_cmd_palette().invoke_cmd();
    }
);

// Next Tab
make_event_controller!(
    NextTabImpl,
    "next_tab",
    Event::Key(Key::Tab),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        view.get_tab_view().on_tab();
        view.update_title();
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// Prev Tab
make_event_controller!(
    PrevTabImpl,
    "prev_tab",
    Event::Shift(Key::Tab),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        view.get_tab_view().on_shift_tab();
        view.update_title();
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// Next column
make_event_controller!(
    NextColImpl,
    "next_col",
    Event::Char('.'),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        let x = view.get_title_view().on_tab();
        view.set_horizontal_offset(x);
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// Prev column
make_event_controller!(
    PrevColImpl,
    "prev_col",
    Event::Char(','),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        let x = view.get_title_view().on_shift_tab();
        view.set_horizontal_offset(x);
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// Right handler impl
make_event_controller!(
    RightImpl,
    "right",
    Event::Key(Key::Right),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        let screen_width = view.get_screen_width();
        view.get_title_view().on_right(screen_width);
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// Left handler impl
make_event_controller!(
    LeftImpl,
    "left",
    Event::Key(Key::Left),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        view.get_title_view().on_left();
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// quit
make_event_controller!(
    QuitImpl,
    "q",
    Event::Char('q'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        c.quit();
    }
);

// Invoke Helper menu
make_event_controller!(
    HelpMenu,
    "help",
    Event::Char('?'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        c.add_fullscreen_layer(ResizedView::with_full_screen(
            OnEventView::new(crate::help_menu::new()).on_event(
                EventTrigger::from('q').or('?'),
                |c| {
                    c.pop_layer();
                },
            ),
        ))
    }
);
