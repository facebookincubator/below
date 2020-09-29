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
