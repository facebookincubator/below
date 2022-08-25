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
use cursive::view::Nameable;
use cursive::view::View;
use cursive::views::Dialog;
use cursive::views::EditView;
use cursive::views::OnEventView;
use cursive::Cursive;

use crate::stats_view::StateCommon;
use crate::MainViewState;

// Set command palette filter
// field_info includes the column title (string formatted for display) and filter
fn set_cp_filter(c: &mut Cursive, field_info: Option<(String, String)>) {
    let state = c
        .user_data::<crate::ViewState>()
        .expect("No user data")
        .main_view_state
        .clone();
    match state {
        MainViewState::Cgroup => crate::cgroup_view::ViewType::cp_filter(c, field_info),
        MainViewState::Process(_) => crate::process_view::ViewType::cp_filter(c, field_info),
        MainViewState::Core => crate::core_view::ViewType::cp_filter(c, field_info),
        #[cfg(fbcode_build)]
        MainViewState::Gpu => crate::gpu_view::ViewType::cp_filter(c, field_info),
    }
}

pub fn new<F>(
    state: Rc<RefCell<impl StateCommon + 'static>>,
    refresh: F,
    tab: String,
    idx: usize,
    title_name: String,
) -> impl View
where
    F: 'static + Copy + Fn(&mut Cursive),
{
    fn set_filter_state_and_cp(
        c: &mut Cursive,
        state: Rc<RefCell<impl StateCommon + 'static>>,
        text: &str,
        tab: &str,
        idx: usize,
        title_name: &String,
    ) {
        if text.is_empty() {
            state.borrow_mut().set_filter_from_tab_idx("", 0, None);
            set_cp_filter(c, None);
        } else {
            state
                .borrow_mut()
                .set_filter_from_tab_idx(tab, idx, Some(text.to_string()));
            set_cp_filter(c, Some((title_name.to_string(), text.to_string())));
        }
    }

    // scope function vars
    let submit_state = state.clone();
    let submit_tab = tab.clone();
    let submit_title_name = title_name.clone();
    let mut editview = EditView::new()
        // Run cb and close popup when user presses "Enter"
        .on_submit(move |c, text| {
            set_filter_state_and_cp(
                c,
                submit_state.clone(),
                text,
                &submit_tab,
                idx,
                &submit_title_name,
            );
            refresh(c);
            c.pop_layer();
        });

    editview.set_content(match state.borrow_mut().get_filter_info().as_ref() {
        None => String::new(),
        Some((_, filter)) => filter.to_string(),
    });

    OnEventView::new(
        Dialog::new()
            .title(format!("Filter by {}", title_name))
            .padding_lrtb(1, 1, 1, 0)
            .content(editview.with_name("filter_popup"))
            .dismiss_button("Close")
            .button("Filter", move |c| {
                let text = c
                    .call_on_name("filter_popup", |view: &mut EditView| view.get_content())
                    .expect("Unable to find filter_popup");
                set_filter_state_and_cp(c, state.clone(), &text, &tab, idx, &title_name);
                refresh(c);

                // Pop dialog
                c.pop_layer();
            }),
    )
    .on_event(Key::Esc, |s| {
        s.pop_layer();
    })
}
