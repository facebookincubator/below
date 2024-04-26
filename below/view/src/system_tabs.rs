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

use base_render::get_fixed_width;
use base_render::RenderConfigBuilder as Rc;
use common::util::get_prefix;
use cursive::utils::markup::StyledString;
use model::system::BtrfsModelFieldId;
use model::system::MemoryModelFieldId;
use model::system::SingleCpuModelFieldId;
use model::system::SingleDiskModelFieldId;
use model::system::SingleSlabModelFieldId;
use model::system::VmModelFieldId;
use model::BtrfsModel;
use model::Queriable;
use model::SingleSlabModel;

use crate::render::ViewItem;
use crate::stats_view::ColumnTitles;
use crate::stats_view::StateCommon;
use crate::system_view::SystemState;
use crate::system_view::SystemStateFieldId;

const FIELD_NAME_WIDTH: usize = 20;
const FIELD_WIDTH: usize = 20;

pub trait SystemTab {
    fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: vec![
                get_fixed_width("Field", FIELD_NAME_WIDTH),
                get_fixed_width("Value", FIELD_WIDTH),
            ],
            pinned_titles: 1,
        }
    }

    fn get_rows(&self, state: &SystemState, offset: Option<usize>) -> Vec<(StyledString, String)>;
}

#[derive(Default, Clone)]
pub struct SystemCpu;

