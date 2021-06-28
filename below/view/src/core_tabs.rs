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

use crate::core_view::CoreState;
use crate::render::ViewItem;
use crate::stats_view::StateCommon;
use base_render::{get_fixed_width, RenderConfigBuilder as Rc};
use common::util::get_prefix;
use model::system::{
    MemoryModelFieldId, SingleCpuModelFieldId, SingleDiskModelFieldId, VmModelFieldId,
};

use cursive::utils::markup::StyledString;

const FIELD_NAME_WIDTH: usize = 20;
const FIELD_WIDTH: usize = 20;

pub trait CoreTab {
    fn get_title_vec(&self) -> Vec<String> {
        vec![
            get_fixed_width("Field", FIELD_NAME_WIDTH),
            get_fixed_width("Value", FIELD_WIDTH),
        ]
    }

    fn get_rows(&self, state: &CoreState, offset: Option<usize>) -> Vec<(StyledString, String)>;
}

#[derive(Default, Clone)]
pub struct CoreCpu;

impl CoreTab for CoreCpu {
    fn get_title_vec(&self) -> Vec<String> {
        SingleCpuModelFieldId::unit_variant_iter()
            .map(|field_id| ViewItem::from_default(field_id).config.render_title())
            .collect()
    }

    fn get_rows(&self, state: &CoreState, offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();
        model
            .cpus
            .iter()
            .filter(|scm| {
                if let Some(f) = &state.filter {
                    scm.idx.to_string().starts_with(f)
                } else {
                    true
                }
            })
            .chain(std::iter::once(&model.total_cpu))
            .map(|scm| {
                (
                    std::iter::once(SingleCpuModelFieldId::Idx)
                        .chain(
                            SingleCpuModelFieldId::unit_variant_iter()
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
pub struct CoreMem;

impl CoreTab for CoreMem {
    fn get_rows(&self, state: &CoreState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        MemoryModelFieldId::unit_variant_iter()
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
                if let Some(f) = &state.filter {
                    s.source().contains(f)
                } else {
                    true
                }
            })
            .map(|s| (s.clone(), "".into()))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct CoreVm;

impl CoreTab for CoreVm {
    fn get_rows(&self, state: &CoreState, _offset: Option<usize>) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        VmModelFieldId::unit_variant_iter()
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
                if let Some(f) = &state.filter {
                    s.source().contains(f)
                } else {
                    true
                }
            })
            .map(|s| (s.clone(), "".into()))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct CoreDisk;

impl CoreTab for CoreDisk {
    fn get_title_vec(&self) -> Vec<String> {
        SingleDiskModelFieldId::unit_variant_iter()
            .map(|field_id| ViewItem::from_default(field_id).config.render_title())
            .collect()
    }

    fn get_rows(&self, state: &CoreState, offset: Option<usize>) -> Vec<(StyledString, String)> {
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
                    .filter
                    .as_ref()
                    .map_or(!collapse, |f| dn.starts_with(f))
                {
                    Some((
                        std::iter::once(SingleDiskModelFieldId::Name)
                            .chain(
                                SingleDiskModelFieldId::unit_variant_iter()
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
