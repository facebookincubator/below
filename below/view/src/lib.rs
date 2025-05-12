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

#![deny(clippy::all)]

/// View Module defines how to render below inside terminal.
///
/// ## High level architecture
/// ```text
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
/// ```text
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
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Result;
use common::logutil::get_last_log_to_display;
use common::open_source_shim;
use common::util::get_belowrc_cmd_section_key;
use common::util::get_belowrc_filename;
use common::util::get_belowrc_view_section_key;
use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use cursive::Cursive;
use cursive::CursiveRunnable;
use cursive::ScreenId;
use cursive::event::Event;
use cursive::theme::BaseColor;
use cursive::theme::Color;
use cursive::theme::PaletteColor;
use cursive::view::Nameable;
use cursive::views::BoxedView;
use cursive::views::LinearLayout;
use cursive::views::NamedView;
use cursive::views::OnEventView;
use cursive::views::Panel;
use cursive::views::ResizedView;
use cursive::views::ScreensView;
use model::CgroupModel;
#[cfg(fbcode_build)]
use model::GpuModel;
use model::Model;
use model::NetworkModel;
use model::ProcessModel;
use model::SystemModel;
use store::Advance;
use toml::value::Value;
use viewrc::ViewRc;
extern crate render as base_render;

open_source_shim!();

mod cgroup_tabs;
pub mod cgroup_view;
pub mod command_palette;
mod default_styles;
mod filter_popup;
mod help_menu;
mod process_tabs;
mod process_view;
mod render;
pub mod stats_view;
mod status_bar;
mod summary_view;
mod system_tabs;
mod system_view;
mod tab_view;

pub struct View {
    inner: CursiveRunnable,
}

macro_rules! advance {
    ($c:ident, $adv:ident, $dir:expr_2021) => {
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
            crate::MainViewState::Process(_) =>
                crate::process_view::ViewType::cp_warn($c, &msg),
            crate::MainViewState::System => crate::system_view::ViewType::cp_warn($c, &msg),
            #[cfg(fbcode_build)]
            crate::MainViewState::Gpu => crate::gpu_view::ViewType::cp_warn($c, &msg),
        }
    }};
}

