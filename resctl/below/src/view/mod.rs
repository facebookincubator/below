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

/// View Module defines how to render below inside terminal.
///
/// ## High level architecture
/// ```
///  ------------------------------------------------------------
/// |                      Status Bar                            |
///  ------------------------------------------------------------
///  ------------------------------------------------------------
/// |                      System View                           |
///  ------------------------------------------------------------
///  ------------------------------------------------------------
/// |                      Stats View                            |
///  ------------------------------------------------------------
/// ```
/// * Status Bar: Displays datetime, elapsed time, hostname, and below version.
/// * System View: Displays overall system stats including cpu, mem, io, iface, transport, and network.
/// * Stats View: Display the detailed stats. Please check the stats view section for more details.
///
/// ### Stats View
/// ```
///  ------------------------------------------------------------
/// |                         Tabs                               |
/// | ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~|
/// |                 Column name in Bold                        |
/// |                 Selectable Stats View                      |
///  ------------------------------------------------------------
/// |                 Command Palette                            |
///  ------------------------------------------------------------
/// ```
/// * Tabs: Defines the topics of stats view. Each stats view by default will show only the general stats:
///   a combination of all important stats from each topic. User can use `tab` or `shift-tab` to switch
///   between different tabs to see the detailed view of that topic. For example, for cgroup view, the `general` tab
///   will only show pressure status of cpu_some, memory_full, io_full. But the `pressure` tab will show all
///   pressure related stats including cpu_some, memory_some, memory_full, io_some, io_full.
///
/// * Column names: The column names line also called title line in below_derive. It defines the table column of
///   the following selectable view. A user can press `,` or `.` to switch between different columns and press `s`
///   or `S` to sort in ascending or descending order.
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use ::cursive::event::{Event, EventResult, EventTrigger};
use ::cursive::theme::{BaseColor, Color, PaletteColor};
use ::cursive::view::Identifiable;
use ::cursive::views::{LinearLayout, NamedView, OnEventView, Panel, ResizedView, StackView};
use ::cursive::Cursive;
use anyhow::Result;

use crate::model::{CgroupModel, Model, NetworkModel, ProcessModel, SystemModel};
use crate::store::Direction;
use crate::Advance;

#[macro_use]
mod stats_view;
mod cgroup_tabs;
mod cgroup_view;
mod command_palette;
mod core_tabs;
mod core_view;
mod filter_popup;
mod help_menu;
mod process_tabs;
mod process_view;
mod status_bar;
mod system_view;
mod tab_view;

pub struct View {
    inner: Cursive,
}

macro_rules! advance {
    ($c:ident, $adv:ident, $dir:expr) => {
        match $adv.advance($dir) {
            Some(data) => {
                $c.user_data::<ViewState>()
                    .expect("No user data set")
                    .update(data);
                refresh($c);
            }
            None => (),
        }
    };
}

#[derive(Clone)]
pub enum MainViewState {
    Cgroup,
    Process,
    ProcessZoomedIntoCgroup,
    Core,
}

#[derive(Clone)]
pub enum VieMode {
    Live,
    Pause(Rc<RefCell<Advance>>),
    Replay(Rc<RefCell<Advance>>),
}

// Invoked either when the data view was explicitly advanced, or
// periodically (during live mode)
fn refresh(c: &mut Cursive) {
    status_bar::refresh(c);
    system_view::refresh(c);
    let current_state = c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .main_view_state
        .clone();
    match current_state {
        MainViewState::Cgroup => cgroup_view::CgroupView::refresh(c),
        MainViewState::Process | MainViewState::ProcessZoomedIntoCgroup => {
            process_view::ProcessView::refresh(c)
        }
        MainViewState::Core => core_view::CoreView::refresh(c),
    }
}

pub struct ViewState {
    pub time_elapsed: Duration,
    pub timestamp: SystemTime,
    pub system: Rc<RefCell<SystemModel>>,
    pub cgroup: Rc<RefCell<CgroupModel>>,
    pub process: Rc<RefCell<ProcessModel>>,
    pub network: Rc<RefCell<NetworkModel>>,
    pub main_view_state: MainViewState,
    pub mode: VieMode,
}

impl ViewState {
    pub fn update(&mut self, model: Model) {
        self.time_elapsed = model.time_elapsed;
        self.timestamp = model.timestamp;
        self.system.replace(model.system);
        self.cgroup.replace(model.cgroup);
        self.process.replace(model.process);
        self.network.replace(model.network);
    }

