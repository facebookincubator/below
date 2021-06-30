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
    "",
    Event::Char(':'),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        view.get_cmd_palette().invoke_cmd();
    }
);

// Next Tab
make_event_controller!(
    NextTabImpl,
    "next_tab",
    "nt",
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
    "pt",
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
    "nc",
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
    "pc",
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
    "",
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
    "",
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
    "quit",
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
    "h",
    Event::Char('?'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let event_map = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .event_controllers
            .clone();
        c.add_fullscreen_layer(ResizedView::with_full_screen(
            OnEventView::new(crate::help_menu::new(event_map)).on_event(
                EventTrigger::from('q').or('?'),
                |c| {
                    c.pop_layer();
                },
            ),
        ))
    }
);

// Invoke Process View
make_event_controller!(
    ProcessView,
    "process",
    "",
    Event::Char('p'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        c.call_on_name("main_view_stack", |stack: &mut NamedView<StackView>| {
            let position = (*stack.get_mut())
                .find_layer_from_name("process_view_panel")
                .expect("Failed to find process view");
            (*stack.get_mut()).move_to_front(position);
        });

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // If the previous state is zoom state, we need to clear the zoom state
        if current_state == MainViewState::ProcessZoomedIntoCgroup {
            crate::process_view::ProcessView::get_process_view(c)
                .state
                .borrow_mut()
                .reset_state_for_quiting_zoom();
        }
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = MainViewState::Process;
    }
);

// Invoke Cgroup View
make_event_controller!(
    CgroupView,
    "cgroup",
    "",
    Event::Char('c'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        c.call_on_name("main_view_stack", |stack: &mut NamedView<StackView>| {
            let position = (*stack.get_mut())
                .find_layer_from_name("cgroup_view_panel")
                .expect("Failed to find cgroup view");
            (*stack.get_mut()).move_to_front(position);
        });

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // If the previous state is zoom state, we need to clear the zoom state
        if current_state == MainViewState::ProcessZoomedIntoCgroup {
            crate::process_view::ProcessView::get_process_view(c)
                .state
                .borrow_mut()
                .reset_state_for_quiting_zoom();
        }
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = MainViewState::Cgroup;
    }
);

// Invoke System View
make_event_controller!(
    SystemView,
    "system",
    "",
    Event::Char('s'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        c.call_on_name("main_view_stack", |stack: &mut NamedView<StackView>| {
            let position = (*stack.get_mut())
                .find_layer_from_name("core_view_panel")
                .expect("Failed to find core view");
            (*stack.get_mut()).move_to_front(position);
        });

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // If the previous state is zoom state, we need to clear the zoom state
        if current_state == MainViewState::ProcessZoomedIntoCgroup {
            crate::process_view::ProcessView::get_process_view(c)
                .state
                .borrow_mut()
                .reset_state_for_quiting_zoom();
        }
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = MainViewState::Core;
    }
);

// Zoom in View
make_event_controller!(
    ZoomView,
    "zoom",
    "",
    Event::Char('z'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let current_selection = crate::cgroup_view::CgroupView::get_cgroup_view(c)
            .state
            .borrow()
            .current_selected_cgroup
            .clone();

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        let next_state = match current_state {
            // Pressing 'z' in zoomed view should remove zoom
            // and bring user back to cgroup view
            MainViewState::ProcessZoomedIntoCgroup => {
                crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .borrow_mut()
                    .reset_state_for_quiting_zoom();
                MainViewState::Cgroup
            }
            MainViewState::Cgroup => {
                crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .borrow_mut()
                    .handle_state_for_entering_zoom(current_selection);
                MainViewState::ProcessZoomedIntoCgroup
            }
            // Pressing 'z' in process view should do nothing
            MainViewState::Process => {
                crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .borrow_mut()
                    .cgroup_filter = None;
                MainViewState::Process
            }
            _ => return,
        };

        c.call_on_name("main_view_stack", |stack: &mut NamedView<StackView>| {
            match &next_state {
                MainViewState::Process | MainViewState::ProcessZoomedIntoCgroup => {
                    // Bring process_view to front
                    let process_pos = (*stack.get_mut())
                        .find_layer_from_name("process_view_panel")
                        .expect("Failed to find process view");
                    (*stack.get_mut()).move_to_front(process_pos);
                }
                MainViewState::Cgroup => {
                    // Bring cgroup_view to front
                    let cgroup_pos = (*stack.get_mut())
                        .find_layer_from_name("cgroup_view_panel")
                        .expect("Failed to find cgroup view");
                    (*stack.get_mut()).move_to_front(cgroup_pos);
                }
                MainViewState::Core => {}
            }
        })
        .expect("failed to find main_view_stack");

        // Set next state
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = next_state;

        // Redraw screen now so we don't have to wait until next tick
        refresh(c)
    }
);

// utl function to parse page length
fn parse_page_length(cmd_vec: &[&str]) -> Result<usize, String> {
    static DEFAULT_PAGE_LENGTH: usize = 15;

    if cmd_vec.len() > 1 {
        match cmd_vec[1].parse::<usize>() {
            Ok(p) => Ok(p),
            Err(e) => Err(format!("Fail to parse argument: {}, {}", cmd_vec[1], e)),
        }
    } else {
        Ok(DEFAULT_PAGE_LENGTH)
    }
}

// Next Page
make_event_controller!(
    NextPageImpl,
    "next_page",
    "np",
    Event::CtrlChar('f'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, cmd_vec: &[&str]| {
        match parse_page_length(cmd_vec) {
            Ok(p) => {
                let mut view = StatsView::<T>::get_view(c);
                view.get_detail_view().select_down(p)(c);
                view.get_list_scroll_view().scroll_to_important_area();
            }
            Err(e) => StatsView::<T>::get_view(c).get_cmd_palette().set_alert(e),
        };
        StatsView::<T>::refresh_myself(c);
    }
);

// Prev Page
make_event_controller!(
    PrevPageImpl,
    "prev_page",
    "pp",
    Event::CtrlChar('b'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, cmd_vec: &[&str]| {
        match parse_page_length(cmd_vec) {
            Ok(p) => {
                let mut view = StatsView::<T>::get_view(c);
                view.get_detail_view().select_up(p)(c);
                view.get_list_scroll_view().scroll_to_important_area();
            }
            Err(e) => StatsView::<T>::get_view(c).get_cmd_palette().set_alert(e),
        };
        StatsView::<T>::refresh_myself(c);
    }
);
