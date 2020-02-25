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

// Copyright (c) 2015 Alexandre Bury
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// This file is copied from GitHub
// gyscos/cursive/blob/master/src/views/select_view.rs so that we can change the
// way highlighted text is drawn. The original implementation has problem when
// terminal default is used as the view background color. All unused functions
// are removed to avoid compiler warnings.

use cursive::align::{Align, HAlign};
use cursive::direction::Direction;
use cursive::event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent};
use cursive::menu::MenuTree;
use cursive::theme::{ColorStyle, Effect, PaletteColor, Style};
use cursive::utils::markup::StyledString;
use cursive::view::{Position, View};
use cursive::views::MenuPopup;
use cursive::Cursive;
use cursive::Printer;
use cursive::Rect;
use cursive::Vec2;
use std::borrow::Borrow;
use std::cell::Cell;
use std::cmp::min;
use std::rc::Rc;

/// View to select an item among a list.
///
/// It contains a list of values of type T, with associated labels.
///
/// # Examples
///
/// ```rust
/// # use cursive::Cursive;
/// # use cursive::views::{SelectView, Dialog, TextView};
/// # use cursive::align::HAlign;
/// let mut time_select = SelectView::new().h_align(HAlign::Center);
/// time_select.add_item("Short", 1);
/// time_select.add_item("Medium", 5);
/// time_select.add_item("Long", 10);
///
/// time_select.set_on_submit(|s, time| {
///     s.pop_layer();
///     let text = format!("You will wait for {} minutes...", time);
///     s.add_layer(Dialog::around(TextView::new(text))
///                     .button("Quit", |s| s.quit()));
/// });
///
/// let mut siv = Cursive::dummy();
/// siv.add_layer(Dialog::around(time_select)
///                      .title("How long is your wait?"));
/// ```
pub struct SelectView<T = String> {
    // The core of the view: we store a list of items
    // `Item` is more or less a `(String, Rc<T>)`.
    items: Vec<Item<T>>,

    // When disabled, we cannot change selection.
    enabled: bool,

    // Callbacks may need to manipulate focus, so give it some mutability.
    focus: Rc<Cell<usize>>,

    // This is a custom callback to include a &T.
    // It will be called whenever "Enter" is pressed or when an item is clicked.
    on_submit: Option<Rc<dyn Fn(&mut Cursive, &T)>>,

    // This callback is called when the selection is changed.
    // TODO: add the previous selection? Indices?
    on_select: Option<Rc<dyn Fn(&mut Cursive, &T)>>,

    // If `true`, when a character is pressed, jump to the next item starting
    // with this character.
    autojump: bool,

    align: Align,

    // `true` if we show a one-line view, with popup on selection.
    popup: bool,

    // We need the last offset to place the popup window
    // We "cache" it during the draw, so we need interior mutability.
    last_offset: Cell<Vec2>,
    last_size: Vec2,
}

