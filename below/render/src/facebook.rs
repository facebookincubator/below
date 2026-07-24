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

use RenderFormat::ReadableFrequency;
use RenderFormat::ReadableSize;

use super::*;

impl HasRenderConfig for model::GpuInfoModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::GpuInfoModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            MajMin(field_id) => model::MajMin::get_render_config_builder(field_id),
            Error => rc.title("Error"),
            DeviceMetadata(field_id) => {
                model::DeviceMetadataModel::get_render_config_builder(field_id)
            }
            Clock(field_id) => model::ClockModel::get_render_config_builder(field_id),
            Memory(field_id) => model::GpuMemoryModel::get_render_config_builder(field_id),
            DeviceInfo(field_id) => model::DeviceInfoModel::get_render_config_builder(field_id),
            Power(field_id) => model::PowerModel::get_render_config_builder(field_id),
            Temperature(field_id) => model::TemperatureModel::get_render_config_builder(field_id),
            Pcie(field_id) => model::PcieModel::get_render_config_builder(field_id),
            Nvlink(field_id) => model::NvlinkModel::get_render_config_builder(field_id),
            Mig(field_id) => model::MigModel::get_render_config_builder(field_id),
        }
    }
}

impl HasRenderConfig for model::MajMin {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::MajMinFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            MajorId => rc.title("Maj"),
            MinorId => rc.title("Min"),
        }
    }
}

impl HasRenderConfig for model::DeviceMetadataModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::DeviceMetadataModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Name => rc.title("Name").width(20),
            Uuid => rc.title("UUID").width(37),
            DriverVersion => rc.title("Driver Version"),
            VbiosVersion => rc.title("VBIOS Version"),
            Serial => rc.title("Serial").width(15),
            NumStreamingMultiprocessors => rc.title("Nr SMs"),
        }
    }
}

impl HasRenderConfig for model::ClockModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::ClockModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            ClockGraphicsCurrentHz => rc.title("Graphics Clock").format(ReadableFrequency),
            ClockMemoryCurrentHz => rc.title("Memory Clock").format(ReadableFrequency),
        }
    }
}

impl HasRenderConfig for model::GpuMemoryModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::GpuMemoryModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            MemoryUtilizationPct => rc.title("Mem Util").suffix("%"),
            MemoryFreeBytes => rc.title("Mem Free").format(ReadableSize),
            MemoryTotalBytes => rc.title("Mem Total").format(ReadableSize),
            MemoryUsedBytes => rc.title("Mem Used").format(ReadableSize),
            HbmMemBwBytesPerSec => rc.title("Hbm Mem Bw").format(ReadableSize).suffix("/s"),
            HbmMemBwUtilPct => rc.title("Hbm Mem Bw Util").suffix("%"),
        }
    }
}

impl HasRenderConfig for model::DeviceInfoModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::DeviceInfoModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            Fp(field_id) => model::FloatingPointModel::get_render_config_builder(field_id),
            // TODO(T121420482): Support dynamic column width based on contents
            Pids => rc.title("PIDs").width(15),
            GpuUtilizationPct => rc.title("GPU Util").suffix("%"),
            SmUtilizationPct => rc.title("SM Util").suffix("%"),
            SmOccupancyPct => rc.title("SM Occupancy").suffix("%"),
            SmOccupancyWarpsPerSm => rc.title("SM Occupancy").suffix(" warps/SM"),
        }
    }
}

impl HasRenderConfig for model::PowerModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::PowerModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            PowerDrawWatts => rc.title("Power Draw").suffix(" W"),
        }
    }
}

impl HasRenderConfig for model::TemperatureModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::TemperatureModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            TemperatureCelcius => rc.title("Temp").suffix(" C"),
        }
    }
}

impl HasRenderConfig for model::PcieModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::PcieModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            PcieRxActiveBytes => rc.title("PCIe Rx Active").format(ReadableSize),
            PcieTxActiveBytes => rc.title("PCIe Tx Active").format(ReadableSize),
            PcieRxBytesPerSec => rc.title("PCIE Rx rate").suffix("/s").format(ReadableSize),
            PcieTxBytesPerSec => rc.title("PCIE Tx rate").suffix("/s").format(ReadableSize),
            PcieTotalBytesPerSec => rc
                .title("PCIE Total rate")
                .suffix("/s")
                .format(ReadableSize),
        }
    }
}

impl HasRenderConfig for model::MigModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::MigModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            MigCurrent => rc.title("MIG state"),
            MigPending => rc.title("MIG pending state"),
        }
    }
}

impl HasRenderConfig for model::FloatingPointModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::FloatingPointModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            TensorCoreActivePct => rc.title("Tensor Core active").suffix("%"),
            Fp16ActivePct => rc.title("FP16 active").suffix("%"),
            Fp32ActivePct => rc.title("FP32 active").suffix("%"),
            Fp64ActivePct => rc.title("FP64 active").suffix("%"),
        }
    }
}

impl HasRenderConfig for model::NvlinkModel {
    fn get_render_config_builder(field_id: &Self::FieldId) -> RenderConfigBuilder {
        use model::NvlinkModelFieldId::*;
        let rc = RenderConfigBuilder::new();
        match field_id {
            NvlinkTxActiveBytes => rc.title("NVLink Tx Active").format(ReadableSize),
            NvlinkRxActiveBytes => rc.title("NVLink Rx Active").format(ReadableSize),
        }
    }
}
