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

use cursive::vec::Vec2;
use cursive::Printer;
use cursive::View;

/// TextView that used to display extra information
///
/// Currently, we will use command palette to display extra information like
/// full cgroup name. But the idea for this view is something like vim's command palette
/// that use for input operation command like search, filter, rearrange, apply config, etc.
pub struct CommandPalette {
    info: String,
}

impl View for CommandPalette {
    fn draw(&self, printer: &Printer) {
        printer.print_hline((0, 0), printer.size.x, "─");
        printer.print((0, 1), &self.info);
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        Vec2::new(1, 2)
    }
}

impl CommandPalette {
    /// Create a new CommandPalette
    pub fn new<T: Into<String>>(info: T) -> Self {
        Self { info: info.into() }
    }

    /// Set the display info
    pub fn set_info<T: Into<String>>(&mut self, info: T) {
        self.info = info.into();
    }
}