impl<T: 'static> Default for SelectView<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> SelectView<T> {
    /// Creates a new empty SelectView.
    pub fn new() -> Self {
        SelectView {
            items: Vec::new(),
            enabled: true,
            focus: Rc::new(Cell::new(0)),
            on_select: None,
            on_submit: None,
            align: Align::top_left(),
            popup: false,
            autojump: false,
            last_offset: Cell::new(Vec2::zero()),
            last_size: Vec2::zero(),
        }
    }

    /// Sets a callback to be used when `<Enter>` is pressed.
    ///
    /// Also happens if the user clicks an item.
    ///
    /// The item currently selected will be given to the callback.
    ///
    /// Here, `V` can be `T` itself, or a type that can be borrowed from `T`.
    pub fn set_on_submit<F, R, V: ?Sized>(&mut self, cb: F)
    where
        F: 'static + Fn(&mut Cursive, &V) -> R,
        T: Borrow<V>,
    {
        self.on_submit = Some(Rc::new(move |s, t| {
            cb(s, t.borrow());
        }));
    }

    /// Returns the value of the currently selected item.
    ///
    /// Returns `None` if the list is empty.
    pub fn selection(&self) -> Option<Rc<T>> {
        let focus = self.focus();
        if self.len() <= focus {
            None
        } else {
            Some(Rc::clone(&self.items[focus].value))
        }
    }

    /// Removes all items from this view.
    pub fn clear(&mut self) {
        self.items.clear();
        self.focus.set(0);
    }

    /// Adds a item to the list, with given label and value.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive::views::SelectView;
    ///
    /// let mut select_view = SelectView::new();
    ///
    /// select_view.add_item("Item 1", 1);
    /// select_view.add_item("Item 2", 2);
    /// ```
    pub fn add_item<S: Into<StyledString>>(&mut self, label: S, value: T) {
        self.items.push(Item::new(label.into(), value));
    }

    /// Iterate on the items in this view.
    ///
    /// Returns an iterator with each item and their labels.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        self.items
            .iter()
            .map(|item| (item.label.source(), &*item.value))
    }

    /// Adds all items from from an iterator.
    pub fn add_all<S, I>(&mut self, iter: I)
    where
        S: Into<StyledString>,
        I: IntoIterator<Item = (S, T)>,
    {
        for (s, t) in iter {
            self.add_item(s, t);
        }
    }

    fn draw_item(&self, printer: &Printer<'_, '_>, i: usize) {
        let l = self.items[i].label.width();
        let x = self.align.h.get_offset(l, printer.size.x);
        printer.print_hline((0, 0), x, " ");
        printer.print_styled((x, 0), (&self.items[i].label).into());
        if l < printer.size.x {
            assert!((l + x) <= printer.size.x);
            printer.print_hline((x + l, 0), printer.size.x - (l + x), " ");
        }
    }

    /// Returns the id of the item currently selected.
    ///
    /// Returns `None` if the list is empty.
    pub fn selected_id(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.focus())
        }
    }

    /// Returns the number of items in this list.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive::views::SelectView;
    ///
    /// let select_view = SelectView::new()
    ///     .item("Item 1", 1)
    ///     .item("Item 2", 2)
    ///     .item("Item 3", 3);
    ///
    /// assert_eq!(select_view.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if this list has no item.
    ///
    /// # Examples
    ///
    /// ```
    /// use cursive::views::SelectView;
    ///
    /// let mut select_view = SelectView::new();
    /// assert!(select_view.is_empty());
    ///
    /// select_view.add_item("Item 1", 1);
    /// select_view.add_item("Item 2", 2);
    /// assert!(!select_view.is_empty());
    ///
    /// select_view.clear();
    /// assert!(select_view.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn focus(&self) -> usize {
        self.focus.get()
    }

    /// Moves the selection to the given position.
    ///
    /// Returns a callback in response to the selection change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn set_selection(&mut self, i: usize) -> Callback {
        // TODO: Check if `i >= self.len()` ?
        // assert!(i < self.len(), "SelectView: trying to select out-of-bound");
        // Or just cap the ID?
        let i = if self.is_empty() {
            0
        } else {
            min(i, self.len() - 1)
        };
        self.focus.set(i);

        self.make_select_cb().unwrap_or_else(Callback::dummy)
    }

    /// Moves the selection down by the given number of rows.
    ///
    /// Returns a callback in response to the selection change.
    ///
    /// You should run this callback with a `&mut Cursive`.
    pub fn select_down(&mut self, n: usize) -> Callback {
        self.focus_down(n);
        self.make_select_cb().unwrap_or_else(Callback::dummy)
    }

    fn focus_up(&mut self, n: usize) {
        let focus = self.focus().saturating_sub(n);
        self.focus.set(focus);
    }

    fn focus_down(&mut self, n: usize) {
        let focus = min(self.focus() + n, self.items.len().saturating_sub(1));
        self.focus.set(focus);
    }

    fn submit(&mut self) -> EventResult {
        let cb = self.on_submit.clone().unwrap();
        // We return a Callback Rc<|s| cb(s, &*v)>
        EventResult::Consumed(
            self.selection()
                .map(|v| Callback::from_fn(move |s| cb(s, &v))),
        )
    }

    fn on_char_event(&mut self, c: char) -> EventResult {
        let i = {
            // * Starting from the current focus, find the first item that
            //   match the char.
            // * Cycle back to the beginning of the list when we reach the end.
            // * This is achieved by chaining twice the iterator.
            let iter = self.iter().chain(self.iter());

            // We'll do a lowercase check.
            let lower_c: Vec<char> = c.to_lowercase().collect();
            let lower_c: &[char] = &lower_c;

            if let Some((i, _)) = iter
                .enumerate()
                .skip(self.focus() + 1)
                .find(|&(_, (label, _))| label.to_lowercase().starts_with(lower_c))
            {
                i % self.len()
            } else {
                return EventResult::Ignored;
            }
        };

        self.focus.set(i);
        // Apply modulo in case we have a hit from the chained iterator
        let cb = self.set_selection(i);
        EventResult::Consumed(Some(cb))
    }

    fn on_event_regular(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Up) if self.focus() > 0 => self.focus_up(1),
            Event::Key(Key::Down) if self.focus() + 1 < self.items.len() => self.focus_down(1),
            Event::Key(Key::PageUp) => self.focus_up(10),
            Event::Key(Key::PageDown) => self.focus_down(10),
            Event::Key(Key::Home) => self.focus.set(0),
            Event::Key(Key::End) => self.focus.set(self.items.len().saturating_sub(1)),
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } if position
                .checked_sub(offset)
                .map(|position| position < self.last_size && position.y < self.len())
                .unwrap_or(false) =>
            {
                self.focus.set(position.y - offset.y)
            }
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if self.on_submit.is_some()
                && position
                    .checked_sub(offset)
                    .map(|position| position < self.last_size && position.y == self.focus())
                    .unwrap_or(false) =>
            {
                return self.submit();
            }
            Event::Key(Key::Enter) if self.on_submit.is_some() => {
                return self.submit();
            }
            Event::Char(c) if self.autojump => return self.on_char_event(c),
            _ => return EventResult::Ignored,
        }

        EventResult::Consumed(self.make_select_cb())
    }

    /// Returns a callback from selection change.
    fn make_select_cb(&self) -> Option<Callback> {
        self.on_select.clone().and_then(|cb| {
            self.selection()
                .map(|v| Callback::from_fn(move |s| cb(s, &v)))
        })
    }

    fn open_popup(&mut self) -> EventResult {
        // Build a shallow menu tree to mimick the items array.
        // TODO: cache it?
        let mut tree = MenuTree::new();
        for (i, item) in self.items.iter().enumerate() {
            let focus = Rc::clone(&self.focus);
            let on_submit = self.on_submit.as_ref().cloned();
            let value = Rc::clone(&item.value);
            tree.add_leaf(item.label.source(), move |s| {
                // TODO: What if an item was removed in the meantime?
                focus.set(i);
                if let Some(ref on_submit) = on_submit {
                    on_submit(s, &value);
                }
            });
        }
        // Let's keep the tree around,
        // the callback will want to use it.
        let tree = Rc::new(tree);

        let focus = self.focus();
        // This is the offset for the label text.
        // We'll want to show the popup so that the text matches.
        // It'll be soo cool.
        let item_length = self.items[focus].label.width();
        let text_offset = (self.last_size.x.saturating_sub(item_length)) / 2;
        // The total offset for the window is:
        // * the last absolute offset at which we drew this view
        // * shifted to the right of the text offset
        // * shifted to the top of the focus (so the line matches)
        // * shifted top-left of the border+padding of the popup
        let offset = self.last_offset.get();
        let offset = offset + (text_offset, 0);
        let offset = offset.saturating_sub((0, focus));
        let offset = offset.saturating_sub((2, 1));

        // And now, we can return the callback that will create the popup.
        EventResult::with_cb(move |s| {
            // The callback will want to work with a fresh Rc
            let tree = Rc::clone(&tree);
            // We'll relativise the absolute position,
            // So that we are locked to the parent view.
            // A nice effect is that window resizes will keep both
            // layers together.
            let current_offset = s.screen().offset();
            let offset = offset.signed() - current_offset;
            // And finally, put the view in view!
            s.screen_mut()
                .add_layer_at(Position::parent(offset), MenuPopup::new(tree).focus(focus));
        })
    }

    // A popup view only does one thing: open the popup on Enter.
    fn on_event_popup(&mut self, event: Event) -> EventResult {
        match event {
            // TODO: add Left/Right support for quick-switch?
            Event::Key(Key::Enter) => self.open_popup(),
            Event::Mouse {
                event: MouseEvent::Release(MouseButton::Left),
                position,
                offset,
            } if position.fits_in_rect(offset, self.last_size) => self.open_popup(),
            _ => EventResult::Ignored,
        }
    }
}

