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

use crate::render::{HasViewStyle, ViewStyle, CPU_HIGHLIGHT, PRESSURE_HIGHLIGHT};

impl HasViewStyle for model::CgroupModel {
    fn get_view_style(field_id: &Self::FieldId) -> Option<ViewStyle> {
        use model::CgroupModelFieldId::{Cpu, Pressure};
        match field_id {
            Cpu(field_id) => model::CgroupCpuModel::get_view_style(field_id),
            Pressure(field_id) => model::CgroupPressureModel::get_view_style(field_id),
            _ => None,
        }
    }
}

impl HasViewStyle for model::CgroupCpuModel {
    fn get_view_style(field_id: &Self::FieldId) -> Option<ViewStyle> {
        use model::CgroupCpuModelFieldId::{SystemPct, UsagePct, UserPct};
        match field_id {
            UsagePct | UserPct | SystemPct => Some(CPU_HIGHLIGHT.clone()),
            _ => None,
        }
    }
}

impl HasViewStyle for model::CgroupPressureModel {
    fn get_view_style(_field_id: &Self::FieldId) -> Option<ViewStyle> {
        Some(PRESSURE_HIGHLIGHT.clone())
    }
}

impl HasViewStyle for model::SingleNetModel {}
