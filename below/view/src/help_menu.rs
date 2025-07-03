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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use cursive::event::Event;
use cursive::view::Nameable;
use cursive::view::Scrollable;
use cursive::view::View;
use cursive::views::LinearLayout;
use cursive::views::Panel;
use cursive::views::SelectView;
use cursive::views::TextView;

use crate::controllers::Controllers;
use crate::controllers::event_to_string;
use crate::tab_view::TabView;

pub struct ControllerHelper {
    events: Vec<Event>,
    description: &'static str,
    cmd: &'static str,
    cmd_short: &'static str,
    args: &'static str,
}

impl std::fmt::Display for ControllerHelper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:<24} {:<24} {:<11} {:<10} {}",
            self.cmd,
            &gen_hotkey_string(&self.events),
            if self.cmd_short.is_empty() {
                "-"
            } else {
                self.cmd_short
            },
            self.args,
            self.description
        )
    }
}

fn gen_hotkey_string(events: &[Event]) -> String {
    events
        .iter()
        .map(event_to_string)
        .collect::<Vec<String>>()
        .join(",")
}

fn get_description(controller: &Controllers) -> &'static str {
    match controller {
        Controllers::CmdPalette => "Invoking command palette.",
        Controllers::NextTab => "Cycle topic tabs.",
        Controllers::PrevTab => "Reverse cycle topic tabs.",
        Controllers::NextCol => "Cycle columns.",
        Controllers::PrevCol => "Reverse cycle columns.",
        Controllers::Right => "Scroll right primary display.",
        Controllers::Left => "Scroll left primary display.",
        Controllers::SortCol => {
            "Sort by the selected title, reverse the result by hitting 'S' again(cgroup view and process view only)."
        }
        Controllers::Filter => "Filter by selected column.",
        Controllers::CFilter => "Clear the current filter.",
        Controllers::JForward => {
            "Jump time by a specific amount forward or to a specific timestamp (replay and live-paused mode)."
        }
        Controllers::JBackward => {
            "Jump time by a specific amount backward or to a specific timestamp (replay and live-paused mode)."
        }
        Controllers::NSample => "Show next sample (replay and live-paused mode).",
        Controllers::PSample => "Show previous sample (replay and live-paused mode).",
        Controllers::Pause => {
            "pause/resume the live mode. While pausing, use the above commands to go forwards or backwards in time"
        }
        Controllers::Quit => "Quit.",
        Controllers::Help => "Toggle help menu.",
        Controllers::Process => "Show process view.",
        Controllers::Cgroup => "Show cgroup view.",
        Controllers::System => "Show system view.",
        Controllers::Gpu => "Show GPU view.",
        Controllers::GpuZoom => "Zoom into process view filtered by selected GPU.",
        Controllers::GpuProcess => "Zoom into process view for all GPU processes.",
        Controllers::Zoom => {
            "If in cgroup view, zoom into process view filtered by cgroup. If in process view, zoom into cgroup view, selected on cgroup of process."
        }
        Controllers::Fold => "Fold processes (post filter) and display aggregated values.",
        Controllers::NextPage => "Scroll down 15 lines primary display.",
        Controllers::PrevPage => "Scroll up 15 lines primary display.",
        _ => "Unknown",
    }
}

fn get_args(controller: &Controllers) -> &'static str {
    match controller {
        Controllers::SortCol => "SortKey",
        Controllers::Filter => "Substring",
        Controllers::JForward => "Time",
        Controllers::JBackward => "Time",
        _ => "-",
    }
}

fn get_title() -> Vec<String> {
    vec![
        format!("{:<24}", "Command"),
        format!("{:<24}", "Hot Key"),
        format!("{:<11}", "Cmd Alias"),
        format!("{:<10}", "Args"),
        "Description".into(),
    ]
}

