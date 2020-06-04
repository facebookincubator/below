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

use std::collections::HashSet;
use std::iter::FromIterator;

use cursive::theme::Effect;
use cursive::view::{Identifiable, Scrollable, View};
use cursive::views::{LinearLayout, OnEventView, ResizedView, SelectView, TextView};
use cursive::Cursive;

use crate::model;
use crate::model::CgroupModel;
use crate::util::{fold_string, get_prefix};
use crate::view::{SortOrder, ViewState};
use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
struct CgroupView {
    #[blink("CgroupModel$get_name")]
    #[bttr(
        title = "Name",
        width = 50,
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)",
        decorator = "fold_string(&$, 50 - self.depth * 3, 0, |c: char| !char::is_alphanumeric(c))"
    )]
    pub name: String,
    #[blink("CgroupModel$cpu?.get_usage_pct")]
    #[bttr(cmp = true)]
    pub cpu_usage_pct: Option<f64>,
    #[blink("CgroupModel$memory?.get_total")]
    #[bttr(cmp = true)]
    pub memory_total: Option<u64>,
    #[blink("CgroupModel$pressure?.get_cpu_some_pct")]
    pub pressure_cpu_some_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_memory_full_pct")]
    pub pressure_memory_full_pct: Option<f64>,
    #[blink("CgroupModel$pressure?.get_io_full_pct")]
    pub pressure_io_full_pct: Option<f64>,
    #[blink("CgroupModel$io_total?.get_rbytes_per_sec")]
    pub io_total_rbytes_per_sec: Option<f64>,
    #[blink("CgroupModel$io_total?.get_wbytes_per_sec")]
    pub io_total_wbytes_per_sec: Option<f64>,
    #[bttr(
        aggr = "CgroupModel: io_total?.rbytes_per_sec? + io_total?.wbytes_per_sec?",
        cmp = true
    )]
    pub disk: Option<f64>,
    depth: usize,
    collapse: bool,
}

impl CgroupView {
    fn new(depth: usize, collapse: bool) -> Self {
        Self {
            depth,
            collapse,
            ..Default::default()
        }
    }
}

fn get_header() -> String {
    let cm: CgroupModel = Default::default();
    let cv = CgroupView::new(0, true);
    cv.get_title_line(&cm)
}

/// Returns a set of full cgroup paths that should be filtered out.
///
/// Note that this algorithm recursively whitelists parents of cgroups that are
/// whitelisted. The reason for this is because cgroups are inherently tree-like
/// and displaying a lone cgroup without its ancestors doesn't make much sense.
fn calculate_filter_out_set(cgroup: &model::CgroupModel, filter: &str) -> HashSet<String> {
    fn should_filter_out(
        cgroup: &model::CgroupModel,
        filter: &str,
        set: &mut HashSet<String>,
    ) -> bool {
        // No children
        if cgroup.count == 1 {
            if !cgroup.full_path.contains(filter) {
                set.insert(cgroup.full_path.clone());
                return true;
            }
            return false;
        }

        let mut filter_cgroup = true;
        for child in &cgroup.children {
            if should_filter_out(&child, &filter, set) {
                set.insert(child.full_path.clone());
            } else {
                // We found a child that's not filtered out. That means
                // we have to keep this (the parent cgroup) too.
                filter_cgroup = false;
            }
        }

        if filter_cgroup {
            set.insert(cgroup.full_path.clone());
        }

        filter_cgroup
    }

    let mut set = HashSet::new();
    should_filter_out(&cgroup, &filter, &mut set);
    set
}

fn get_cgroup_rows(view_state: &ViewState) -> Vec<(String, String)> {
    fn output_cgroup(
        cgroup: &model::CgroupModel,
        sort_order: SortOrder,
        collapsed_cgroups: &HashSet<String>,
        filter_out_set: &Option<HashSet<String>>,
        output: &mut Vec<(String, String)>,
    ) {
        if let Some(set) = &filter_out_set {
            if set.contains(&cgroup.full_path) {
                return;
            }
        }

        let collapsed = collapsed_cgroups.contains(&cgroup.full_path);
        let cv = CgroupView::new(cgroup.depth as usize, collapsed);
        let row = cv.get_field_line(&cgroup, &cgroup);
        // Each row is (label, value), where label is visible and value is used
        // as identifier to correlate the row with its state in global data.
        output.push((row, cgroup.full_path.clone()));
        if collapsed {
            return;
        }

        let mut children = Vec::from_iter(&cgroup.children);

        // Here we map the sort order to an index (or for disk, do some custom sorting)
        match sort_order {
            SortOrder::CPU => children.sort_by(|lhs, rhs| {
                CgroupView::cmp_by_cpu_usage_pct(&lhs, &rhs)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse()
            }),
            SortOrder::Memory => children.sort_by(|lhs, rhs| {
                CgroupView::cmp_by_memory_total(&lhs, &rhs)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse()
            }),
            SortOrder::Disk => children.sort_by(|lhs, rhs| {
                CgroupView::cmp_by_disk(&lhs, &rhs)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse()
            }),
            _ => (),
        };

        for child_cgroup in &children {
            output_cgroup(
                child_cgroup,
                sort_order,
                collapsed_cgroups,
                &filter_out_set,
                output,
            );
        }
    };

    let filter_out_set = if let Some(f) = &view_state.cgroup_filter {
        Some(calculate_filter_out_set(&view_state.model.cgroup, &f))
    } else {
        None
    };

    let mut rows = Vec::new();
    output_cgroup(
        &view_state.model.cgroup,
        view_state.sort_order,
        &view_state.collapsed_cgroups,
        &filter_out_set,
        &mut rows,
    );
    rows
}

fn fill_content(c: &mut Cursive, v: &mut SelectView) {
    let view_state = &mut c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    let pos = v.selected_id().unwrap_or(0);
    v.clear();

    v.add_all(get_cgroup_rows(view_state));
    v.select_down(pos)(c);
}

pub fn refresh(c: &mut Cursive) {
    let mut v = c
        .find_name::<SelectView>("cgroup_view")
        .expect("No cgroup_view view found!");

    fill_content(c, &mut v);
}

#[allow(unused)]
fn submit_filter(c: &mut Cursive, text: &str) {
    let view_state = &mut c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    view_state.cgroup_filter = if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    };

    refresh(c);
}

pub fn new(c: &mut Cursive) -> impl View {
    let mut list = SelectView::new();
    fill_content(c, &mut list);
    list.set_on_submit(|c, cgroup: &String| {
        let view_state = &mut c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!");
        if view_state.collapsed_cgroups.contains(cgroup) {
            view_state.collapsed_cgroups.remove(cgroup);
        } else {
            view_state.collapsed_cgroups.insert(cgroup.to_string());
        }
        refresh(c);
    });

    list.set_on_select(|c, cgroup: &String| {
        let view_state = &mut c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!");
        view_state.current_selected_cgroup = cgroup.clone();
    });

    let header = get_header();

    OnEventView::new(
        LinearLayout::vertical()
            .child(TextView::new(header).effect(Effect::Bold))
            .child(ResizedView::with_full_screen(
                list.with_name("cgroup_view").scrollable(),
            ))
            .scrollable()
            .scroll_x(true)
            .scroll_y(false),
    )
    .on_event('/', |c| {
        let _initial_content = match &c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .cgroup_filter
        {
            Some(s) => s.clone(),
            None => "".to_string(),
        };

        // c.add_layer(filter_popup::new(initial_content.as_str(), submit_filter));
    })
}
