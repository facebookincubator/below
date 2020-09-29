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

use super::*;
use crate::filter_popup;

// Sort by selected column
make_event_controller!(
    SortByColumn,
    "sort",
    Event::Char('S'),
    |view: &mut StatsView<T>, _cmd_vec: &[&str]| {
        let tab_view = view.get_tab_view();
        let tab = tab_view.get_cur_selected();
        let title_view = view.get_title_view();
        let title_idx = title_view.current_selected;
        let title = title_view.get_cur_selected().to_string();
        let sort_res = view
            .state
            .borrow_mut()
            .set_sort_tag(tab, title_idx, view.reverse_sort);

        if sort_res {
            view.reverse_sort = !view.reverse_sort;
        } else {
            view.get_cmd_palette()
                .set_alert(&format!("\"{}\" is not sortable currently.", title.trim()));
        }
    },
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        StatsView::<T>::refresh_myself(c);
    }
);

// Trigger filter popup
make_event_controller!(
    FilterPopup,
    "filter",
    Event::Char('/'),
    |_view: &mut StatsView<T>, _cmd_vec: &[&str]| {},
    |c: &mut Cursive, _cmd_vec: &[&str]| {
        let state = StatsView::<T>::get_view(c).state.clone();
        c.add_layer(filter_popup::new(state, StatsView::<T>::refresh_myself));
    }
);
