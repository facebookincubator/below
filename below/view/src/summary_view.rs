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
use cursive::view::Nameable;
use cursive::view::View;
use cursive::views::LinearLayout;
use cursive::views::TextView;
use cursive::Cursive;

use crate::ViewState;

mod render_impl {
    use std::collections::BTreeMap;
    use std::str::FromStr;

    use base_render::RenderConfig;
    use base_render::RenderConfigBuilder as Rc;
    use cursive::utils::markup::StyledString;
    use model::Model;
    use model::ModelFieldId;
    use model::Queriable;
    use model::SingleDiskModel;
    use model::SingleNetModel;
    use model::SystemModel;
    use once_cell::sync::Lazy;

    use crate::render::ViewItem;
    use crate::viewrc::ViewRc;

    /// Renders corresponding Fields From SystemModel.
    type SummaryViewItem = ViewItem<model::SystemModelFieldId>;

    static SYS_CPU_ITEMS: Lazy<Vec<SummaryViewItem>> = Lazy::new(|| {
        use model::SingleCpuModelFieldId::SystemPct;
        use model::SingleCpuModelFieldId::UsagePct;
        use model::SingleCpuModelFieldId::UserPct;
        use model::SystemModelFieldId::Cpu;
        vec![
            ViewItem::from_default(Cpu(UsagePct)),
            ViewItem::from_default(Cpu(UserPct)),
            ViewItem::from_default(Cpu(SystemPct)),
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

    const ROW_NAME_WIDTH: usize = 15;
    const ROW_FIELD_NAME_WIDTH: usize = 9;
    const ROW_FIELD_WIDTH: usize = 21;
    /// ROW_FIELD_WIDTH_HALVED * 2 + 1 == ROW_NAME_WIDTH, the +1 is for the '|'
    ///
    /// Need 10 chars for each field b/c each value has up to 4 significant digits
    /// with a dot, a space, 4 chars for units
    const ROW_FIELD_WIDTH_HALVED: usize = 10;
    /// Maximum number of I/O devices to display
    const MAX_IO_DEVICES: usize = 5;

    pub fn render_row<T: Queriable>(
        name: &'static str,
        model: &T,
        items: impl Iterator<Item = ViewItem<T::FieldId>>,
    ) -> StyledString {
        let mut row = StyledString::new();
        row.append(base_render::get_fixed_width(name, ROW_NAME_WIDTH));
        for item in items {
            let title = item.config.render_config.get_title();
            row.append(base_render::get_fixed_width(title, ROW_FIELD_NAME_WIDTH));
            row.append(item.update(Rc::new().width(ROW_FIELD_WIDTH)).render(model));
        }
        row
    }

    pub fn render_extra_row(extra_row: &SummaryViewExtraRow, model: &Model) -> StyledString {
        let mut row = StyledString::new();
        if let Some(title) = &extra_row.title {
            row.append(title.clone());
        }
        for item in &extra_row.items {
            if !row.is_empty() {
                row.append(" | ");
            }
            row.append(format!("{} ", item.config.render_config.get_title()));
            row.append(item.render_tight(model));
        }
        row
    }

    pub fn render_read_write_models_row<'a, T: 'a + Queriable>(
        name: &'static str,
        models: impl Iterator<Item = (&'a String, &'a T)>,
        read_item: ViewItem<T::FieldId>,
        write_item: ViewItem<T::FieldId>,
    ) -> StyledString {
        let mut count = 0;
        let read_item = read_item.update(Rc::new().width(ROW_FIELD_WIDTH_HALVED));
        let write_item = write_item.update(Rc::new().width(ROW_FIELD_WIDTH_HALVED));

        let mut row = StyledString::new();
        row.append(base_render::get_fixed_width(name, ROW_NAME_WIDTH));
        for (name, model) in models {
            if count >= MAX_IO_DEVICES {
                row.append(base_render::get_fixed_width(name, ROW_FIELD_NAME_WIDTH));
                row.append_plain("[...]");
                break;
            }

            row.append(base_render::get_fixed_width(name, ROW_FIELD_NAME_WIDTH));
            row.append(read_item.render(model));
            row.append_plain("|");
            row.append(write_item.render(model));
            count += 1;
        }
        row
    }

    pub fn render_cpu_row(model: &SystemModel) -> StyledString {
        render_row("CPU", model, SYS_CPU_ITEMS.iter().cloned())
    }

    pub fn render_mem_row(model: &SystemModel) -> StyledString {
        render_row("Mem", model, SYS_MEM_ITEMS.iter().cloned())
    }

    pub fn render_vm_row(model: &SystemModel) -> StyledString {
        render_row("VM", model, SYS_VM_ITEMS.iter().cloned())
    }

    pub fn render_io_row(disks: &BTreeMap<String, SingleDiskModel>) -> StyledString {
        use model::SingleDiskModelFieldId::ReadBytesPerSec;
        use model::SingleDiskModelFieldId::WriteBytesPerSec;
        render_read_write_models_row(
            "I/O   (Rd|Wr)", // Line up () with Iface's below
            disks.iter().filter(|(_, sdm)| sdm.minor == Some(0)),
            ViewItem::from_default(ReadBytesPerSec),
            ViewItem::from_default(WriteBytesPerSec),
        )
    }

    pub fn render_iface_row(ifaces: &BTreeMap<String, SingleNetModel>) -> StyledString {
        use model::SingleNetModelFieldId::RxBytesPerSec;
        use model::SingleNetModelFieldId::TxBytesPerSec;
        render_read_write_models_row(
            "Iface (Rx|Tx)",
            ifaces.iter(),
            ViewItem::from_default(RxBytesPerSec),
            ViewItem::from_default(TxBytesPerSec),
        )
    }

    pub struct SummaryViewExtraRow {
        pub title: Option<String>,
        pub items: Vec<ViewItem<model::ModelFieldId>>,
    }

    pub fn get_summary_view_extra_rows(viewrc: &ViewRc) -> Vec<SummaryViewExtraRow> {
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
    let view_state = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!");

    let system_model = view_state.system.borrow();
    let network_model = view_state.network.borrow();
    let cpu_row = render_impl::render_cpu_row(&system_model);
    let mem_row = render_impl::render_mem_row(&system_model);
    let vm_row = render_impl::render_vm_row(&system_model);
    let io_row = render_impl::render_io_row(&system_model.disks);
    let iface_row = render_impl::render_iface_row(&network_model.interfaces);

    let mut view = LinearLayout::vertical();
    view.add_child(TextView::new(cpu_row));
    view.add_child(TextView::new(mem_row));
    view.add_child(TextView::new(vm_row));
    view.add_child(TextView::new(io_row));
    view.add_child(TextView::new(iface_row));

    let model = view_state.model.borrow();
    // TODO: Save the parsed extra rows in a struct and reuse
    let extra_rows = render_impl::get_summary_view_extra_rows(&view_state.viewrc);
    for extra_row in extra_rows {
        view.add_child(TextView::new(render_impl::render_extra_row(
            &extra_row, &model,
        )));
    }

    *v = view;
}

pub fn refresh(c: &mut Cursive) {
    let mut v = c
        .find_name::<LinearLayout>("summary_view")
        .expect("No summary_view view found!");

    fill_content(c, &mut v);
}

pub fn new(c: &mut Cursive) -> impl View {
    let mut view = LinearLayout::vertical();
    fill_content(c, &mut view);
    view.with_name("summary_view")
}
