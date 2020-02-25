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
use cursive::views::{LinearLayout, TextView};
use cursive::Cursive;

use super::util::convert_bytes;
use crate::model::{CpuModel, IoModel, MemoryModel};
use crate::view::ViewState;

fn get_cpu_row(cpu: &Option<CpuModel>) -> String {
    format!(
        "{:6.6}\t{:7.7}{:10.10}\t{:7.7}{:10.10}\t{:7.7}{:10.10}",
        "Cpu",
        "Usage",
        cpu.as_ref().map_or("?".to_string(), |cpu| {
            cpu.usage_pct
                .map_or("?".to_string(), |usage| format!("{:.2}%", usage))
        }),
        "User",
        cpu.as_ref().map_or("?".to_string(), |cpu| {
            cpu.user_pct
                .map_or("?".to_string(), |user| format!("{:.2}%", user))
        }),
        "System",
        cpu.as_ref().map_or("?".to_string(), |cpu| {
            cpu.system_pct
                .map_or("?".to_string(), |sys| format!("{:.2}%", sys))
        }),
    )
}

fn get_mem_row(mem: &Option<MemoryModel>) -> String {
    format!(
        "{:6.6}\t{:7.7}{:10.10}\t{:7.7}{:10.10}\t{:7.7}{:10.10}\t{:7.7}{:10.10}",
        "Mem",
        "Total",
        mem.as_ref().map_or("?".to_string(), |mem| {
            mem.total
                .map_or("?".to_string(), |total| convert_bytes(total as f64))
        }),
        "Free",
        mem.as_ref().map_or("?".to_string(), |mem| {
            mem.free
                .map_or("?".to_string(), |free| convert_bytes(free as f64))
        }),
        "Anon",
        mem.as_ref().map_or("?".to_string(), |mem| {
            mem.anon
                .map_or("?".to_string(), |anon| convert_bytes(anon as f64))
        }),
        "File",
        mem.as_ref().map_or("?".to_string(), |mem| {
            mem.file
                .map_or("?".to_string(), |file| convert_bytes(file as f64))
        }),
    )
}

fn get_io_row(io: &Option<IoModel>) -> String {
    format!(
        "{:6.6}\t{:7.7}{:10.10}\t{:7.7}{:10.10}",
        "Io",
        "R/sec",
        io.as_ref().map_or("?".to_string(), |io| {
            io.rbytes_per_sec
                .map_or("?".to_string(), |r| convert_bytes(r))
        }),
        "W/sec",
        io.as_ref().map_or("?".to_string(), |io| {
            io.wbytes_per_sec
                .map_or("?".to_string(), |w| convert_bytes(w))
        }),
    )
}

fn fill_content(c: &mut Cursive, v: &mut LinearLayout) {
    let model = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .model;

    let system_model = &model.system;
    let cpu_row = get_cpu_row(&system_model.cpu);
    let mem_row = get_mem_row(&system_model.mem);
    let io_row = get_io_row(&system_model.io);

    let mut view = LinearLayout::vertical();
    view.add_child(TextView::new(cpu_row));
    view.add_child(TextView::new(mem_row));
    view.add_child(TextView::new(io_row));

    std::mem::replace(v, view);
}

pub fn refresh(c: &mut Cursive) {
    let mut v = c
        .find_name::<LinearLayout>("system_view")
        .expect("No system_view view found!");

    fill_content(c, &mut v);
}

pub fn new(c: &mut Cursive) -> impl View {
    let mut view = LinearLayout::vertical();
    fill_content(c, &mut view);
    view.with_name("system_view")
}
