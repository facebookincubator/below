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

use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter::FromIterator;

use cursive::theme::Effect;
use cursive::view::{Identifiable, Scrollable, View};
use cursive::views::{LinearLayout, ResizedView, SelectView, TextView};
use cursive::Cursive;

use super::util::convert_bytes;
use crate::model;
use crate::view::{SortOrder, ViewState};

/// This trait described a field in the display so we can generically
/// display it's name and value and sort on it
trait Field {
    /// The "model" type this field is contained in
    type Model;

    /// The name of the field (to be displayed in the title bar)
    fn name(&self) -> &'static str;
    /// The width to pad out for this field
    fn display_width(&self) -> usize;
    /// The string format of the value - not yet padded out
    fn display_value(&self, model: &Self::Model, collapsed: bool) -> String;
    /// A comparison function to order on the given field
    fn compare(&self, a: &Self::Model, b: &Self::Model) -> Option<Ordering>;
}

struct NameField {}

impl NameField {
    fn get_value<'a>(&self, model: &'a model::CgroupModel) -> Option<&'a String> {
        Some(&model.name)
    }
}

impl Field for NameField {
    type Model = model::CgroupModel;

    fn name(&self) -> &'static str {
        "Name"
    }
    fn display_width(&self) -> usize {
        50
    }
    fn display_value(&self, model: &Self::Model, collapsed: bool) -> String {
        let mut display = if model.depth > 0 {
            let mut s = "   ".repeat(model.depth as usize - 1);
            s.push_str("└");
            s.push_str(if collapsed { "+ " } else { "─ " });
            s
        } else {
            String::new()
        };
        display.push_str(self.get_value(model).expect("Failed to get cgroup name"));
        display
    }

    fn compare(&self, a: &Self::Model, b: &Self::Model) -> Option<Ordering> {
        self.get_value(a).partial_cmp(&self.get_value(b))
    }
}

/// A macro to reduce boiler plate for field creation
macro_rules! field {
    ($struct_name:ident, $display_name:expr, $width:expr, $value_type:ty, $get_value:expr, $format_value:expr) => {
        struct $struct_name {}

        impl $struct_name {
            fn get_value<'a>(&self, model: &'a model::CgroupModel) -> Option<&'a $value_type> {
                $get_value(model)
            }
        }

        impl Field for $struct_name {
            type Model = model::CgroupModel;

            fn name(&self) -> &'static str {
                $display_name
            }

            fn display_width(&self) -> usize {
                $width
            }

            fn display_value(&self, model: &Self::Model, _: bool) -> String {
                self.get_value(model)
                    .map_or_else(|| "?".to_string(), $format_value)
            }

            fn compare(&self, a: &Self::Model, b: &Self::Model) -> Option<Ordering> {
                self.get_value(a).partial_cmp(&self.get_value(b))
            }
        }
    };
}

/// Even more reduced boilerplate for fields that are a percentage
macro_rules! pct_field {
    ($struct_name:ident, $display_name:expr, $get_value:expr) => {
        field!(
            $struct_name,
            $display_name,
            15,
            f64,
            $get_value,
            |v| format!("{:.2}%", v)
        );
    };
}

pct_field!(CpuField, "CPU", |model: &'a model::CgroupModel| model
    .cpu
    .as_ref()
    .and_then(|cpu_model| cpu_model.usage_pct.as_ref()));

field!(
    MemoryField,
    "Memory",
    11,
    u64,
    |model: &'a model::CgroupModel| model
        .memory
        .as_ref()
        .and_then(|memory_model| memory_model.total.as_ref()),
    |v| convert_bytes(*v as f64)
);

pct_field!(
    CpuPressureField,
    "CPU Pressure",
    |model: &'a model::CgroupModel| model
        .pressure
        .as_ref()
        .and_then(|pressure| pressure.cpu_some_pct.as_ref())
);

pct_field!(
    MemoryPressureField,
    "Memory Pressure",
    |model: &'a model::CgroupModel| model
        .pressure
        .as_ref()
        .and_then(|pressure| pressure.memory_full_pct.as_ref())
);

pct_field!(
    IoPressureField,
    "I/O Pressure",
    |model: &'a model::CgroupModel| model
        .pressure
        .as_ref()
        .and_then(|pressure| pressure.io_full_pct.as_ref())
);

