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
use cursive::view::{Identifiable, Scrollable, View};
use cursive::views::{LinearLayout, OnEventView, ResizedView, SelectView, TextView};
use cursive::Cursive;

use std::iter::FromIterator;

use crate::model::SingleProcessModel;
use crate::view::{MainViewState, SortOrder, ViewState};
use below_derive::BelowDecor;

#[derive(BelowDecor, Default)]
struct ProcessView {
    #[blink("SingleProcessModel$get_comm")]
    #[bttr(cmp = true)]
    pub comm: Option<String>,
    #[blink("SingleProcessModel$get_cgroup")]
    pub cgroup: Option<String>,
    #[blink("SingleProcessModel$get_pid")]
    pub pid: Option<i32>,
    #[blink("SingleProcessModel$get_state")]
    pub state: Option<procfs::PidState>,
    #[bttr(
        title = "CPU",
        width = 11,
        precision = 2,
        unit = "%",
        aggr = "SingleProcessModel: cpu?.user_pct? + cpu?.system_pct?",
        cmp = true
    )]
    pub cpu: Option<f64>,
    #[blink("SingleProcessModel$cpu?.get_user_pct")]
    pub cpu_user_pct: Option<f64>,
    #[blink("SingleProcessModel$cpu?.get_system_pct")]
    pub cpu_system_pct: Option<f64>,
    #[blink("SingleProcessModel$mem?.get_rss_bytes")]
    #[bttr(cmp = true)]
    pub mem_rss_bytes: Option<u64>,
    #[blink("SingleProcessModel$mem?.get_minorfaults_per_sec")]
    pub mem_minorfaults_per_sec: Option<f64>,
    #[blink("SingleProcessModel$mem?.get_majorfaults_per_sec")]
    pub mem_majorfaults_per_sec: Option<f64>,
    #[blink("SingleProcessModel$io?.get_rbytes_per_sec")]
    pub io_rbytes_per_sec: Option<f64>,
    #[blink("SingleProcessModel$io?.get_wbytes_per_sec")]
    pub io_wbytes_per_sec: Option<f64>,
    #[blink("SingleProcessModel$get_uptime_secs")]
    pub uptime_secs: Option<u64>,
    #[blink("SingleProcessModel$cpu?.get_num_threads")]
    pub cpu_num_threads: Option<u64>,
    #[bttr(
        aggr = "SingleProcessModel: io?.rbytes_per_sec? + io?.wbytes_per_sec?",
        cmp = true
    )]
    pub disk: Option<f64>,
}

fn get_header() -> String {
    let spm: SingleProcessModel = Default::default();
    let pv: ProcessView = Default::default();
    pv.get_title_line(&spm)
}

fn get_pid_rows(view_state: &ViewState) -> Vec<String> {
    let unknown = "?".to_string();
    let mut processes = Vec::from_iter(&view_state.model.process.processes);

    match view_state.sort_order {
        SortOrder::CPU => processes.sort_by(|lhs, rhs| {
            ProcessView::cmp_by_cpu(&lhs.1, &rhs.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .reverse()
        }),
        SortOrder::Memory => processes.sort_by(|lhs, rhs| {
            ProcessView::cmp_by_mem_rss_bytes(&lhs.1, &rhs.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .reverse()
        }),
        SortOrder::Disk => processes.sort_by(|lhs, rhs| {
            ProcessView::cmp_by_disk(&lhs.1, &rhs.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .reverse()
        }),
        SortOrder::Name => processes.sort_by(|lhs, rhs| {
            ProcessView::cmp_by_comm(&lhs.1, &rhs.1).unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortOrder::PID => (),
    }
    let pv: ProcessView = Default::default();
    processes
        .iter()
        .filter(|(_, spm)| {
            // If we're in zoomed cgroup mode, only show processes belonging to
            // our zoomed cgroup
            match &view_state.main_view_state {
                MainViewState::ProcessZoomedIntoCgroup(c) => {
                    spm.cgroup.as_ref().unwrap_or(&unknown).starts_with(c)
                }
                _ => true,
            }
        })
        .filter(|(_, spm)| {
            // If we're filtering by name, only show processes who pass the filter
            if let Some(f) = &view_state.process_filter {
                spm.comm.as_ref().unwrap_or(&unknown).contains(f)
            } else {
                true
            }
        })
        .map(|(_, spm)| pv.get_field_line(&spm, &spm))
        .collect()
}

fn fill_content(c: &mut Cursive, v: &mut SelectView) {
    let view_state = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    let pos = v.selected_id().unwrap_or(0);
    v.clear();

    v.add_all_str(get_pid_rows(view_state));

    v.select_down(pos)(c);
}

pub fn refresh(c: &mut Cursive) {
    let mut v = c
        .find_name::<SelectView>("process_view")
        .expect("No process_view view found!");

    fill_content(c, &mut v);
}

#[allow(unused)]
fn submit_filter(c: &mut Cursive, text: &str) {
    let view_state = &mut c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    view_state.process_filter = if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    };

    refresh(c);
}

pub fn new(c: &mut Cursive) -> impl View {
    let mut list = SelectView::new();
    fill_content(c, &mut list);

    let header = get_header();

    OnEventView::new(
        LinearLayout::vertical()
            .child(TextView::new(header).effect(Effect::Bold))
            .child(ResizedView::with_full_screen(
                list.with_name("process_view").scrollable(),
            ))
            .scrollable()
            .scroll_x(true)
            .scroll_y(false),
    )
    .on_event('/', |c| {
        let _initial_content = match &c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!")
            .process_filter
        {
            Some(s) => s.clone(),
            None => "".to_string(),
        };

        // c.add_layer(filter_popup::new(initial_content.as_str(), submit_filter));
    })
}
