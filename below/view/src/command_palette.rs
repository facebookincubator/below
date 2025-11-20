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
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use cursive::Cursive;
use cursive::Printer;
use cursive::View;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::event::Key;
use cursive::theme::ColorStyle;
use cursive::vec::Vec2;
use cursive::views::EditView;
use cursive::views::NamedView;

use crate::controllers::Controllers;
use crate::stats_view::StatsView;
use crate::stats_view::ViewBridge;

const MAX_CMD_HISTORY: usize = 10;

/// Command palette will have different mode:
/// Info is used to show info like full cgroup path.
/// Alert is used to show error messages.
/// Command is used to turn command palette in Command mode.
// TODO: command mode for command palette.
#[derive(PartialEq)]
enum CPMode {
    Info,
    Alert,
    Command,
}

/// TextView that used to display extra information
///
/// Currently, we will use command palette to display extra information like
/// full cgroup name. But the idea for this view is something like vim's command palette
/// that use for input operation command like search, filter, rearrange, apply config, etc.
pub struct CommandPalette {
    content: String,
    filter_info: Option<(String, String)>,
    fold: bool,
    mode: CPMode,
    cmd_view: Arc<Mutex<EditView>>,
    cmd_controllers: Arc<Mutex<HashMap<&'static str, Controllers>>>,
    cmd_history: VecDeque<String>,
    cur_cmd_idx: usize,
}

impl View for CommandPalette {
    fn draw(&self, printer: &Printer) {
        // Right most X position that contains text
        let mut max_x = printer.size.x;

        printer.print_hline((0, 0), printer.size.x, "─");
        if let Some((field, filter)) = &self.filter_info {
            let output = format!(
                "|| Filtered Column: {:>10.10} | Filter: {:>10.10} ||",
                field, filter
            );
            max_x -= output.len();
            printer.print((max_x, 0), &output);
        }

        if self.fold {
            let text = "| Fold |";
            max_x -= text.len();
            printer.print((max_x, 0), text);
        }

        match self.mode {
            CPMode::Command => {
                printer.print((0, 1), ":");
                let inner_printer = printer.offset((1, 1));
                self.cmd_view.lock().unwrap().layout(inner_printer.size);
                self.cmd_view.lock().unwrap().draw(&inner_printer);
            }
            _ => {
                // Split content by newlines first, then wrap each line by screen width
                let mut line = 1;
                for content_line in self.content.lines() {
                    let mut msg_len_left = content_line.len();
                    let mut idx = 0;
                    while msg_len_left > printer.size.x {
                        self.print_line(printer, (0, line), content_line, idx, printer.size.x);
                        msg_len_left -= printer.size.x;
                        idx += printer.size.x;
                        line += 1;
                    }
                    self.print_line(printer, (0, line), content_line, idx, msg_len_left);
                    line += 1;
                }
            }
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Up) => {
                self.prev_cmd();
                EventResult::Consumed(None)
            }
            Event::Key(Key::Down) => {
                self.next_cmd();
                EventResult::Consumed(None)
            }
            _ => self.cmd_view.lock().unwrap().on_event(event),
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        // Count actual lines after splitting by newlines, plus wrapping within each line
        let mut total_lines = 1; // Start with 1 for the horizontal separator
        for content_line in self.content.lines() {
            // Calculate how many wrapped lines this content line needs
            let line_len = content_line.len();
            if line_len == 0 {
                total_lines += 1;
            } else {
                total_lines += line_len.div_ceil(constraint.x);
            }
        }
        Vec2::new(1, total_lines)
    }
}