impl SelectView<String> {
    /// Convenient method to use the label as value.
    pub fn add_item_str<S: Into<String>>(&mut self, label: S) {
        let label = label.into();
        self.add_item(label.clone(), label);
    }

    /// Adds all strings from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cursive::views::SelectView;
    /// let mut select_view = SelectView::new();
    /// select_view.add_all_str(vec!["a", "b", "c"]);
    /// ```
    pub fn add_all_str<S, I>(&mut self, iter: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = S>,
    {
        for s in iter {
            self.add_item_str(s);
        }
    }
}

impl<T: 'static> View for SelectView<T> {
    fn draw(&self, printer: &Printer<'_, '_>) {
        self.last_offset.set(printer.offset);

        if self.popup {
            // Popup-select only draw the active element.
            // We'll draw the full list in a popup if needed.
            let style = if !(self.enabled && printer.enabled) {
                ColorStyle::secondary()
            } else if printer.focused {
                ColorStyle::highlight()
            } else {
                ColorStyle::primary()
            };
            let x = match printer.size.x.checked_sub(1) {
                Some(x) => x,
                None => return,
            };

            printer.with_color(style, |printer| {
                // Prepare the entire background
                printer.print_hline((1, 0), x, " ");
                // Draw the borders
                printer.print((0, 0), "<");
                printer.print((x, 0), ">");

                let label = &self.items[self.focus()].label;

                // And center the text?
                let offset = HAlign::Center.get_offset(label.width(), x + 1);

                printer.print_styled((offset, 0), label.into());
            });
        } else {
            // Non-popup mode: we always print the entire list.
            let h = self.items.len();
            let offset = self.align.v.get_offset(h, printer.size.y);
            let printer = &printer.offset((0, offset));

            for i in 0..self.len() {
                let style = if i == self.focus() {
                    Style::merge(&[
                        Effect::Reverse.into(),
                        ColorStyle {
                            front: if printer.focused {
                                PaletteColor::Highlight.into()
                            } else {
                                PaletteColor::HighlightInactive.into()
                            },
                            back: PaletteColor::View.into(),
                        }
                        .into(),
                    ])
                } else if !(self.enabled && printer.enabled) {
                    ColorStyle::secondary().into()
                } else {
                    ColorStyle::primary().into()
                };
                printer.offset((0, i)).with_style(style, |printer| {
                    self.draw_item(printer, i);
                });
            }
        }
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        // Items here are not compressible.
        // So no matter what the horizontal requirements are,
        // we'll still return our longest item.
        let w = self
            .items
            .iter()
            .map(|item| item.label.width())
            .max()
            .unwrap_or(1);
        if self.popup {
            Vec2::new(w + 2, 1)
        } else {
            let h = self.items.len();

            Vec2::new(w, h)
        }
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.popup {
            self.on_event_popup(event)
        } else {
            self.on_event_regular(event)
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled && !self.items.is_empty()
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
    }

    fn important_area(&self, size: Vec2) -> Rect {
        self.selected_id()
            .map(|i| Rect::from_size((0, i), (size.x, 1)))
            .unwrap_or_else(|| Rect::from((0, 0)))
    }
}

// We wrap each value in a `Rc` and add a label
struct Item<T> {
    label: StyledString,
    value: Rc<T>,
}

impl<T> Item<T> {
    fn new(label: StyledString, value: T) -> Self {
        let value = Rc::new(value);
        Item { label, value }
    }
}