field!(
    ReadBytesField,
    "Read Bytes/Sec",
    11,
    f64,
    |model: &'a model::CgroupModel| model
        .io_total
        .as_ref()
        .and_then(|io| io.rbytes_per_sec.as_ref()),
    |v| format!("{}/s", convert_bytes(*v))
);

field!(
    WriteBytesField,
    "Write Bytes/Sec",
    11,
    f64,
    |model: &'a model::CgroupModel| model
        .io_total
        .as_ref()
        .and_then(|io| io.wbytes_per_sec.as_ref()),
    |v| format!("{}/s", convert_bytes(*v))
);

const NAME_FIELD: NameField = NameField {};
const CPU_FIELD: CpuField = CpuField {};
const MEMORY_FIELD: MemoryField = MemoryField {};
const CPU_PRESSURE_FIELD: CpuPressureField = CpuPressureField {};
const MEMORY_PRESSURE_FIELD: MemoryPressureField = MemoryPressureField {};
const IO_PRESSURE_FIELD: IoPressureField = IoPressureField {};
const READ_BYTES_FIELD: ReadBytesField = ReadBytesField {};
const WRITE_BYTES_FIELD: WriteBytesField = WriteBytesField {};
const CGROUP_FIELDS: [&'static dyn Field<Model = model::CgroupModel>; 8] = [
    &NAME_FIELD,
    &CPU_FIELD,
    &MEMORY_FIELD,
    &CPU_PRESSURE_FIELD,
    &MEMORY_PRESSURE_FIELD,
    &IO_PRESSURE_FIELD,
    &READ_BYTES_FIELD,
    &WRITE_BYTES_FIELD,
];

fn get_cgroup_rows(view_state: &ViewState) -> Vec<(String, String)> {
    fn output_cgroup(
        cgroup: &model::CgroupModel,
        sort_order: SortOrder,
        collapsed_cgroups: &HashSet<String>,
        output: &mut Vec<(String, String)>,
    ) {
        let collapsed = collapsed_cgroups.contains(&cgroup.full_path);
        let mut row = String::new();
        for field in &CGROUP_FIELDS {
            row.push_str(&format!(
                "{:width$.width$}",
                field.display_value(cgroup, collapsed),
                width = field.display_width()
            ));
            row.push(' ');
        }
        // Each row is (label, value), where label is visible and value is used
        // as identifier to correlate the row with its state in global data.
        output.push((row, cgroup.full_path.clone()));
        if collapsed {
            return;
        }

        let mut children = Vec::from_iter(&cgroup.children);

        // Here we map the sort order to an index (or for disk, do some custom sorting)
        let sort_field_index = match sort_order {
            SortOrder::CPU => Some(1),
            SortOrder::Memory => Some(2),
            SortOrder::Disk => {
                let sum_bytes = |model: &model::CgroupIoModel| {
                    model.rbytes_per_sec.unwrap_or(0.0) + model.wbytes_per_sec.unwrap_or(0.0)
                };
                children.sort_by(|lhs, rhs| {
                    let a = lhs.io_total.as_ref().map_or(0.0, sum_bytes);
                    let b = rhs.io_total.as_ref().map_or(0.0, sum_bytes);
                    return b.partial_cmp(&a).unwrap_or(Ordering::Equal);
                });
                None
            }
            _ => None,
        };

        if let Some(index) = sort_field_index {
            if let Some(field) = CGROUP_FIELDS.get(index) {
                children.sort_by(|a, b| {
                    field
                        .compare(a, b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .reverse()
                });
            }
        }

        for child_cgroup in &children {
            output_cgroup(child_cgroup, sort_order, collapsed_cgroups, output);
        }
    };

    let mut rows = Vec::new();
    output_cgroup(
        &view_state.model.cgroup,
        view_state.sort_order,
        &view_state.collapsed_cgroups,
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

    let mut header = String::new();
    for field in &CGROUP_FIELDS {
        header.push_str(&format!(
            "{:width$.width$}",
            field.name(),
            width = field.display_width()
        ));
        header.push(' ');
    }

    LinearLayout::vertical()
        .child(TextView::new(header).effect(Effect::Bold))
        .child(ResizedView::with_full_screen(
            list.with_name("cgroup_view").scrollable(),
        ))
        .scrollable()
        .scroll_x(true)
        .scroll_y(false)
}
