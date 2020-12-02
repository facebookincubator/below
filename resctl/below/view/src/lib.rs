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
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use cursive::event::Event;
use cursive::theme::{BaseColor, Color, PaletteColor};
use cursive::view::Identifiable;
use cursive::views::{LinearLayout, OnEventView, Panel, ResizedView, StackView};
use cursive::Cursive;
use toml::value::Value;

use common::open_source_shim;
use model::{CgroupModel, Model, NetworkModel, ProcessModel, SystemModel};
use store::advance::Advance;

open_source_shim!();

#[macro_use]
pub mod stats_view;
mod cgroup_tabs;
pub mod cgroup_view;
pub mod command_palette;
mod core_tabs;
mod core_view;
mod filter_popup;
mod help_menu;
mod process_tabs;
mod process_view;
mod status_bar;
mod system_view;
mod tab_view;

const BELOW_CMD_RC: &str = "/.config/below/cmdrc";

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
            }
            None => view_warn!(
                $c,
                "Data is not available{}",
                if $dir == Direction::Forward {
                    " yet."
                } else {
                    "."
                }
            ),
        }
    };
}

// Raise warning message in current view.
macro_rules! view_warn {
    ($c:ident, $($args:tt)*) => {{
        let state = $c
            .user_data::<crate::ViewState>()
            .expect("No user data set")
            .main_view_state
            .clone();
        let msg = format!($($args)*);
        match state {
            crate::MainViewState::Cgroup => crate::cgroup_view::ViewType::cp_warn($c, &msg),
            crate::MainViewState::Process | crate::MainViewState::ProcessZoomedIntoCgroup => {
                crate::process_view::ViewType::cp_warn($c, &msg)
            }
            crate::MainViewState::Core => crate::core_view::ViewType::cp_warn($c, &msg),
        }
    }};
}

// controllers depends on Advance
pub mod controllers;
// Jump popup depends on view_warn
mod jump_popup;

#[derive(Clone, PartialEq)]
pub enum MainViewState {
    Cgroup,
    Process,
    ProcessZoomedIntoCgroup,
    Core,
}

#[derive(Clone)]
pub enum ViewMode {
    Live(Rc<RefCell<Advance>>),
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
    pub mode: ViewMode,
    pub event_controllers: Rc<RefCell<HashMap<Event, controllers::Controllers>>>,
    pub cmd_controllers: Rc<RefCell<HashMap<&'static str, controllers::Controllers>>>,
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

    pub fn new_with_advance(main_view_state: MainViewState, model: Model, mode: ViewMode) -> Self {
        Self {
            time_elapsed: model.time_elapsed,
            timestamp: model.timestamp,
            system: Rc::new(RefCell::new(model.system)),
            cgroup: Rc::new(RefCell::new(model.cgroup)),
            process: Rc::new(RefCell::new(model.process)),
            network: Rc::new(RefCell::new(model.network)),
            main_view_state,
            mode,
            event_controllers: Rc::new(RefCell::new(HashMap::new())),
            cmd_controllers: Rc::new(RefCell::new(controllers::make_cmd_controller_map())),
        }
    }

    pub fn view_mode_str(&self) -> &'static str {
        match self.mode {
            ViewMode::Live(_) => "live",
            ViewMode::Pause(_) => "live-paused",
            ViewMode::Replay(_) => "replay",
        }
    }

    pub fn is_paused(&self) -> bool {
        match self.mode {
            ViewMode::Pause(_) => true,
            _ => false,
        }
    }
}

impl View {
    pub fn new_with_advance(model: model::Model, mode: ViewMode) -> View {
        let mut inner = cursive::Cursive::new(|| {
            let termion_backend = cursive::backends::crossterm::Backend::init().unwrap();
            Box::new(cursive_buffered_backend::BufferedBackend::new(
                termion_backend,
            ))
        });
        inner.set_user_data(ViewState::new_with_advance(
            MainViewState::Cgroup,
            model,
            mode,
        ));
        View { inner }
    }

    pub fn cb_sink(&mut self) -> &::cursive::CbSink {
        self.inner.set_fps(4);
        self.inner.cb_sink()
    }

    // Function to generate event_controller_map, we cannot make
    // event_controller_map during ViewState construction since it
    // depends on CommandPalette to construct for raising errors
    pub fn generate_event_controller_map(c: &mut Cursive, filename: String) {
        // Verify cmdrc file format
        let cmdrc_opt = match std::fs::read_to_string(filename) {
            Ok(cmdrc_str) => match cmdrc_str.parse::<Value>() {
                Ok(cmdrc) => Some(cmdrc),
                Err(e) => {
                    view_warn!(c, "Failed to parse cmdrc: {}", e);
                    None
                }
            },
            _ => None,
        };

        let event_controller_map = controllers::make_event_controller_map(c, &cmdrc_opt);

        c.user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .event_controllers
            .replace(event_controller_map);
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

        self.inner
            .add_global_callback(Event::CtrlChar('z'), |_| unsafe {
                if libc::raise(libc::SIGTSTP) != 0 {
                    panic!("failed to SIGTSTP self");
                }
            });
        self.inner.add_global_callback(Event::Refresh, |c| {
            refresh(c);
        });
        self.inner.add_global_callback(Event::CtrlChar('r'), |c| {
            c.clear();
            refresh(c);
        });

        let status_bar = status_bar::new(&mut self.inner);
        let system_view = system_view::new(&mut self.inner);
        let cgroup_view = cgroup_view::CgroupView::new(&mut self.inner);
        let process_view = process_view::ProcessView::new(&mut self.inner);
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
                        .with_name("dynamic_view"),
                    ),
            ));

        self.inner
            .focus_name("dynamic_view")
            .expect("Could not set focus at initialization!");

        // Raise warning message if failed to map the customzied command.
        Self::generate_event_controller_map(
            &mut self.inner,
            format!(
                "{}{}",
                std::env::var("HOME").unwrap_or_else(|_| "".into()),
                BELOW_CMD_RC
            ),
        );
        self.inner.run();

        Ok(())
    }
}