impl SystemTab for SystemCpu {
    fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: enum_iterator::all::<SingleCpuModelFieldId>()
                .map(|field_id| ViewItem::from_default(field_id).config.render_title())
                .collect(),
            pinned_titles: 1,
        }
    }

    fn get_rows(&self, state: &SystemState, offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();
        model
            .cpus
            .values()
            .filter(|scm| {
                if let Some((SystemStateFieldId::Cpu(field), filter)) = &state.filter_info {
                    match scm.query(field) {
                        None => true,
                        Some(value) => value.to_string().starts_with(filter),
                    }
                } else {
                    true
                }
            })
            .chain(std::iter::once(&model.total_cpu))
            .map(|scm| {
                (
                    std::iter::once(SingleCpuModelFieldId::Idx)
                        .chain(
                            enum_iterator::all::<SingleCpuModelFieldId>()
                                .skip(offset.unwrap_or(0) + 1),
                        )
                        .fold(StyledString::new(), |mut line, field_id| {
                            let view_item = ViewItem::from_default(field_id.clone());
                            let rendered =
                                if field_id == SingleCpuModelFieldId::Idx && scm.idx == -1 {
                                    view_item.config.render(Some("total".to_owned().into()))
                                } else {
                                    view_item.render(scm)
                                };
                            line.append(rendered);
                            line.append_plain(" ");
                            line
                        }),
                    "".to_owned(),
                )
            })
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct SystemMem;

impl SystemTab for SystemMem {
    fn get_rows(&self, state: &SystemState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        enum_iterator::all::<MemoryModelFieldId>()
            .map(|field_id| {
                let mut line = StyledString::new();
                let item =
                    ViewItem::from_default(field_id).update(Rc::new().width(FIELD_NAME_WIDTH));
                line.append_plain(item.config.render_title());
                line.append_plain(" ");
                line.append(item.update(Rc::new().width(FIELD_WIDTH)).render(&model.mem));
                line
            })
            .filter(|s| {
                if let Some((_, filter)) = &state.filter_info {
                    s.source().contains(filter)
                } else {
                    true
                }
            })
            .map(|s| (s.clone(), "".into()))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct SystemVm;

impl SystemTab for SystemVm {
    fn get_rows(&self, state: &SystemState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        enum_iterator::all::<VmModelFieldId>()
            .map(|field_id| {
                let mut line = StyledString::new();
                let item =
                    ViewItem::from_default(field_id).update(Rc::new().width(FIELD_NAME_WIDTH));
                line.append_plain(item.config.render_title());
                line.append_plain(" ");
                line.append(item.update(Rc::new().width(FIELD_WIDTH)).render(&model.vm));
                line
            })
            .filter(|s| {
                if let Some((_, filter)) = &state.filter_info {
                    s.source().contains(filter)
                } else {
                    true
                }
            })
            .map(|s| (s.clone(), "".into()))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct SystemSlab;

impl SystemTab for SystemSlab {
    fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: enum_iterator::all::<SingleSlabModelFieldId>()
                .map(|field_id| ViewItem::from_default(field_id).config.render_title())
                .collect(),
            pinned_titles: 1,
        }
    }

    fn get_rows(&self, state: &SystemState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();
        let mut slab: Vec<&SingleSlabModel> = model.slab.values().collect();

        if let Some(SystemStateFieldId::Slab(sort_order)) = state.sort_order.as_ref() {
            model::sort_queriables(&mut slab, sort_order, state.reverse);
        }

        slab.into_iter()
            .map(|ssm| {
                enum_iterator::all::<SingleSlabModelFieldId>().fold(
                    StyledString::new(),
                    |mut line, field_id| {
                        let view_item = ViewItem::from_default(field_id.clone());
                        line.append(view_item.render(ssm));
                        line.append_plain(" ");
                        line
                    },
                )
            })
            .filter(|s| {
                if let Some((_, filter)) = &state.filter_info {
                    s.source().contains(filter)
                } else {
                    true
                }
            })
            .map(|s| (s.clone(), "".into()))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct SystemDisk;

impl SystemTab for SystemDisk {
    fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: enum_iterator::all::<SingleDiskModelFieldId>()
                .map(|field_id| ViewItem::from_default(field_id).config.render_title())
                .collect(),
            pinned_titles: 1,
        }
    }

    fn get_rows(&self, state: &SystemState, offset: Option<usize>) -> Vec<(StyledString, String)> {
        state
            .get_model()
            .disks
            .iter()
            .filter_map(|(dn, sdm)| {
                // We use the partition parent id to check if it exists in collapsed_disk set.
                let idx_major = format!("{}.0", sdm.major.unwrap_or(0));
                let idx = format!("{}.{}", sdm.major.unwrap_or(0), sdm.minor.unwrap_or(0));
                let collapse = state.collapsed_disk.contains(&idx_major) && sdm.minor != Some(0);
                if state
                    .filter_info
                    .as_ref()
                    .map_or(!collapse, |(_, f)| dn.starts_with(f))
                {
                    Some((
                        std::iter::once(SingleDiskModelFieldId::Name)
                            .chain(
                                enum_iterator::all::<SingleDiskModelFieldId>()
                                    .skip(offset.unwrap_or(0) + 1),
                            )
                            .fold(StyledString::new(), |mut line, field_id| {
                                let view_item = ViewItem::from_default(field_id.clone());
                                let rendered = if field_id == SingleDiskModelFieldId::Name {
                                    view_item
                                        .update(Rc::new().indented_prefix(get_prefix(collapse)))
                                        .render_indented(sdm)
                                } else {
                                    view_item.render(sdm)
                                };
                                line.append(rendered);
                                line.append_plain(" ");
                                line
                            }),
                        idx,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Renders corresponding Fields From BtrfsModel.
type BtrfsViewItem = ViewItem<model::BtrfsModelFieldId>;

#[derive(Default, Clone)]
pub struct SystemBtrfs {
    pub view_items: Vec<BtrfsViewItem>,
}

impl SystemBtrfs {
    fn new(view_items: Vec<BtrfsViewItem>) -> Self {
        Self { view_items }
    }
}

impl SystemTab for SystemBtrfs {
    fn get_titles(&self) -> ColumnTitles {
        ColumnTitles {
            titles: enum_iterator::all::<BtrfsModelFieldId>()
                .map(|field_id| ViewItem::from_default(field_id).config.render_title())
                .collect(),
            pinned_titles: 0,
        }
    }

    fn get_rows(&self, state: &SystemState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        if let Some(btrfs_model) = state.get_model().btrfs.as_ref() {
            let mut subvolumes: Vec<&BtrfsModel> = btrfs_model.values().collect();

            if let Some(SystemStateFieldId::Btrfs(sort_order)) = state.sort_order.as_ref() {
                model::sort_queriables(&mut subvolumes, sort_order, state.reverse);
            }

            subvolumes
                .iter()
                .filter(|bmodel| {
                    if let Some((SystemStateFieldId::Btrfs(field), filter)) = &state.filter_info {
                        match bmodel.query(field) {
                            None => true,
                            Some(value) => value.to_string().contains(filter),
                        }
                    } else {
                        true
                    }
                })
                .map(|bmodel| {
                    (
                        enum_iterator::all::<BtrfsModelFieldId>().fold(
                            StyledString::new(),
                            |mut line, field_id| {
                                let view_item = ViewItem::from_default(field_id);
                                let rendered = view_item.render(bmodel);
                                line.append(rendered);
                                line.append_plain(" ");
                                line
                            },
                        ),
                        bmodel.name.as_ref().expect("No name for row").clone(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

pub mod default_tabs {
    use model::BtrfsModelFieldId::DiskBytes;
    use model::BtrfsModelFieldId::DiskFraction;
    use model::BtrfsModelFieldId::Name;
    use once_cell::sync::Lazy;

    use super::*;

    pub static SYSTEM_BTRFS_TAB: Lazy<SystemBtrfs> = Lazy::new(|| {
        SystemBtrfs::new(vec![
            ViewItem::from_default(Name),
            ViewItem::from_default(DiskFraction),
            ViewItem::from_default(DiskBytes),
        ])
    });
    pub enum SystemTabs {
        Btrfs(&'static SystemBtrfs),
    }
}