// controllers depends on Advance
pub mod controllers;
pub mod viewrc;
// Jump popup depends on view_warn
mod jump_popup;

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessZoomState {
    NoZoom,
    Cgroup,
    Pids,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MainViewState {
    Cgroup,
    Process(ProcessZoomState),
    System,
    #[cfg(fbcode_build)]
    Gpu,
}

impl MainViewState {
    pub fn is_process_zoom_state(&self) -> bool {
        matches!(&self, &MainViewState::Process(zoom) if zoom != &ProcessZoomState::NoZoom)
    }
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
    summary_view::refresh(c);
    let current_state = c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .main_view_state
        .clone();
    match current_state {
        MainViewState::Cgroup => cgroup_view::CgroupView::refresh(c),
        MainViewState::Process(_) => process_view::ProcessView::refresh(c),
        MainViewState::System => system_view::SystemView::refresh(c),
        #[cfg(fbcode_build)]
        MainViewState::Gpu => gpu_view::GpuView::refresh(c),
    }
}

pub fn set_active_screen(c: &mut Cursive, name: &str) {
    let screen_id = *c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .main_view_screens
        .get(name)
        .unwrap_or_else(|| panic!("Failed to find screen id for {}", name));
    c.call_on_name(
        "main_view_screens",
        |screens: &mut NamedView<ScreensView>| {
            screens.get_mut().set_active_screen(screen_id);
            screens
                .get_mut()
                .screen_mut()
                .unwrap()
                .take_focus(cursive::direction::Direction::none())
                .ok();
        },
    )
    .expect("failed to find main_view_screens");
}

pub struct ViewState {
    pub time_elapsed: Duration,
    /// Keep track of the lowest seen `time_elapsed` so that view can highlight abnormal
    /// elapsed times. Below will never go faster than the requested interval rate but
    /// can certainly go higher (b/c of a loaded system or other delays).
    pub lowest_time_elapsed: Duration,
    pub timestamp: SystemTime,
    // TODO: Replace other fields with model
    pub model: Rc<RefCell<Model>>,
    pub system: Rc<RefCell<SystemModel>>,
    pub cgroup: Rc<RefCell<CgroupModel>>,
    pub process: Rc<RefCell<ProcessModel>>,
    pub network: Rc<RefCell<NetworkModel>>,
    #[cfg(fbcode_build)]
    pub gpu: Rc<RefCell<Option<GpuModel>>>,
    pub main_view_state: MainViewState,
    pub main_view_screens: HashMap<String, ScreenId>,
    pub mode: ViewMode,
    pub viewrc: ViewRc,
    pub viewrc_error: Option<String>,
    pub event_controllers: Rc<RefCell<HashMap<Event, controllers::Controllers>>>,
    pub cmd_controllers: Rc<RefCell<HashMap<&'static str, controllers::Controllers>>>,
}

impl ViewState {
    pub fn update(&mut self, model: Model) {
        self.time_elapsed = model.time_elapsed;
        if model.time_elapsed.as_secs() != 0 && model.time_elapsed < self.lowest_time_elapsed {
            self.lowest_time_elapsed = model.time_elapsed;
        }
        self.timestamp = model.timestamp;
        self.model.replace(model.clone());
        self.system.replace(model.system);
        self.cgroup.replace(model.cgroup);
        self.process.replace(model.process);
        self.network.replace(model.network);
        #[cfg(fbcode_build)]
        self.gpu.replace(model.gpu);
    }

    pub fn new_with_advance(
        main_view_state: MainViewState,
        model: Model,
        mode: ViewMode,
        viewrc: ViewRc,
        viewrc_error: Option<String>,
    ) -> Self {
        Self {
            time_elapsed: model.time_elapsed,
            lowest_time_elapsed: model.time_elapsed,
            timestamp: model.timestamp,
            model: Rc::new(RefCell::new(model.clone())),
            system: Rc::new(RefCell::new(model.system)),
            cgroup: Rc::new(RefCell::new(model.cgroup)),
            process: Rc::new(RefCell::new(model.process)),
            network: Rc::new(RefCell::new(model.network)),
            #[cfg(fbcode_build)]
            gpu: Rc::new(RefCell::new(model.gpu)),
            main_view_state,
            main_view_screens: HashMap::new(),
            mode,
            viewrc,
            viewrc_error,
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
        matches!(self.mode, ViewMode::Pause(_))
    }
}

impl View {
    pub fn new_with_advance(model: model::Model, mode: ViewMode) -> View {
        let mut inner = cursive::CursiveRunnable::new(|| {
            let backend = cursive::backends::crossterm::Backend::init().map(|backend| {
                Box::new(cursive_buffered_backend::BufferedBackend::new(backend))
                    as Box<(dyn cursive::backend::Backend)>
            });
            execute!(std::io::stdout(), DisableMouseCapture).expect("Failed to disable mouse.");
            backend
        });
        let (viewrc, viewrc_error) = viewrc::ViewRc::new();
        inner.set_user_data(ViewState::new_with_advance(
            MainViewState::Cgroup,
            model,
            mode,
            viewrc,
            viewrc_error,
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
        // Verify belowrc file format
        let cmdrc_opt = match std::fs::read_to_string(filename) {
            Ok(belowrc_str) => match belowrc_str.parse::<Value>() {
                Ok(belowrc) => belowrc
                    .get(get_belowrc_cmd_section_key())
                    .map(|cmdrc| cmdrc.to_owned()),
                Err(e) => {
                    view_warn!(c, "Failed to parse belowrc: {}", e);
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
            .add_global_callback(Event::CtrlChar('z'), |c| unsafe {
                use crossterm::cursor::Hide;
                use crossterm::cursor::Show;
                use crossterm::terminal::EnterAlternateScreen;
                use crossterm::terminal::LeaveAlternateScreen;

                // The following logic is necessary on crossterm as it does not
                // disable/re-enable tty on SIGTSTP, while ncurses does.

                // Reset tty to original mode
                execute!(std::io::stdout(), LeaveAlternateScreen, Show)
                    .expect("Failed to reset tty");
                crossterm::terminal::disable_raw_mode().expect("Failed to disable tty");

                // Send signal to put process to background
                if libc::raise(libc::SIGTSTP) != 0 {
                    panic!("failed to SIGTSTP self");
                }

                // Re-enable tty
                crossterm::terminal::enable_raw_mode().expect("Failed to enable tty");
                execute!(std::io::stdout(), EnterAlternateScreen, Hide)
                    .expect("Failed to setup tty");
                // Use WindowResize event to force redraw everything.
                c.on_event(Event::WindowResize);
            });
        self.inner.add_global_callback(Event::Refresh, |c| {
            refresh(c);
        });
        self.inner.add_global_callback(Event::CtrlChar('r'), |c| {
            c.clear();
            refresh(c);
        });

        // Used to handle warning assignment to the correct view
        let init_warnings = get_last_log_to_display();

        let status_bar = status_bar::new(&mut self.inner);
        let summary_view = summary_view::new(&mut self.inner);
        let cgroup_view = cgroup_view::CgroupView::new(&mut self.inner);
        let process_view = process_view::ProcessView::new(&mut self.inner);
        let system_view = system_view::SystemView::new(&mut self.inner);
        #[cfg(fbcode_build)]
        let gpu_view = gpu_view::GpuView::new(&mut self.inner);

        let mut screens_view = ScreensView::new();
        let main_view_screens = &mut self
            .inner
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .main_view_screens;
        main_view_screens.insert(
            "cgroup_view_panel".to_owned(),
            screens_view.add_screen(BoxedView::boxed(ResizedView::with_full_screen(cgroup_view))),
        );
        main_view_screens.insert(
            "process_view_panel".to_owned(),
            screens_view.add_screen(BoxedView::boxed(ResizedView::with_full_screen(
                process_view,
            ))),
        );
        main_view_screens.insert(
            "system_view_panel".to_owned(),
            screens_view.add_screen(BoxedView::boxed(ResizedView::with_full_screen(system_view))),
        );
        #[cfg(fbcode_build)]
        main_view_screens.insert(
            "gpu_view_panel".to_owned(),
            screens_view.add_screen(BoxedView::boxed(ResizedView::with_full_screen(gpu_view))),
        );

        self.inner
            .add_fullscreen_layer(ResizedView::with_full_screen(
                LinearLayout::vertical()
                    .child(Panel::new(status_bar))
                    .child(Panel::new(summary_view))
                    .child(
                        OnEventView::new(screens_view.with_name("main_view_screens"))
                            .with_name("dynamic_view"),
                    ),
            ));

        self.inner
            .focus_name("dynamic_view")
            .expect("Could not set focus at initialization!");

        // Set default view from viewrc
        if let Some(view) = self
            .inner
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .viewrc
            .default_view
            .clone()
        {
            let main_view_state = &mut self
                .inner
                .user_data::<ViewState>()
                .expect("No data stored in Cursive object!")
                .main_view_state;
            match view {
                viewrc::DefaultFrontView::Cgroup => {
                    *main_view_state = MainViewState::Cgroup;
                    set_active_screen(&mut self.inner, "cgroup_view_panel")
                }
                viewrc::DefaultFrontView::Process => {
                    *main_view_state = MainViewState::Process(ProcessZoomState::NoZoom);
                    set_active_screen(&mut self.inner, "process_view_panel")
                }
                viewrc::DefaultFrontView::System => {
                    *main_view_state = MainViewState::System;
                    set_active_screen(&mut self.inner, "system_view_panel")
                }
            }
        }

        // Raise warning message if failed to map the customized command.
        Self::generate_event_controller_map(&mut self.inner, get_belowrc_filename());
        if let Some(msg) = &self
            .inner
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .viewrc_error
        {
            let msg = msg.clone();
            let c = &mut self.inner;
            view_warn!(c, "{}", msg);
        }
        if let Some(msg) = init_warnings {
            let c = &mut self.inner;
            view_warn!(c, "{}", msg);
        }
        self.inner.run();

        Ok(())
    }
}

#[cfg(test)]
pub mod fake_view {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    use common::logutil::get_logger;
    use cursive::views::DummyView;
    use cursive::views::ViewRef;
    use model::Collector;
    use store::advance::new_advance_local;

    use self::viewrc::ViewRc;
    use super::*;
    use crate::MainViewState;
    use crate::ViewMode;
    use crate::ViewState;
    use crate::cgroup_view::CgroupView;
    use crate::command_palette::CommandPalette;
    use crate::stats_view::StatsView;

    pub struct FakeView {
        pub inner: CursiveRunnable,
    }

    #[allow(clippy::new_without_default)]
    impl FakeView {
        pub fn new() -> Self {
            let time = SystemTime::now();
            let logger = get_logger();
            let advance = new_advance_local(logger.clone(), PathBuf::new(), time);
            let mut collector = Collector::new(logger.clone(), Default::default());
            let model = collector
                .collect_and_update_model()
                .expect("Fail to get model");

            let mut inner = CursiveRunnable::dummy();
            let mut user_data = ViewState::new_with_advance(
                MainViewState::Cgroup,
                model,
                ViewMode::Live(Rc::new(RefCell::new(advance))),
                ViewRc::default(),
                None,
            );
            // Dummy screen to make switching panel no-op except state changes.
            inner.add_layer(
                ScreensView::single_screen(BoxedView::boxed(DummyView))
                    .with_name("main_view_screens"),
            );
            user_data.main_view_screens = [
                ("cgroup_view_panel".to_owned(), 0),
                ("process_view_panel".to_owned(), 0),
                ("system_view_panel".to_owned(), 0),
            ]
            .into();
            inner.set_user_data(user_data);

            Self { inner }
        }

        pub fn add_cgroup_view(&mut self) {
            let cgroup_view = CgroupView::new(&mut self.inner);
            self.inner.add_layer(cgroup_view);
        }

        pub fn get_cmd_palette(&mut self, name: &str) -> ViewRef<CommandPalette> {
            self.inner
                .find_name::<StatsView<CgroupView>>(name)
                .expect("Failed to dereference command palette")
                .get_cmd_palette()
        }
    }
}
