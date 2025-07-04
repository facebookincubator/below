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
use cursive::Cursive;
use cursive::view::Margins;
use cursive::view::Nameable;
use cursive::view::View;
use cursive::views::LinearLayout;
use cursive::views::PaddedView;
use cursive::views::Panel;

use crate::ViewState;

mod render_impl {
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::str::FromStr;

    use base_render::RenderConfig;
    use cursive::utils::markup::StyledString;
    use cursive::views::TextView;
    use model::Model;
    use model::ModelFieldId;
    use model::ProcessModel;
    use model::Queriable;
    use model::SingleDiskModel;
    use model::SingleNetModel;
    use model::SystemModel;
    use once_cell::sync::Lazy;
    use procfs::PidState;

    use crate::render::ViewItem;
    use crate::viewrc::ViewRc;

    /// Renders corresponding Fields From SystemModel.
    type SummaryViewItem = ViewItem<model::SystemModelFieldId>;

    static SYS_CPU_ITEMS: Lazy<Vec<SummaryViewItem>> = Lazy::new(|| {
        use model::SingleCpuModelFieldId::NicePct;
        use model::SingleCpuModelFieldId::SystemPct;
        use model::SingleCpuModelFieldId::UsagePct;
        use model::SingleCpuModelFieldId::UserPct;
        use model::SystemModelFieldId::Cpu;
        vec![
            ViewItem::from_default(Cpu(UsagePct)),
            ViewItem::from_default(Cpu(UserPct)),
            ViewItem::from_default(Cpu(SystemPct)),
            ViewItem::from_default(Cpu(NicePct)),
        ]
    });

    static SYS_MEM_ITEMS: Lazy<Vec<SummaryViewItem>> = Lazy::new(|| {
        use model::MemoryModelFieldId::Anon;
        use model::MemoryModelFieldId::File;
        use model::MemoryModelFieldId::Free;
        use model::MemoryModelFieldId::Total;
        use model::SystemModelFieldId::Mem;
        vec![
            ViewItem::from_default(Mem(Total)),
            ViewItem::from_default(Mem(Free)),
            ViewItem::from_default(Mem(Anon)),
            ViewItem::from_default(Mem(File)),
        ]
    });

    static SYS_VM_ITEMS: Lazy<Vec<SummaryViewItem>> = Lazy::new(|| {
        use model::SystemModelFieldId::Vm;
        use model::VmModelFieldId::PgpginPerSec;
        use model::VmModelFieldId::PgpgoutPerSec;
        use model::VmModelFieldId::PswpinPerSec;
        use model::VmModelFieldId::PswpoutPerSec;
        vec![
            ViewItem::from_default(Vm(PgpginPerSec)),
            ViewItem::from_default(Vm(PgpgoutPerSec)),
            ViewItem::from_default(Vm(PswpinPerSec)),
            ViewItem::from_default(Vm(PswpoutPerSec)),
        ]
    });

    /// Represents a single title/value entry in this view.
    pub struct Entry {
        title: String,
        value: String,
    }

    pub fn gather<T: Queriable>(
        model: &T,
        items: impl Iterator<Item = ViewItem<T::FieldId>>,
    ) -> Vec<Entry> {
        let mut group = Vec::new();
        for item in items {
            group.push(Entry {
                title: item.config.render_config.get_title().to_string(),
                value: item.render_tight(model).source().to_string(),
            });
        }
        group
    }

    pub fn render_extra_group(group: &SummaryViewExtraRow, model: &Model) -> TextView {
        let mut view = TextView::new("");
        let title_width = group
            .items
            .iter()
            .map(|item| item.config.render_config.get_title().len() + 2)
            .max()
            .unwrap_or(10);

        for item in &group.items {
            let mut row = StyledString::new();
            row.append(base_render::get_fixed_width(
                item.config.render_config.get_title(),
                title_width,
            ));
            row.append(item.render_tight(model));
            row.append('\n');

            view.append(row);
        }

        view
    }

    pub fn gather_read_write_models<'a, T: 'a + Queriable>(
        models: impl Iterator<Item = (&'a String, &'a T)>,
        read_item: ViewItem<T::FieldId>,
        write_item: ViewItem<T::FieldId>,
    ) -> Vec<Entry> {
        // Maximum number of I/O devices to display.
        const MAX_IO_DEVICES: usize = 3;

        let mut group = Vec::new();
        for (count, (name, model)) in models.enumerate() {
            if count >= MAX_IO_DEVICES {
                group.push(Entry {
                    title: "[...]".to_string(),
                    value: "[...]".to_string(),
                });
                break;
            }

            group.push(Entry {
                title: format!("{name} Read"),
                value: format!(
                    // Provide a reasonable fixed width for both read and write so that
                    // fluctuations do not cause columns to shift left and right every interval.
                    "{:10}",
                    read_item.render_tight(model).source(),
                ),
            });
            group.push(Entry {
                title: format!("{name} Write"),
                value: format!(
                    // See above.
                    "{:10}",
                    write_item.render_tight(model).source(),
                ),
            });
        }
        group
    }

    pub fn gather_cpu(model: &SystemModel) -> Vec<Entry> {
        gather(model, SYS_CPU_ITEMS.iter().cloned())
    }

    pub fn gather_mem(model: &SystemModel) -> Vec<Entry> {
        gather(model, SYS_MEM_ITEMS.iter().cloned())
    }

    pub fn gather_vm(model: &SystemModel) -> Vec<Entry> {
        gather(model, SYS_VM_ITEMS.iter().cloned())
    }

