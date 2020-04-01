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
use cursive::views::{LinearLayout, ResizedView, SelectView, TextView};
use cursive::Cursive;

use std::cmp::Ordering;
use std::iter::FromIterator;

use super::util::{convert_bytes, get_header, Field};
use crate::model;
use crate::view::{SortOrder, ViewState};

fn get_pid_rows(view_state: &ViewState) -> Vec<Vec<Field>> {
    let unknown = "?".to_string();
    let mut processes = Vec::from_iter(&view_state.model.process.processes);

    match view_state.sort_order {
        SortOrder::CPU => {
            let sum_cpu = |v: &model::SingleProcessModel| {
                v.cpu.as_ref().map_or(0.0, |o| {
                    o.user_pct.map_or(0.0, |p| p) + o.system_pct.map_or(0.0, |p| p)
                })
            };
            processes.sort_by(|lhs, rhs| {
                sum_cpu(&rhs.1)
                    .partial_cmp(&sum_cpu(&lhs.1))
                    .unwrap_or(Ordering::Equal)
            });
        }
        SortOrder::Memory => {
            let sum_bytes = |v: &model::SingleProcessModel| {
                v.mem.as_ref().map_or(0, |o| o.rss_bytes.map_or(0, |b| b))
            };
            processes.sort_by(|lhs, rhs| {
                sum_bytes(&rhs.1)
                    .partial_cmp(&sum_bytes(&lhs.1))
                    .unwrap_or(Ordering::Equal)
            });
        }
        SortOrder::Disk => {
            let sum_bytes = |v: &model::SingleProcessModel| {
                v.io.as_ref().map_or(0.0, |o| {
                    o.rbytes_per_sec.map_or(0.0, |b| b) + o.wbytes_per_sec.map_or(0.0, |b| b)
                })
            };
            processes.sort_by(|lhs, rhs| {
                sum_bytes(&rhs.1)
                    .partial_cmp(&sum_bytes(&lhs.1))
                    .unwrap_or(Ordering::Equal)
            });
        }
        SortOrder::Name => {
            processes.sort_by(
                |lhs, rhs| match (lhs.1.comm.as_ref(), rhs.1.comm.as_ref()) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    _ => Ordering::Equal,
                },
            );
        }
        SortOrder::PID => {}
    }
    processes
        .iter()
        .map(|(pid, spm)| {
            let mut row: Vec<Field> = Vec::new();
            row.push(Field {
                name: format!("{:12.12}", "Comm"),
                value: format!("{:12.12}", spm.comm.as_ref().unwrap_or(&unknown)),
            });
            row.push(Field {
                name: format!("{:50.50}", "Cgroup"),
                value: format!("{:50.50}", spm.cgroup.as_ref().unwrap_or(&unknown)),
            });
            row.push(Field {
                name: format!("{:11.11}", "Pid"),
                value: format!("{:11.11}", pid.to_string()),
            });
            row.push(Field {
                name: format!("{:11.11}", "State"),
                value: format!("{:11.11}", get_inner_or_default!(spm.state, unknown)),
            });
            row.push(Field {
                name: format!("{:11.11}", "CPU"),
                value: format!(
                    "{:11.11}",
                    spm.cpu.as_ref().map_or_else(
                        || unknown.clone(),
                        |cpu| {
                            if let (Some(u), Some(s)) = (cpu.user_pct, cpu.system_pct) {
                                return format!("{:.2}%", (u + s));
                            }

                            unknown.clone()
                        }
                    )
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "User CPU"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.cpu, unknown, user_pct, |i| format!("{:.2}%", i))
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "Sys CPU"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.cpu, unknown, system_pct, |i| format!("{:.2}%", i))
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "RSS"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.mem, unknown, rss_bytes, |n| {
                        convert_bytes(n as f64)
                    })
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "Minflt/sec"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.mem, unknown, minorfaults_per_sec, |i| format!(
                        "{:.2}",
                        i
                    ))
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "Majflt/sec"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.mem, unknown, majorfaults_per_sec, |i| format!(
                        "{:.2}",
                        i
                    ))
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "Reads/sec"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.io, unknown, rbytes_per_sec, |i| convert_bytes(
                        i as f64
                    ))
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "Writes/sec"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.io, unknown, wbytes_per_sec, |i| convert_bytes(
                        i as f64
                    ))
                ),
            });
            row.push(Field {
                name: format!("{:11.11}", "Uptime(sec)"),
                value: format!("{:11.11}", get_inner_or_default!(spm.uptime_secs, unknown)),
            });
            row.push(Field {
                name: format!("{:11.11}", "Threads"),
                value: format!(
                    "{:11.11}",
                    get_inner_or_default!(spm.cpu, unknown, num_threads)
                ),
            });

            row
        })
        .collect()
}

fn fill_content(c: &mut Cursive, v: &mut SelectView) {
    let view_state = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    let pos = v.selected_id().unwrap_or(0);
    v.clear();

    v.add_all_str(get_pid_rows(view_state).iter().map(|row| {
        let mut content = String::new();
        for field in row {
            content.push_str(&field.value);
            content.push(' ');
        }

        content
    }));

    v.select_down(pos)(c);
}

pub fn refresh(c: &mut Cursive) {
    let mut v = c
        .find_name::<SelectView>("process_view")
        .expect("No process_view view found!");

    fill_content(c, &mut v);
}

pub fn new(c: &mut Cursive) -> impl View {
    let mut list = SelectView::new();
    fill_content(c, &mut list);

    let header: String;
    {
        let view_state = &c
            .user_data::<ViewState>()
            .expect("No data stored in Cursive object!");

        let rows = get_pid_rows(view_state);
        header = get_header(&rows);
    }

    LinearLayout::vertical()
        .child(TextView::new(header).effect(Effect::Bold))
        .child(ResizedView::with_full_screen(
            list.with_name("process_view").scrollable(),
        ))
        .scrollable()
        .scroll_x(true)
        .scroll_y(false)
}
