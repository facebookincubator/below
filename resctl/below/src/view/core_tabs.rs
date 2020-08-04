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

use crate::model::system::*;
use crate::view::core_view::CoreState;
use crate::view::stats_view::StateCommon;

use cursive::utils::markup::StyledString;

pub trait CoreTab {
    fn get_title_vec(&self, _: &SystemModel) -> Vec<String> {
        vec![format!("{:<20.20}", "Field"), format!("{:<20.20}", "Value")]
    }

    fn get_rows(&mut self, state: &CoreState) -> Vec<(StyledString, String)>;
}

#[derive(Default, Clone)]
pub struct CoreCpu;

impl CoreTab for CoreCpu {
    fn get_title_vec(&self, _: &SystemModel) -> Vec<String> {
        let scm = SingleCpuModel {
            ..Default::default()
        };
        let mut res: Vec<String> = scm
            .get_title_pipe()
            .trim()
            .split('|')
            .map(|s| s.to_string())
            .collect();
        res.pop();
        res
    }

    fn get_rows(&mut self, state: &CoreState) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        let mut res: Vec<(StyledString, String)> = model
            .cpu
            .cpus
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .filter(|scm| {
                if let Some(f) = &state.filter {
                    scm.idx.to_string().starts_with(f)
                } else {
                    true
                }
            })
            .map(|scm| (scm.get_field_line(), "".into()))
            .collect();

        res.push((
            model
                .cpu
                .total_cpu
                .as_ref()
                .unwrap_or(&SingleCpuModel {
                    ..Default::default()
                })
                .get_field_line(),
            "".into(),
        ));

        res
    }
}

#[derive(Default, Clone)]
pub struct CoreMem;

impl CoreTab for CoreMem {
    fn get_rows(&mut self, state: &CoreState) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        model
            .mem
            .get_interleave_line(" ")
            .iter()
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
    fn get_rows(&mut self, state: &CoreState) -> Vec<(StyledString, String)> {
        let model = state.get_model();

        model
            .vm
            .get_interleave_line(" ")
            .iter()
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
    fn get_title_vec(&self, _: &SystemModel) -> Vec<String> {
        let sdm = SingleDiskModel {
            ..Default::default()
        };
        let mut res: Vec<String> = sdm
            .get_title_pipe()
            .trim()
            .split('|')
            .map(|s| s.to_string())
            .collect();
        res.pop();
        res
    }

    fn get_rows(&mut self, state: &CoreState) -> Vec<(StyledString, String)> {
        state
            .get_model_mut()
            .disks
            .iter_mut()
            .map(|(dn, sdm)| {
                // We use the partition parent id to check if it exists in collapsed_disk set.
                let idx_major = format!("{}.0", sdm.major.unwrap_or(0));
                let idx = format!("{}.{}", sdm.major.unwrap_or(0), sdm.minor.unwrap_or(0));
                sdm.collapse = state.collapsed_disk.contains(&idx_major) && sdm.minor != Some(0);
                (dn, sdm, idx)
            })
            .filter(|(dn, sdm, _idx)| {
                if let Some(f) = &state.filter {
                    dn.starts_with(f)
                } else {
                    !sdm.collapse
                }
            })
            .map(|(_, sdm, idx)| (sdm.get_field_line(), idx))
            .collect()
    }
}
