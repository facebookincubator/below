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

use cursive::theme::Effect;
use cursive::vec::Vec2;
use cursive::Printer;
use cursive::View;

use anyhow::{bail, Result};

/// TextView that has a vector of string as tabs
///
/// TabView will not implement any event. Instead, it will provide handler functions
/// on how the view should change on specific event like tab and shift-tab. cur_length and
/// total_length is used to calculate the selection offset. We use the selection offset to
/// automatically horizontal scroll the tab view.
pub struct TabView {
    pub tabs: Vec<String>,
    pub current_selected: usize,
    pub current_offset_idx: usize,
    separator: String,
    pub cur_length: usize,
    pub cur_offset: usize,
    pub total_length: usize,
}

impl View for TabView {
    fn draw(&self, printer: &Printer) {
        let mut current_offset = 0;
        let mut current_offset_idx = self.current_offset_idx;
        for idx in 0..self.tabs.len() {
            // Skip the hide item
            if idx != 0 && current_offset_idx > 0 {
                current_offset_idx -= 1;
                continue;
            }

            let content = self.tabs[idx].to_string();

            if idx == self.current_selected {
                let trimed = &content.trim_end();
                printer.with_effects(Effect::Bold | Effect::Underline, |printer| {
                    printer.print((current_offset, 0), trimed);
                });
                printer.print_hline(
                    (current_offset + trimed.len(), 0),
                    content.len() - trimed.len(),
                    " ",
                );
            } else {
                printer.print((current_offset, 0), &content);
            }

            current_offset += content.len();
            printer.print((current_offset, 0), &self.separator);
            current_offset += self.separator.len();
        }
        printer.print_hline((0, 1), printer.size.x, "─");
    }

    fn required_size(&mut self, _constraint: Vec2) -> Vec2 {
        Vec2::new(1, 2)
    }
}

impl TabView {
    /// Create a new TabView
    pub fn new(tabs: Vec<String>, sep: &str) -> Result<Self> {
        if tabs.is_empty() {
            bail!("Fail to construct TabView with empty tabs");
        }

        // The default general here is necessary, otherwise TextView will
        // not show anyting when we set the value.
        let total_length = tabs.iter().fold(0, |acc, x| acc + x.len() + 1);
        let cur_length = tabs[0].len();

        Ok(Self {
            tabs,
            current_selected: 0,
            current_offset_idx: 0,
            separator: sep.into(),
            cur_length,
            cur_offset: 0,
            total_length,
        })
    }

    /// Get current selected string.
    pub fn get_cur_selected(&self) -> &String {
        &self.tabs[self.current_selected]
    }

    /// Forward selection handler.
    pub fn on_tab(&mut self) -> usize {
        self.current_selected += 1;
        self.current_selected %= self.tabs.len();
        if self.current_selected == 0 {
            self.cur_length = self.tabs[0].len();
        } else {
            self.cur_length += self.get_cur_selected().len() + 1;
        }
        self.cur_length
    }

    /// Backward selection handler.
    pub fn on_shift_tab(&mut self) -> usize {
        if self.current_selected == 0 {
            self.current_selected = self.tabs.len() - 1;
            self.cur_length = self.total_length;
        } else {
            self.cur_length -= self.get_cur_selected().len() + 1;
            self.current_selected -= 1;
        }
        self.cur_length
    }

    /// Right scrolling offset
    pub fn on_right(&mut self, screen_size: usize) -> usize {
        if self.total_length - self.cur_offset <= screen_size {
            return self.cur_offset;
        }
        self.current_offset_idx += 1;
        self.cur_offset += self.tabs[self.current_offset_idx].len() + 1;
        self.cur_offset
    }

    /// Left scrolling offset
    pub fn on_left(&mut self) -> usize {
        if self.current_offset_idx == 0 {
            return 0;
        }

        self.cur_offset -= self.tabs[self.current_offset_idx].len() + 1;
        self.current_offset_idx -= 1;
        self.cur_offset
    }

    /// Scroll to the target offset and automatically fill the length gap
    pub fn scroll_to_offset(&mut self, offset: usize, screen_size: usize) -> usize {
        self.cur_offset = 0;
        self.current_offset_idx = 0;
        if offset == 0 {
            return 0;
        }

        while self.total_length - self.cur_offset > screen_size && self.cur_offset < offset {
            self.on_right(screen_size);
        }

        self.cur_offset
    }
}
