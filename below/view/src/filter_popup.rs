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

use std::cell::RefCell;
use std::rc::Rc;

use cursive::event::Key;
use cursive::view::{Identifiable, View};
use cursive::views::{Dialog, EditView, OnEventView};
use cursive::Cursive;

use crate::stats_view::StateCommon;
use crate::MainViewState;

// Set command palette filter
fn set_cp_filter(c: &mut Cursive, text: Option<String>) {
    let state = c
        .user_data::<crate::ViewState>()
        .expect("No user data")
        .main_view_state
        .clone();
    match state {
        MainViewState::Cgroup => crate::cgroup_view::ViewType::cp_filter(c, text),
        MainViewState::Process | MainViewState::ProcessZoomedIntoCgroup => {
            crate::process_view::ViewType::cp_filter(c, text)
        }
        MainViewState::Core => crate::core_view::ViewType::cp_filter(c, text),
        MainViewState::Gpu => crate::gpu_view::ViewType::cp_filter(c, text),
    }
}

pub fn new<F>(state: Rc<RefCell<impl StateCommon + 'static>>, refresh: F) -> impl View
where
    F: 'static + Copy + Fn(&mut Cursive),
{
    let submit_state = state.clone();
    let mut editview = EditView::new()
        // Run cb and close popup when user presses "Enter"
        .on_submit(move |c, text| {
            if text.is_empty() {
                *submit_state.borrow_mut().get_filter() = None;
                set_cp_filter(c, None);
            } else {
                *submit_state.borrow_mut().get_filter() = Some(text.to_string());
                set_cp_filter(c, Some(text.to_string()));
            }
            refresh(c);
            c.pop_layer();
        });

    editview.set_content(
        state
            .borrow_mut()
            .get_filter()
            .as_ref()
            .unwrap_or(&"".to_string()),
    );

    OnEventView::new(
        Dialog::new()
            .title("Filter by name")
            .padding_lrtb(1, 1, 1, 0)
            .content(editview.with_name("filter_popup"))
            .dismiss_button("Close")
            .button("Filter", move |c| {
                let text = c
                    .call_on_name("filter_popup", |view: &mut EditView| view.get_content())
                    .expect("Unable to find filter_popup");

                if text.is_empty() {
                    *state.borrow_mut().get_filter() = None;
                    set_cp_filter(c, None);
                } else {
                    *state.borrow_mut().get_filter() = Some(text.to_string());
                    set_cp_filter(c, Some(text.to_string()));
                }

                refresh(c);

                // Pop dialog
                c.pop_layer();
            }),
    )
    .on_event(Key::Esc, |s| {
        s.pop_layer();
    })
}
