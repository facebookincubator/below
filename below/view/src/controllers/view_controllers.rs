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

use cursive::views::OnEventView;
use cursive::views::ResizedView;

use super::*;
use crate::MainViewState;
use crate::ProcessZoomState;
use crate::set_active_screen;

// Invoke command palette
make_event_controller!(
    InvokeCmdPalette,
    "invoke_cmd_palette",
    "",
    vec![Event::Char(':')],
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        view.get_cmd_palette().invoke_cmd();
    }
);

// Next Tab
make_event_controller!(
    NextTabImpl,
    "next_tab",
    "nt",
    vec![Event::Key(Key::Tab)],
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
    vec![Event::Shift(Key::Tab)],
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
    vec![Event::Char('.')],
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
    vec![Event::Char(',')],
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
    vec![Event::Key(Key::Right)],
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
    vec![Event::Key(Key::Left)],
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
    vec![Event::Char('q')],
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
    vec![Event::Char('?'), Event::Char('h')],
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
    vec![Event::Char('p')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        set_active_screen(c, "process_view_panel");

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // If the previous state is zoom state, we need to clear the zoom state
        if current_state.is_process_zoom_state() {
            crate::process_view::ProcessView::get_process_view(c)
                .state
                .lock()
                .unwrap()
                .reset_state_for_quiting_zoom();
        }
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = MainViewState::Process(ProcessZoomState::NoZoom);
    }
);

// Invoke Cgroup View
make_event_controller!(
    CgroupView,
    "cgroup",
    "",
    vec![Event::Char('c')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        set_active_screen(c, "cgroup_view_panel");

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // If the previous state is zoom state, we need to clear the zoom state
        if current_state.is_process_zoom_state() {
            crate::process_view::ProcessView::get_process_view(c)
                .state
                .lock()
                .unwrap()
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
    vec![Event::Char('s')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        set_active_screen(c, "system_view_panel");

        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // If the previous state is zoom state, we need to clear the zoom state
        if current_state.is_process_zoom_state() {
            crate::process_view::ProcessView::get_process_view(c)
                .state
                .lock()
                .unwrap()
                .reset_state_for_quiting_zoom();
        }
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = MainViewState::System;
    }
);

// Zoom in View
make_event_controller!(
    ZoomView,
    "zoom",
    "",
    vec![Event::Char('z')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        let next_state = match current_state {
            // Pressing 'z' in process view should remove any zoom
            // and bring user back to cgroup view, selected on
            // leaf cgroup of selected process
            MainViewState::Process(zoom) => {
                if zoom != ProcessZoomState::NoZoom {
                    crate::process_view::ProcessView::get_process_view(c)
                        .state
                        .lock()
                        .unwrap()
                        .reset_state_for_quiting_zoom();
                }
                let selected_cgroup = crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .lock()
                    .unwrap()
                    .get_cgroup_for_selected_pid();
                if let Some(cgroup) = selected_cgroup {
                    crate::cgroup_view::CgroupView::get_cgroup_view(c)
                        .state
                        .lock()
                        .unwrap()
                        .handle_state_for_entering_focus(cgroup);
                } else {
                    // Probably no entries in process view. We still move to
                    // cgroup view in this case.
                }
                MainViewState::Cgroup
            }
            MainViewState::Cgroup => {
                let current_selection = crate::cgroup_view::CgroupView::get_cgroup_view(c)
                    .state
                    .lock()
                    .unwrap()
                    .current_selected_cgroup
                    .clone();
                crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .lock()
                    .unwrap()
                    .handle_state_for_entering_zoom(current_selection);
                MainViewState::Process(ProcessZoomState::Cgroup)
            }
            _ => return,
        };

        match &next_state {
            MainViewState::Process(_) => {
                // Bring process_view to front
                set_active_screen(c, "process_view_panel");
            }
            MainViewState::Cgroup => {
                // Bring cgroup_view to front
                set_active_screen(c, "cgroup_view_panel");
            }
            MainViewState::System => {}
            #[cfg(fbcode_build)]
            MainViewState::Gpu => {}
        }

        // Set next state
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = next_state;

        // Redraw screen now so we don't have to wait until next tick
        refresh(c)
    }
);

// Fold processes in process view
make_event_controller!(
    FoldProcessView,
    "fold",
    "",
    vec![Event::Char('f')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        // NB: scope the borrowing to refresh() doesn't re-borrow and panic
        if current_state == MainViewState::Process(ProcessZoomState::NoZoom) {
            let mut process_view = crate::process_view::ProcessView::get_process_view(c);
            process_view.get_cmd_palette().toggle_fold();
            process_view.state.lock().unwrap().toggle_fold();
        }

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
    vec![Event::CtrlChar('f')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, cmd_vec: &[&str]| {
        match parse_page_length(cmd_vec) {
            Ok(p) => {
                let mut view = StatsView::<T>::get_view(c);
                view.get_detail_view().select_down(p)(c);
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
    vec![Event::CtrlChar('b')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, cmd_vec: &[&str]| {
        match parse_page_length(cmd_vec) {
            Ok(p) => {
                let mut view = StatsView::<T>::get_view(c);
                view.get_detail_view().select_up(p)(c);
            }
            Err(e) => StatsView::<T>::get_view(c).get_cmd_palette().set_alert(e),
        };
        StatsView::<T>::refresh_myself(c);
    }
);

make_event_controller!(
    NextSelectionImpl,
    "next_selection",
    "ns",
    vec![Event::CtrlChar('n')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        {
            let mut view = StatsView::<T>::get_view(c);
            view.get_detail_view().select_down(1)(c);
        }
        StatsView::<T>::refresh_myself(c);
    }
);

make_event_controller!(
    PrevSelectionImpl,
    "prev_selection",
    "ps",
    vec![Event::CtrlChar('p')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        {
            let mut view = StatsView::<T>::get_view(c);
            view.get_detail_view().select_up(1)(c);
        }
        StatsView::<T>::refresh_myself(c);
    }
);
