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

use base_render::RenderConfigBuilder as Rc;
use base_render::get_fixed_width;
use common::util::get_prefix;
use cursive::utils::markup::StyledString;
use model::BtrfsModel;
use model::Queriable;
use model::ResctrlL3MonModel;
use model::ResctrlL3MonModelFieldId;
use model::SingleSlabModel;
use model::system::BtrfsModelFieldId;
use model::system::KsmModelFieldId;
use model::system::MemoryModelFieldId;
use model::system::SingleCpuModelFieldId;
use model::system::SingleDiskModelFieldId;
use model::system::SingleSlabModelFieldId;
use model::system::VmModelFieldId;

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
        let mut slab: Vec<&SingleSlabModel> = model.slab.iter().collect();

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
pub struct SystemKsm;

impl SystemTab for SystemKsm {
    fn get_rows(&self, state: &SystemState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        if let Some(ksm_model) = state.get_model().ksm.as_ref() {
            enum_iterator::all::<KsmModelFieldId>()
                .map(|field_id| {
                    let mut line = StyledString::new();
                    let item =
                        ViewItem::from_default(field_id).update(Rc::new().width(FIELD_NAME_WIDTH));
                    line.append_plain(item.config.render_title());
                    line.append_plain(" ");
                    line.append(item.update(Rc::new().width(FIELD_WIDTH)).render(ksm_model));
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
        } else {
            Vec::new()
        }
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
                // Use _partition suffix to tell apart partitions from disks
                let idx = if sdm.is_partition == Some(true) {
                    format!("{}_partition", sdm.name.as_ref().map_or("", |v| v))
                } else {
                    sdm.name.clone().unwrap_or("".to_string())
                };
                // Only hide partitions whose parent is collapsed
                // Partitions always starts with their parent disk name with p?[0-9]+ suffix
                let collapse = sdm.is_partition == Some(true)
                    && state
                        .collapsed_disk
                        .iter()
                        .any(|d| sdm.name.as_ref().is_some_and(|v| v.starts_with(d)));
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

// Wide enough for an indented mon group name like
// "  MON_409e00b71d974765888774189e179920" (a "MON_" prefix + 32 hex chars,
// plus indentation) without clipping.
const RESCTRL_GROUP_WIDTH: usize = 42;
const RESCTRL_MODE_WIDTH: usize = 15;

/// The L3 monitoring fields rendered (as the per-group total) by the Resctrl tab.
const RESCTRL_L3_FIELDS: [ResctrlL3MonModelFieldId; 3] = [
    ResctrlL3MonModelFieldId::LlcOccupancyBytes,
    ResctrlL3MonModelFieldId::MbmTotalBytesPerSec,
    ResctrlL3MonModelFieldId::MbmLocalBytesPerSec,
];

/// Renders a single resctrl group as a row: group name (indented by `depth`),
/// mode, the L3 total monitoring stats, and finally the cpuset. The cpuset is
/// last because it is variable length and can be wide.
fn render_resctrl_row(
    name: &str,
    depth: usize,
    mode: &str,
    cpuset: &str,
    l3: Option<&ResctrlL3MonModel>,
) -> StyledString {
    let mut line = StyledString::new();
    let indented_name = format!("{:indent$}{}", "", name, indent = depth * 2);
    line.append_plain(get_fixed_width(&indented_name, RESCTRL_GROUP_WIDTH));
    line.append_plain(get_fixed_width(mode, RESCTRL_MODE_WIDTH));

    let default_l3 = ResctrlL3MonModel::default();
    let l3 = l3.unwrap_or(&default_l3);
    for field_id in RESCTRL_L3_FIELDS.iter() {
        line.append(ViewItem::from_default(field_id.clone()).render(l3));
        line.append_plain(" ");
    }

    // Cpuset is the final column, so render it in full rather than truncating.
    line.append_plain(cpuset);
    line
}

#[derive(Default, Clone)]
pub struct SystemResctrl;

impl SystemTab for SystemResctrl {
    fn get_titles(&self) -> ColumnTitles {
        let mut titles = vec![
            get_fixed_width("Group", RESCTRL_GROUP_WIDTH),
            get_fixed_width("Mode", RESCTRL_MODE_WIDTH),
        ];
        titles.extend(RESCTRL_L3_FIELDS.iter().map(|field_id| {
            ViewItem::from_default(field_id.clone())
                .config
                .render_title()
        }));
        titles.push("Cpuset".to_owned());
        ColumnTitles {
            titles,
            pinned_titles: 1,
        }
    }

    fn get_rows(&self, state: &SystemState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        let resctrl = state.resctrl.lock().unwrap();
        let model = match resctrl.as_ref() {
            Some(model) => model,
            None => return Vec::new(),
        };

        // Root group, followed by its directly-nested MON groups.
        let root_mode = model
            .mode
            .as_ref()
            .map_or_else(|| "-".to_owned(), |mode| format!("{:?}", mode));
        let root_cpuset = model
            .cpuset
            .as_ref()
            .map_or_else(|| "-".to_owned(), |cpuset| cpuset.to_string());
        let root_row = std::iter::once((
            render_resctrl_row(
                "<root>",
                0,
                &root_mode,
                &root_cpuset,
                model.mon.as_ref().map(|mon| &mon.total),
            ),
            "<root>".to_owned(),
        ));
        let root_mon_groups = model.mon_groups.iter().map(|(name, mon_group)| {
            (
                render_resctrl_row(name, 1, "-", "-", Some(&mon_group.mon.total)),
                mon_group.full_path.clone(),
            )
        });

        // Each CTRL_MON group, followed by its nested MON groups.
        let ctrl_mon_groups = model.ctrl_mon_groups.iter().flat_map(|(name, ctrl)| {
            let mode = ctrl
                .mode
                .as_ref()
                .map_or_else(|| "-".to_owned(), |mode| format!("{:?}", mode));
            let cpuset = ctrl
                .cpuset
                .as_ref()
                .map_or_else(|| "-".to_owned(), |cpuset| cpuset.to_string());
            std::iter::once((
                render_resctrl_row(name, 0, &mode, &cpuset, Some(&ctrl.mon.total)),
                ctrl.full_path.clone(),
            ))
            .chain(ctrl.mon_groups.iter().map(|(name, mon_group)| {
                (
                    render_resctrl_row(name, 1, "-", "-", Some(&mon_group.mon.total)),
                    mon_group.full_path.clone(),
                )
            }))
            .collect::<Vec<_>>()
        });

        root_row
            .chain(root_mon_groups)
            .chain(ctrl_mon_groups)
            .filter(|(line, _)| {
                if let Some((_, filter)) = &state.filter_info {
                    line.source().contains(filter)
                } else {
                    true
                }
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::Mutex;

    use model::ResctrlCtrlMonGroupModel;
    use model::ResctrlL3MonModel;
    use model::ResctrlModel;
    use model::ResctrlMonModel;

    use super::*;

    fn l3(llc_occupancy_bytes: u64) -> ResctrlL3MonModel {
        ResctrlL3MonModel {
            llc_occupancy_bytes: Some(llc_occupancy_bytes),
            mbm_total_bytes_per_sec: Some(llc_occupancy_bytes * 2),
            mbm_local_bytes_per_sec: Some(llc_occupancy_bytes),
        }
    }

    fn mon(total: ResctrlL3MonModel) -> ResctrlMonModel {
        ResctrlMonModel {
            total,
            ..Default::default()
        }
    }

    fn model_with_one_ctrl_mon_group() -> ResctrlModel {
        let mut ctrl_mon_groups = BTreeMap::new();
        ctrl_mon_groups.insert(
            "group1".to_owned(),
            ResctrlCtrlMonGroupModel {
                name: "group1".to_owned(),
                full_path: "group1".to_owned(),
                mon: mon(l3(512 * 1024)),
                ..Default::default()
            },
        );
        ResctrlModel {
            mon: Some(mon(l3(1024 * 1024))),
            ctrl_mon_groups,
            ..Default::default()
        }
    }

    fn state_with(model: Option<ResctrlModel>) -> SystemState {
        SystemState {
            resctrl: Arc::new(Mutex::new(model)),
            ..Default::default()
        }
    }

    #[test]
    fn test_resctrl_get_rows() {
        let state = state_with(Some(model_with_one_ctrl_mon_group()));
        let rows = SystemResctrl.get_rows(&state, None);

        // One row for the <root> group plus one for the single CTRL_MON group.
        assert_eq!(rows.len(), 2, "expected root + one ctrl_mon group row");
        assert_eq!(rows[0].1, "<root>");
        assert_eq!(rows[1].1, "group1");
        assert!(
            rows[0].0.source().contains("<root>"),
            "root row should render the group name"
        );
        // The root group's 1 MiB LLC occupancy should render as a readable size.
        assert!(
            rows[0].0.source().contains("MB") || rows[0].0.source().contains("MiB"),
            "root row should render LLC occupancy as a human-readable size, got: {}",
            rows[0].0.source()
        );
    }

    #[test]
    fn test_resctrl_get_rows_filter() {
        let mut state = state_with(Some(model_with_one_ctrl_mon_group()));
        state.filter_info = Some((
            SystemStateFieldId::Resctrl(ResctrlL3MonModelFieldId::LlcOccupancyBytes),
            "group1".to_owned(),
        ));

        let rows = SystemResctrl.get_rows(&state, None);
        assert_eq!(rows.len(), 1, "filter should keep only the matching group");
        assert_eq!(rows[0].1, "group1");
    }

    #[test]
    fn test_resctrl_get_rows_no_data() {
        let state = state_with(None);
        assert!(
            SystemResctrl.get_rows(&state, None).is_empty(),
            "absent resctrl data should yield no rows"
        );
    }
}
