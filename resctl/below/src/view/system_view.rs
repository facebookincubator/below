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

use crate::model::{CpuModel, IoModel, MemoryModel, NetworkModel};
use crate::view::ViewState;

use below_derive::BelowDecor;

// Generate the get row function.
// We have to use a macro here since BelowDecor is not a trait, so we do not
// have access to the `get_interleave_line` function in the trait definition.
macro_rules! gen_row_impl {
    ($struct_type:ident, $model_type:ident, $title:expr) => {
        impl $struct_type {
            fn get_default() -> Self {
                Default::default()
            }

            fn get_row(model: &$model_type) -> String {
                format!(
                    "{:8.8}\t{}",
                    $title,
                    Self::get_default().get_interleave_line("", "\t", model)
                )
            }
        }
    };
}

#[derive(BelowDecor, Default)]
struct SysCpu {
    #[blink("CpuModel$get_usage_pct")]
    pub usage_pct: Option<f64>,
    #[blink("CpuModel$get_user_pct")]
    pub user_pct: Option<f64>,
    #[blink("CpuModel$get_system_pct")]
    pub sys_pct: Option<f64>,
}

gen_row_impl!(SysCpu, CpuModel, "CPU");

#[derive(BelowDecor, Default)]
struct SysMem {
    #[blink("MemoryModel$get_total")]
    pub total: Option<u64>,
    #[blink("MemoryModel$get_free")]
    pub free: Option<u64>,
    #[blink("MemoryModel$get_anon")]
    pub anon: Option<u64>,
    #[blink("MemoryModel$get_file")]
    pub file: Option<u64>,
}

gen_row_impl!(SysMem, MemoryModel, "Mem");

#[derive(BelowDecor, Default)]
struct SysIo {
    #[blink("IoModel$get_rbytes_per_sec")]
    pub rbytes_per_sec: Option<f64>,
    #[blink("IoModel$get_wbytes_per_sec")]
    pub wbytes_per_sec: Option<f64>,
}

gen_row_impl!(SysIo, IoModel, "I/O");

struct SysIface;

impl SysIface {
    fn get_row(net: &NetworkModel) -> String {
        let mut network = format!("{:8.8}\t", "Iface");

        net.interfaces.iter().for_each(|(interface, snm)| {
            network.push_str(&format!(
                "{:7.7}{:<10.10}\t",
                interface,
                format!("{}/s", snm.get_throughput_per_sec_str())
            ))
        });
        network
    }
}

fn fill_content(c: &mut Cursive, v: &mut LinearLayout) {
    let model = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .model;

    let system_model = &model.system;
    let cpu_row = SysCpu::get_row(system_model.cpu.as_ref().unwrap_or(&Default::default()));
    let mem_row = SysMem::get_row(system_model.mem.as_ref().unwrap_or(&Default::default()));
    let io_row = SysIo::get_row(system_model.io.as_ref().unwrap_or(&Default::default()));
    let iface_row = SysIface::get_row(&model.network);

    let mut view = LinearLayout::vertical();
    view.add_child(TextView::new(cpu_row));
    view.add_child(TextView::new(mem_row));
    view.add_child(TextView::new(io_row));
    view.add_child(TextView::new(iface_row));

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