    pub fn gather_io(disks: &BTreeMap<String, SingleDiskModel>) -> Vec<Entry> {
        use model::SingleDiskModelFieldId::ReadBytesPerSec;
        use model::SingleDiskModelFieldId::WriteBytesPerSec;
        gather_read_write_models(
            disks.iter().filter(|(_, sdm)| sdm.minor == Some(0)),
            ViewItem::from_default(ReadBytesPerSec),
            ViewItem::from_default(WriteBytesPerSec),
        )
    }

    pub fn gather_iface(ifaces: &BTreeMap<String, SingleNetModel>) -> Vec<Entry> {
        use model::SingleNetModelFieldId::RxBytesPerSec;
        use model::SingleNetModelFieldId::TxBytesPerSec;
        gather_read_write_models(
            // Do not show localhost traffic
            ifaces.iter().filter(|(name, _model)| *name != "lo"),
            ViewItem::from_default(RxBytesPerSec),
            ViewItem::from_default(TxBytesPerSec),
        )
    }

    pub fn gather_state(processes: &ProcessModel) -> Vec<Entry> {
        let mut counts: HashMap<procfs::PidState, u32> = HashMap::new();
        for process in processes.processes.values() {
            if let Some(state) = process.state.clone() {
                let count = counts.entry(state).or_insert(0);
                *count += 1;
            }
        }
        let mut group = Vec::new();
        group.push(Entry {
            title: "Total".to_string(),
            value: processes.processes.len().to_string(),
        });

        for (state, name) in [
            (PidState::Running, "Running"),
            (PidState::Sleeping, "Sleeping"),
            (PidState::UninterruptibleSleep, "D-state"),
            (PidState::Zombie, "Zombie"),
        ] {
            let mut count = *counts.get(&state).unwrap_or(&0);
            if state == PidState::Sleeping {
                count += *counts.get(&PidState::Idle).unwrap_or(&0);
            }
            group.push(Entry {
                title: name.to_string(),
                value: count.to_string(),
            });
        }
        group
    }

    /// Render a group of entries.
    pub fn render_group(entries: &[Entry]) -> TextView {
        let mut view = TextView::new("");
        let title_width = entries
            .iter()
            .map(|e| e.title.len() + 2)
            .max()
            .unwrap_or(10);
        for entry in entries {
            let mut row = StyledString::new();
            row.append(base_render::get_fixed_width(&entry.title, title_width));
            row.append(&entry.value);
            row.append('\n');

            view.append(row);
        }
        view
    }

    pub struct SummaryViewExtraRow {
        pub title: Option<String>,
        pub items: Vec<ViewItem<model::ModelFieldId>>,
    }

    pub fn get_summary_view_extra_group(viewrc: &ViewRc) -> Vec<SummaryViewExtraRow> {
        if let Some(viewrc_rows) = viewrc.summary_view_extra_rows.as_ref() {
            viewrc_rows
                .iter()
                .map(|viewrc_row| SummaryViewExtraRow {
                    title: viewrc_row.title.clone(),
                    items: viewrc_row
                        .items
                        .iter()
                        // Skip invalid field ids
                        .filter_map(|item| {
                            ModelFieldId::from_str(&item.field_id)
                                .map(|field_id| {
                                    ViewItem::from_default(field_id).update(RenderConfig {
                                        title: item.alias.clone(),
                                        ..Default::default()
                                    })
                                })
                                .ok()
                        })
                        .collect(),
                })
                .collect()
        } else {
            vec![]
        }
    }
}

fn fill_content(c: &mut Cursive, v: &mut LinearLayout) {
    use render_impl::render_group;

    let view_state = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    let system_model = view_state.system.lock().unwrap();
    let network_model = view_state.network.lock().unwrap();
    let process_model = view_state.process.lock().unwrap();
    let cpu = render_impl::gather_cpu(&system_model);
    let mem = render_impl::gather_mem(&system_model);
    let vm = render_impl::gather_vm(&system_model);
    let io = render_impl::gather_io(&system_model.disks);
    let iface = render_impl::gather_iface(&network_model.interfaces);
    let state = render_impl::gather_state(&process_model);

    let mut view = LinearLayout::horizontal();
    let pad = |v| PaddedView::new(Margins::lr(0, 1), v);
    view.add_child(pad(Panel::new(render_group(&cpu)).title("CPU")));
    view.add_child(pad(Panel::new(render_group(&mem)).title("Mem")));
    view.add_child(pad(Panel::new(render_group(&vm)).title("Vm")));
    view.add_child(pad(Panel::new(render_group(&state)).title("Process")));
    view.add_child(pad(Panel::new(render_group(&io)).title("I/O")));
    view.add_child(pad(Panel::new(render_group(&iface)).title("Interface")));

    let model = view_state.model.lock().unwrap();
    // TODO: Save the parsed extra rows in a struct and reuse
    let extra_groups = render_impl::get_summary_view_extra_group(&view_state.viewrc);
    for extra_group in extra_groups {
        let mut panel = Panel::new(render_impl::render_extra_group(&extra_group, &model));
        if let Some(title) = &extra_group.title {
            panel.set_title(title);
        }
        view.add_child(pad(panel));
    }

    *v = view;
}

pub fn refresh(c: &mut Cursive) {
    let mut v = c
        .find_name::<LinearLayout>("summary_view")
        .expect("No summary_view view found!");

    fill_content(c, &mut v);
}

pub fn new(c: &mut Cursive) -> impl View + use<> {
    let mut view = LinearLayout::horizontal();
    fill_content(c, &mut view);
    view.with_name("summary_view")
}
