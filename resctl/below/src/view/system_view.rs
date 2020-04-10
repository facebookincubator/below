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

use crate::model::{CpuModel, IoModel, MemoryModel};
use crate::view::ViewState;

fn get_cpu_row(cpu: &Option<CpuModel>) -> String {
    format!(
        "{:6.6}\t{}",
        "Cpu",
        cpu.as_ref()
            .unwrap_or(&Default::default())
            .get_interleave_line("", "\t")
    )
}

fn get_mem_row(mem: &Option<MemoryModel>) -> String {
    format!(
        "{:6.6}\t{}",
        "Mem",
        mem.as_ref()
            .unwrap_or(&Default::default())
            .get_interleave_line("", "\t")
    )
}

fn get_io_row(io: &Option<IoModel>) -> String {
    format!(
        "{:6.6}\t{}",
        "Io",
        io.as_ref()
            .unwrap_or(&Default::default())
            .get_interleave_line("", "\t")
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
