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
use cursive::Cursive;
use cursive::event::Event;

use super::*;
use crate::MainViewState;
use crate::ProcessZoomState;
use crate::set_active_screen;
use crate::stats_view::StatsView;
use crate::stats_view::ViewBridge;

// Invoke Gpu View
#[cfg(fbcode_build)]
make_event_controller!(
    GpuView,
    "gpu",
    "",
    vec![Event::Char('g')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        set_active_screen(c, "gpu_view_panel");

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
            .main_view_state = MainViewState::Gpu;
    }
);

// Invoke Gpu Processes View
#[cfg(fbcode_build)]
make_event_controller!(
    GpuProcessView,
    "gpu_process",
    "",
    vec![Event::Char('G')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
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

        let all_gpu_pids = crate::gpu_view::GpuView::get_gpu_view(c)
            .state
            .lock()
            .unwrap()
            .get_all_gpu_pids()
            .unwrap_or_default();

        crate::process_view::ProcessView::get_process_view(c)
            .state
            .lock()
            .unwrap()
            .handle_state_for_entering_pids_zoom(all_gpu_pids);

        set_active_screen(c, "process_view_panel");

        // Set next state
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = MainViewState::Process(ProcessZoomState::Pids);

        // Redraw screen now so we don't have to wait until next tick
        refresh(c)
    }
);

// Zoom in View
make_event_controller!(
    GpuZoomView,
    "gpu_zoom",
    "",
    vec![Event::Char('Z')],
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let current_state = c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state
            .clone();

        let next_state = match current_state {
            // Pressing 'Z' in zoomed view should remove zoom
            // and bring user back to GPU view
            MainViewState::Process(ProcessZoomState::Pids) => {
                crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .lock()
                    .unwrap()
                    .reset_state_for_quiting_zoom();
                MainViewState::Gpu
            }
            MainViewState::Gpu => {
                let current_selection = crate::gpu_view::GpuView::get_gpu_view(c)
                    .state
                    .lock()
                    .unwrap()
                    .get_pids_for_current_selected_dev()
                    .unwrap_or_default();
                crate::process_view::ProcessView::get_process_view(c)
                    .state
                    .lock()
                    .unwrap()
                    .handle_state_for_entering_pids_zoom(current_selection);
                MainViewState::Process(ProcessZoomState::Pids)
            }
            // Pressing 'Z' in process view should do nothing
            _ => return,
        };

        match &next_state {
            MainViewState::Process(ProcessZoomState::Pids) => {
                // Bring process_view to front
                set_active_screen(c, "process_view_panel");
            }
            MainViewState::Gpu => {
                // Bring gpu_view to front
                set_active_screen(c, "gpu_view_panel");
            }
            _ => panic!("bug: next_state is {:?}", next_state),
        }

        // Set next state
        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_state = next_state;

        // Redraw screen now so we don't have to wait until next tick
        refresh(c)
    }
);
