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

use cursive::view::{Identifiable, View};
use cursive::views::{Dialog, EditView};
use cursive::Cursive;

pub fn new<F>(initial_content: &str, cb: F) -> impl View
where
    F: 'static + Copy + Fn(&mut Cursive, &str),
{
    let mut editview = EditView::new()
        // Run cb and close popup when user presses "Enter"
        .on_submit(move |c, text| {
            cb(c, text);
            c.pop_layer();
        });

    editview.set_content(initial_content);

    Dialog::new()
        .title("Filter by name")
        .padding_lrtb(1, 1, 1, 0)
        .content(editview.with_name("filter_popup"))
        .dismiss_button("Close")
        .button("Filter", move |c| {
            let text = c
                .call_on_name("filter_popup", |view: &mut EditView| view.get_content())
                .expect("Unable to find filter_popup");

            // Update state
            cb(c, &text);

            // Pop dialog
            c.pop_layer();
        })
}
