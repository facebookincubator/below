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

use cursive::theme::ColorStyle;
use cursive::vec::Vec2;
use cursive::Printer;
use cursive::View;

/// Command palette will have different mode:
/// Info is used to show info like full cgroup path.
/// Alert is used to show error messages.
// TODO: command mode for command palette.
#[derive(PartialEq)]
enum CPMode {
    Info,
    Alert,
}

/// TextView that used to display extra information
///
/// Currently, we will use command palette to display extra information like
/// full cgroup name. But the idea for this view is something like vim's command palette
/// that use for input operation command like search, filter, rearrange, apply config, etc.
pub struct CommandPalette {
    content: String,
    filter: Option<String>,
    mode: CPMode,
}

impl View for CommandPalette {
    fn draw(&self, printer: &Printer) {
        printer.print_hline((0, 0), printer.size.x, "─");
        if let Some(filter) = &self.filter {
            let filter = format!("| Filter: {:>10.10} |", filter);
            printer.print((printer.size.x - filter.len(), 0), &filter);
        }

        // Message should adapt the screen size
        let mut msg_len_left = self.content.len();
        let mut idx = 0;
        let mut line = 1;
        while msg_len_left > printer.size.x {
            self.print(printer, (0, line), idx);
            msg_len_left -= printer.size.x;
            idx += printer.size.x;
            line += 1;
        }
        self.print(printer, (0, line), idx);
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        Vec2::new(1, self.content.len() / constraint.x + 2)
    }
}

impl CommandPalette {
    /// Create a new CommandPalette
    pub fn new<T: Into<String>>(content: T) -> Self {
        Self {
            content: content.into(),
            filter: None,
            mode: CPMode::Info,
        }
    }

    /// Set the display info
    pub fn set_info<T: Into<String>>(&mut self, content: T) {
        self.content = content.into();
        self.mode = CPMode::Info;
    }

    /// Set alert
    /// This will preempt the command palette mode.
    pub fn set_alert<T: Into<String>>(&mut self, content: T) {
        self.content = content.into();
        self.mode = CPMode::Alert;
    }

    pub fn set_filter(&mut self, filter: Option<String>) {
        self.filter = filter;
    }

    fn print_info(&self, printer: &Printer, pos: Vec2, idx: usize) {
        if idx + printer.size.x > self.content.len() {
            printer.print(pos, &self.content[idx..]);
        } else {
            printer.print(pos, &self.content[idx..idx + printer.size.x]);
        }
    }

    fn print_alert(&self, printer: &Printer, pos: Vec2, idx: usize) {
        printer.with_color(ColorStyle::title_primary(), |printer| {
            if idx + printer.size.x > self.content.len() {
                printer.print(pos, &self.content[idx..]);
            } else {
                printer.print(pos, &self.content[idx..idx + printer.size.x]);
            }
        })
    }

    fn print<T: Into<Vec2>>(&self, printer: &Printer, pos: T, idx: usize) {
        match self.mode {
            CPMode::Info => self.print_info(printer, pos.into(), idx),
            CPMode::Alert => self.print_alert(printer, pos.into(), idx),
        }
    }
}