impl CommandPalette {
    /// Create a new CommandPalette
    pub fn new<V: 'static + ViewBridge>(
        name: &'static str,
        content: &str,
        cmd_controllers: Arc<Mutex<HashMap<&'static str, Controllers>>>,
    ) -> Self {
        Self {
            content: content.into(),
            filter_info: None,
            fold: false,
            mode: CPMode::Info,
            cmd_view: Arc::new(Mutex::new(
                EditView::new()
                    .on_submit(move |c, cmd| {
                        Self::handle_cmd_history(name, c, cmd);
                        Self::run_cmd::<V>(name, c, cmd)
                    })
                    .style(ColorStyle::terminal_default()),
            )),
            cmd_controllers,
            cmd_history: VecDeque::new(),
            cur_cmd_idx: 0,
        }
    }

    fn handle_cmd_history(name: &'static str, c: &mut Cursive, cmd: &str) {
        c.call_on_name(
            &format!("{}_cmd_palette", name),
            |cp: &mut NamedView<CommandPalette>| {
                let mut cmd_palette = cp.get_mut();
                cmd_palette.cmd_history.push_back(cmd.into());
                if cmd_palette.cmd_history.len() > MAX_CMD_HISTORY {
                    cmd_palette.cmd_history.pop_front();
                }
                cmd_palette.cur_cmd_idx = cmd_palette.cmd_history.len() - 1;
            },
        );
    }

    fn prev_cmd(&mut self) {
        if self.cmd_history.is_empty() {
            return;
        }
        self.cmd_view
            .lock()
            .unwrap()
            .set_content(&self.cmd_history[self.cur_cmd_idx]);
        if self.cur_cmd_idx > 0 {
            self.cur_cmd_idx -= 1;
        }
    }

    fn next_cmd(&mut self) {
        if self.cmd_history.is_empty() {
            return;
        }
        if self.cur_cmd_idx == self.cmd_history.len() - 1 {
            self.cmd_view.lock().unwrap().set_content("");
        } else {
            self.cur_cmd_idx += 1;
            self.cmd_view
                .lock()
                .unwrap()
                .set_content(&self.cmd_history[self.cur_cmd_idx]);
        }
    }

    /// Run the captured command
    // In this function, we should avoid borrowing command palette object, since
    // it will cause a double mut borrow in the handler.
    pub fn run_cmd<V: 'static + ViewBridge>(name: &'static str, c: &mut Cursive, cmd: &str) {
        let cmd_vec = cmd.trim().split(' ').collect::<Vec<&str>>();
        let controller = c
            .find_name::<Self>(&format!("{}_cmd_palette", name))
            .expect("Fail to get cmd_palette")
            .cmd_controllers
            .lock()
            .unwrap()
            .get(cmd_vec[0])
            .unwrap_or(&Controllers::Unknown)
            .clone();

        match controller {
            Controllers::Unknown => {
                let mut cp = c
                    .find_name::<Self>(&format!("{}_cmd_palette", name))
                    .expect("Fail to get cmd_palette");
                cp.mode = CPMode::Alert;
                cp.content = "Unknown Command".into();
                cp.cmd_view.lock().unwrap().set_content("");
            }
            _ => {
                controller.handle(&mut StatsView::<V>::get_view(c), &cmd_vec);
                controller.callback::<V>(c, &cmd_vec);
                c.call_on_name(
                    &format!("{}_cmd_palette", name),
                    |cp: &mut NamedView<CommandPalette>| {
                        cp.get_mut().reset_cmd();
                    },
                );
            }
        }
    }

    pub fn reset_cmd(&mut self) {
        self.mode = CPMode::Info;
        self.cmd_view.lock().unwrap().set_content("");
    }

    /// Turn cmd_palette into command input mode
    pub fn invoke_cmd(&mut self) {
        self.mode = CPMode::Command;
        self.content = "".into()
    }

    /// Check if command palette is in command mode
    pub fn is_cmd_mode(&self) -> bool {
        self.mode == CPMode::Command
    }

    /// Set the display info
    pub fn set_info<T: Into<String>>(&mut self, content: T) {
        self.content = content.into();
        if self.mode != CPMode::Command {
            self.mode = CPMode::Info;
        }
    }

    /// Set alert
    /// This will preempt the command palette mode.
    pub fn set_alert<T: Into<String>>(&mut self, content: T) {
        if self.mode == CPMode::Alert {
            // Attach to current alert if it is not consumed.
            self.content = format!("{} |=| {}", self.content, content.into());
        } else {
            self.content = content.into();
            if self.mode != CPMode::Command {
                self.mode = CPMode::Alert;
            }
        }
    }

    pub fn set_filter(&mut self, filter_info: Option<(String, String)>) {
        self.filter_info = filter_info;
    }

    pub fn toggle_fold(&mut self) {
        self.fold = !self.fold;
    }

    fn print_line<T: Into<Vec2>>(
        &self,
        printer: &Printer,
        pos: T,
        line: &str,
        start: usize,
        len: usize,
    ) {
        let end = std::cmp::min(start + len, line.len());
        match self.mode {
            CPMode::Info => printer.print(pos, &line[start..end]),
            CPMode::Alert => printer.with_color(ColorStyle::title_primary(), |printer| {
                printer.print(pos, &line[start..end]);
            }),
            _ => {}
        }
    }

    pub fn is_alerting(&self) -> bool {
        self.mode == CPMode::Alert
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }
}