    pub fn new(main_view_state: MainViewState, model: Model) -> Self {
        Self {
            time_elapsed: model.time_elapsed,
            timestamp: model.timestamp,
            system: Rc::new(RefCell::new(model.system)),
            cgroup: Rc::new(RefCell::new(model.cgroup)),
            process: Rc::new(RefCell::new(model.process)),
            network: Rc::new(RefCell::new(model.network)),
            main_view_state,
            mode: VieMode::Live,
        }
    }

    pub fn new_with_advance(
        main_view_state: MainViewState,
        model: Model,
        advance: Advance,
    ) -> Self {
        let mut view_state = ViewState::new(main_view_state, model);
        view_state.mode = VieMode::Replay(Rc::new(RefCell::new(advance)));
        view_state
    }

    pub fn is_paused(&self) -> bool {
        match self.mode {
            VieMode::Pause(_) => true,
            _ => false,
        }
    }
}

impl View {
    pub fn new(model: crate::model::Model) -> View {
        let mut inner = Cursive::default();
        inner.set_user_data(ViewState::new(MainViewState::Cgroup, model));
        View { inner }
    }

    pub fn new_with_advance(model: crate::model::Model, advance: Advance) -> View {
        let mut inner = Cursive::default();
        inner.set_user_data(ViewState::new_with_advance(
            MainViewState::Cgroup,
            model,
            advance,
        ));
        View { inner }
    }

    pub fn cb_sink(&mut self) -> &::cursive::CbSink {
        self.inner.set_fps(4);
        self.inner.cb_sink()
    }

    pub fn register_replay_event(&mut self) {
        // Move sample forward
        self.inner.add_global_callback('t', |c| {
            let view_state = c.user_data::<ViewState>().expect("user data not set");
            match view_state.mode.clone() {
                VieMode::Pause(adv) | VieMode::Replay(adv) => {
                    let mut adv = adv.borrow_mut();
                    advance!(c, adv, Direction::Forward);
                }
                _ => (),
            }
        });

        // Move sample backward
        self.inner.add_global_callback('T', move |c| {
            let view_state = c.user_data::<ViewState>().expect("user data not set");
            match view_state.mode.clone() {
                VieMode::Pause(adv) | VieMode::Replay(adv) => {
                    let mut adv = adv.borrow_mut();
                    advance!(c, adv, Direction::Reverse);
                }
                _ => (),
            }
        });
    }

    pub fn register_live_event(&mut self, logger: slog::Logger, dir: PathBuf) {
        // Pause/Resume the sample collection by setting/unset the "advance" in ViewState
        self.inner.add_global_callback(Event::Char(' '), move |c| {
            let mut view_state = c.user_data::<ViewState>().expect("user data not set");
            if view_state.is_paused() {
                view_state.mode = VieMode::Live;
            } else {
                let mut adv = Advance::new(logger.clone(), dir.clone(), SystemTime::now());
                adv.initialize();
                view_state.mode = VieMode::Pause(Rc::new(RefCell::new(adv)));
            }
            refresh(c);
        });
        self.register_replay_event();
    }