// Grab the user customized keymaps and generate helper message
fn fill_controllers(
    v: &mut SelectView<String>,
    event_controllers: Arc<Mutex<HashMap<Event, Controllers>>>,
) {
    // event_controllers can generate helper messages in completely random order base on
    // user's customization. Instead of using it directly, we will generate a cmd-msg map
    // to ensure the order.
    //
    let mut cmd_map: HashMap<Controllers, ControllerHelper> = HashMap::new();
    for (event, controller) in event_controllers.lock().unwrap().iter() {
        match cmd_map.get_mut(controller) {
            Some(ref mut item) => item.events.push(event.clone()),
            None => drop(cmd_map.insert(
                controller.clone(),
                ControllerHelper {
                    events: vec![event.clone()],
                    cmd: controller.command(),
                    cmd_short: controller.cmd_shortcut(),
                    description: get_description(controller),
                    args: get_args(controller),
                },
            )),
        }
    }

    // Unwrap in this vec! must be success, otherwise we may have lost
    // controller(s) and should be detected by unit test.
    let mut controllers = vec![
        cmd_map.get(&Controllers::Help).unwrap().to_string(),
        cmd_map.get(&Controllers::CmdPalette).unwrap().to_string(),
        cmd_map.get(&Controllers::Quit).unwrap().to_string(),
        cmd_map.get(&Controllers::Left).unwrap().to_string(),
        cmd_map.get(&Controllers::Right).unwrap().to_string(),
        cmd_map.get(&Controllers::NextTab).unwrap().to_string(),
        cmd_map.get(&Controllers::PrevTab).unwrap().to_string(),
        cmd_map.get(&Controllers::NextCol).unwrap().to_string(),
        cmd_map.get(&Controllers::PrevCol).unwrap().to_string(),
        cmd_map.get(&Controllers::JForward).unwrap().to_string(),
        cmd_map.get(&Controllers::JBackward).unwrap().to_string(),
        cmd_map.get(&Controllers::NSample).unwrap().to_string(),
        cmd_map.get(&Controllers::PSample).unwrap().to_string(),
        cmd_map.get(&Controllers::Pause).unwrap().to_string(),
        cmd_map.get(&Controllers::SortCol).unwrap().to_string(),
        cmd_map.get(&Controllers::Filter).unwrap().to_string(),
        cmd_map.get(&Controllers::CFilter).unwrap().to_string(),
        cmd_map.get(&Controllers::Zoom).unwrap().to_string(),
        cmd_map.get(&Controllers::Fold).unwrap().to_string(),
        cmd_map.get(&Controllers::Process).unwrap().to_string(),
        cmd_map.get(&Controllers::Cgroup).unwrap().to_string(),
        cmd_map.get(&Controllers::System).unwrap().to_string(),
        cmd_map.get(&Controllers::NextPage).unwrap().to_string(),
        cmd_map.get(&Controllers::PrevPage).unwrap().to_string(),
    ];

    controllers.extend(crate::get_extra_controller_str(&cmd_map));

    v.add_all_str(controllers);
}

fn fill_reserved(v: &mut LinearLayout) {
    let lines = vec![
        " <DOWN>         - scroll down primary display, next command if command palette activated\n",
        " <UP>           - scroll up primary display, last command if command palette activated\n",
        " <PgDn>         - scroll down 15 lines primary display\n",
        " <PgUp>         - scroll up 15 lines primary display\n",
        " <Home>         - scroll to top of primary display\n",
        " <End>          - scroll to end of primary display\n",
        " <Enter>        - collapse/expand cgroup tree, submit command if command palette activated\n",
        " '='            - collapse immediate children of selected cgroup\n",
        " <Ctrl>-r       - refresh the screen",
        " 'P'            - sort by pid (process view only)\n",
        " 'N'            - sort by name (process view only)\n",
        " 'C'            - sort by cpu (cgroup view and process view only)\n",
        " 'M'            - sort by memory (cgroup view and process view only)\n",
        " 'D'            - sort by total disk activity(cgroup view and process view only)\n",
    ];

    for line in lines {
        v.add_child(TextView::new(line));
    }
}

pub fn new(event_controllers: Arc<Mutex<HashMap<Event, Controllers>>>) -> impl View {
    let mut reserved = LinearLayout::vertical();
    fill_reserved(&mut reserved);
    let mut controllers = SelectView::<String>::new();
    fill_controllers(&mut controllers, event_controllers);
    LinearLayout::vertical()
        .child(Panel::new(reserved))
        .child(Panel::new(
            LinearLayout::vertical()
                .child(
                    TabView::new(get_title(), " ", 0 /* pinned titles */)
                        .expect("Failed to construct title tab in help menu"),
                )
                .child(controllers)
                .scrollable()
                .scroll_x(true),
        ))
        .with_name("help_menu")
}
