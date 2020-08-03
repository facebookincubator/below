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
use std::collections::BTreeMap;

use cursive::utils::markup::StyledString;
use cursive::view::{Identifiable, View};
use cursive::views::{LinearLayout, TextView};
use cursive::Cursive;

use crate::model::{CpuModel, MemoryModel, NetworkModel, SingleDiskModel};
use crate::util::convert_bytes;
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

            // Styled strings don't handle \t's well. Probably b/c it registers
            // the \t as taking up one cell but we expect it to expand to 8
            // cells. So instead we use a bunch of spaces.
            fn get_row(model: &$model_type) -> StyledString {
                let mut row = StyledString::new();
                row.append(format!("{:8.8}{:7.7}", $title, ""));
                for line in Self::get_default().get_interleave_line("", model) {
                    row.append(line);
                    row.append(format!("{:7.7}", ""));
                }

                row
            }
        }
    };
}

#[derive(BelowDecor, Default)]
struct SysCpu {
    #[bttr(
        title = "Usage",
        unit = "%",
        width = 10,
        title_width = 7,
        precision = 2
    )]
    #[blink("CpuModel$total_cpu?.get_usage_pct")]
    pub usage_pct: Option<f64>,
    #[bttr(title = "User", unit = "%", width = 10, title_width = 7, precision = 2)]
    #[blink("CpuModel$total_cpu?.get_user_pct")]
    pub user_pct: Option<f64>,
    #[bttr(
        title = "System",
        unit = "%",
        width = 10,
        title_width = 7,
        precision = 2
    )]
    #[blink("CpuModel$total_cpu?.get_system_pct")]
    pub sys_pct: Option<f64>,
}

gen_row_impl!(SysCpu, CpuModel, "CPU");

#[derive(BelowDecor, Default)]
struct SysMem {
    #[bttr(
        title = "Total",
        decorator = "convert_bytes($ as f64)",
        width = 10,
        title_width = 7
    )]
    #[blink("MemoryModel$get_total")]
    pub total: Option<u64>,
    #[bttr(
        title = "Free",
        decorator = "convert_bytes($ as f64)",
        width = 10,
        title_width = 7
    )]
    #[blink("MemoryModel$get_free")]
    pub free: Option<u64>,
    #[bttr(
        title = "Anon",
        decorator = "convert_bytes($ as f64)",
        width = 10,
        title_width = 7
    )]
    #[blink("MemoryModel$get_anon")]
    pub anon: Option<u64>,
    #[bttr(
        title = "File",
        decorator = "convert_bytes($ as f64)",
        width = 10,
        title_width = 7
    )]
    #[blink("MemoryModel$get_file")]
    pub file: Option<u64>,
}

gen_row_impl!(SysMem, MemoryModel, "Mem");

struct SysIo;

impl SysIo {
    fn get_row(disks: &BTreeMap<String, SingleDiskModel>) -> StyledString {
        let mut disk_stat = format!("{:8.8}\t", "I/O");

        disks
            .iter()
            .filter(|(disk_name, _)| !disk_name.chars().last().unwrap().is_digit(10))
            .for_each(|(disk_name, sdm)| {
                disk_stat.push_str(&format!(
                    "{:7.7}{:<10.10}\t",
                    disk_name,
                    sdm.get_disk_total_bytes_per_sec_str(),
                ))
            });

        disk_stat.into()
    }
}

struct SysIface;

impl SysIface {
    fn get_row(net: &NetworkModel) -> StyledString {
        let mut network = format!("{:8.8}\t", "Iface");

        net.interfaces.iter().for_each(|(interface, snm)| {
            network.push_str(&format!(
                "{:7.7}{:<10.10}\t",
                interface,
                format!("{}/s", snm.get_throughput_per_sec_str())
            ))
        });
        network.into()
    }
}

fn fill_content(c: &mut Cursive, v: &mut LinearLayout) {
    let view_state = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    let system_model = view_state.system.borrow();
    let network_model = view_state.network.borrow();
    let cpu_row = SysCpu::get_row(&system_model.cpu);
    let mem_row = SysMem::get_row(&system_model.mem);
    let io_row = SysIo::get_row(&system_model.disks);
    let iface_row = SysIface::get_row(&network_model);

    let mut view = LinearLayout::vertical();
    view.add_child(TextView::new(cpu_row));
    view.add_child(TextView::new(mem_row));
    view.add_child(TextView::new(io_row));
    view.add_child(TextView::new(iface_row));

    *v = view;
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