    pub fn run(&mut self) -> Result<()> {
        let mut theme = self.inner.current_theme().clone();
        theme.palette[PaletteColor::Background] = Color::TerminalDefault;
        theme.palette[PaletteColor::View] = Color::TerminalDefault;
        theme.palette[PaletteColor::Primary] = Color::TerminalDefault;
        theme.palette[PaletteColor::Highlight] = Color::Dark(BaseColor::Cyan);
        theme.palette[PaletteColor::HighlightText] = Color::Dark(BaseColor::Black);
        theme.shadow = false;

        self.inner.set_theme(theme);

        self.inner.add_global_callback('q', Cursive::quit);
        self.inner.add_global_callback('?', |s| {
            s.add_fullscreen_layer(ResizedView::with_full_screen(
                OnEventView::new(help_menu::new()).on_event(EventTrigger::from('q').or('?'), |s| {
                    s.pop_layer();
                }),
            ))
        });
        self.inner
            .add_global_callback(Event::CtrlChar('z'), |_| unsafe {
                if libc::raise(libc::SIGTSTP) != 0 {
                    panic!("failed to SIGTSTP self");
                }
            });
        self.inner.add_global_callback(Event::Refresh, |c| {
            refresh(c);
        });

        let status_bar = status_bar::new(&mut self.inner);
        let system_view = system_view::new(&mut self.inner);
        let process_view = process_view::ProcessView::new(&mut self.inner);
        let cgroup_view = cgroup_view::CgroupView::new(&mut self.inner);
        let core_view = core_view::CoreView::new(&mut self.inner);
        self.inner
            .add_fullscreen_layer(ResizedView::with_full_screen(
                LinearLayout::vertical()
                    .child(Panel::new(status_bar))
                    .child(Panel::new(system_view))
                    .child(
                        OnEventView::new(
                            StackView::new()
                                .fullscreen_layer(ResizedView::with_full_screen(
                                    core_view.with_name("core_view_panel"),
                                ))
                                .fullscreen_layer(ResizedView::with_full_screen(
                                    process_view.with_name("process_view_panel"),
                                ))
                                .fullscreen_layer(ResizedView::with_full_screen(
                                    cgroup_view.with_name("cgroup_view_panel"),
                                ))
                                .with_name("main_view_stack"),
                        )
                        .on_pre_event_inner('p', |stack, _| {
                            let position = (*stack.get_mut())
                                .find_layer_from_name("process_view_panel")
                                .expect("Failed to find process view");
                            (*stack.get_mut()).move_to_front(position);

                            Some(EventResult::with_cb(|c| {
                                let view_state = c
                                    .user_data::<ViewState>()
                                    .expect("No data stored in Cursive object!");
                                view_state.main_view_state = MainViewState::Process;
                            }))
                        })
                        .on_pre_event_inner('c', |stack, _| {
                            let position = (*stack.get_mut())
                                .find_layer_from_name("cgroup_view_panel")
                                .expect("Failed to find cgroup view");
                            (*stack.get_mut()).move_to_front(position);

                            Some(EventResult::with_cb(|c| {
                                let view_state = c
                                    .user_data::<ViewState>()
                                    .expect("No data stored in Cursive object!");
                                view_state.main_view_state = MainViewState::Cgroup;
                            }))
                        })
                        .on_pre_event_inner('s', |stack, _| {
                            let position = (*stack.get_mut())
                                .find_layer_from_name("core_view_panel")
                                .expect("Failed to find core view");
                            (*stack.get_mut()).move_to_front(position);

                            Some(EventResult::with_cb(|c| {
                                let view_state = c
                                    .user_data::<ViewState>()
                                    .expect("No data stored in Cursive object!");
                                view_state.main_view_state = MainViewState::Core;
                            }))
                        })
                        .on_pre_event('z', |c| {
                            let current_selection = cgroup_view::CgroupView::get_cgroup_view(c)
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
                                    process_view::ProcessView::get_process_view(c)
                                        .state
                                        .borrow_mut()
                                        .cgroup_filter = None;
                                    MainViewState::Cgroup
                                }
                                MainViewState::Cgroup => {
                                    process_view::ProcessView::get_process_view(c)
                                        .state
                                        .borrow_mut()
                                        .cgroup_filter = Some(current_selection);
                                    MainViewState::ProcessZoomedIntoCgroup
                                }
                                // Pressing 'z' in process view should do nothing
                                MainViewState::Process => {
                                    process_view::ProcessView::get_process_view(c)
                                        .state
                                        .borrow_mut()
                                        .cgroup_filter = None;
                                    MainViewState::Process
                                }
                                _ => return,
                            };

                            c.call_on_name(
                                "main_view_stack",
                                |stack: &mut NamedView<StackView>| {
                                    match &next_state {
                                        MainViewState::Process
                                        | MainViewState::ProcessZoomedIntoCgroup => {
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
                                        MainViewState::Core => (),
                                    }
                                },
                            )
                            .expect("failed to find main_view_stack");

                            // Set next state
                            c.user_data::<ViewState>()
                                .expect("No data stored in Cursive object!")
                                .main_view_state = next_state;

                            // Redraw screen now so we don't have to wait until next tick
                            refresh(c)
                        })
                        .with_name("dynamic_view"),
                    ),
            ));

        self.inner
            .focus_name("dynamic_view")
            .expect("Could not set focus at initialization!");

        self.inner.run();

        Ok(())
    }
}
