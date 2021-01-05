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

use super::*;

use crate::render_config as rc;

use RenderFormat::{MaxOrReadableSize, Precision, ReadableSize};

impl HasRenderConfig for model::CgroupModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupModelFieldId::*;
        match field_id {
            Name => rc!(title("Name"), width(50)),
            FullPath => rc!(title("Full Path"), width(50)),
            InodeNumber => rc!(title("Inode Number")),
            Cpu(field_id) => model::CgroupCpuModel::get_render_config(field_id),
            Io(field_id) => model::CgroupIoModel::get_render_config(field_id),
            Mem(field_id) => model::CgroupMemoryModel::get_render_config(field_id),
            Pressure(field_id) => model::CgroupPressureModel::get_render_config(field_id),
        }
    }
}

impl HasRenderConfig for model::CgroupCpuModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupCpuModelFieldId::*;
        match field_id {
            UsagePct => rc!(title("CPU Usage"), suffix("%"), format(Precision(2))),
            UserPct => rc!(title("CPU User"), suffix("%"), format(Precision(2))),
            SystemPct => rc!(title("CPU Sys"), suffix("%"), format(Precision(2))),
            NrPeriodsPerSec => rc!(title("Nr Period"), suffix("/s"), format(Precision(2))),
            NrThrottledPerSec => rc!(title("Nr Throttled"), suffix("/s"), format(Precision(2))),
            ThrottledPct => rc!(title("Throttled"), suffix("%"), format(Precision(2))),
        }
    }
}

impl HasRenderConfig for model::CgroupIoModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupIoModelFieldId::*;
        match field_id {
            RbytesPerSec => rc!(title("Reads"), suffix("/s"), format(ReadableSize)),
            WbytesPerSec => rc!(title("Writes"), suffix("/s"), format(ReadableSize)),
            RiosPerSec => rc!(title("Read IOPS"), format(Precision(1))),
            WiosPerSec => rc!(title("Write IOPS"), format(Precision(1))),
            DbytesPerSec => rc!(title("Discards"), suffix("/s"), format(ReadableSize)),
            DiosPerSec => rc!(title("Discard IOPS"), format(Precision(1))),
            RwbytesPerSec => rc!(title("RW Total"), suffix("/s"), format(ReadableSize)),
        }
    }
}

impl HasRenderConfig for model::CgroupMemoryModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupMemoryModelFieldId::*;
        match field_id {
            Total => rc!(title("Memory"), format(ReadableSize)),
            Swap => rc!(title("Memory Swap"), format(ReadableSize)),
            MemoryHigh => rc!(title("Memory High"), format(MaxOrReadableSize)),
            EventsLow => rc!(title("Events Low")),
            EventsHigh => rc!(title("Events High")),
            EventsMax => rc!(title("Events Max")),
            EventsOom => rc!(title("Events OOM")),
            EventsOomKill => rc!(title("Events Kill")),
            Anon => rc!(title("Anon"), format(ReadableSize)),
            File => rc!(title("File"), format(ReadableSize)),
            KernelStack => rc!(title("Kernel Stack"), format(ReadableSize)),
            Slab => rc!(title("Slab"), format(ReadableSize)),
            Sock => rc!(title("Sock"), format(ReadableSize)),
            Shmem => rc!(title("Shmem"), format(ReadableSize)),
            FileMapped => rc!(title("File Mapped"), format(ReadableSize)),
            FileDirty => rc!(title("File Dirty"), format(ReadableSize)),
            FileWriteback => rc!(title("File WB"), format(ReadableSize)),
            AnonThp => rc!(title("Anon THP"), format(ReadableSize)),
            InactiveAnon => rc!(title("Inactive Anon"), format(ReadableSize)),
            ActiveAnon => rc!(title("Active Anon"), format(ReadableSize)),
            InactiveFile => rc!(title("Inactive File"), format(ReadableSize)),
            ActiveFile => rc!(title("Active File"), format(ReadableSize)),
            Unevictable => rc!(title("Unevictable"), format(ReadableSize)),
            SlabReclaimable => rc!(title("Slab Reclaimable"), format(ReadableSize)),
            SlabUnreclaimable => rc!(title("Slab Unreclaimable"), format(ReadableSize)),
            Pgfault => rc!(title("Pgfault/s")),
            Pgmajfault => rc!(title("Pgmajfault/s")),
            WorkingsetRefault => rc!(title("Workingset Refault/s")),
            WorkingsetActivate => rc!(title("Workingset Activate/s")),
            WorkingsetNodereclaim => rc!(title("Workingset Nodereclaim/s")),
            Pgrefill => rc!(title("Pgrefill/s")),
            Pgscan => rc!(title("Pgscan/s")),
            Pgsteal => rc!(title("Pgsteal/s")),
            Pgactivate => rc!(title("Pgactivate/s")),
            Pgdeactivate => rc!(title("Pgdeactivate/s")),
            Pglazyfree => rc!(title("Pglazyfree/s")),
            Pglazyfreed => rc!(title("Pglazyfreed/s")),
            ThpFaultAlloc => rc!(title("THP Fault Alloc/s")),
            ThpCollapseAlloc => rc!(title("THP Collapse Alloc/s")),
        }
    }
}

impl HasRenderConfig for model::CgroupPressureModel {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig {
        use model::CgroupPressureModelFieldId::*;
        match field_id {
            CpuSomePct => rc!(title("CPU Pressure"), suffix("%"), format(Precision(2))),
            IoSomePct => rc!(
                title("I/O Some Pressure"),
                suffix("%"),
                format(Precision(2)),
            ),
            IoFullPct => rc!(title("I/O Pressure"), suffix("%"), format(Precision(2))),
            MemorySomePct => rc!(
                title("Mem Some Pressure"),
                suffix("%"),
                format(Precision(2)),
            ),
            MemoryFullPct => rc!(title("Mem Pressure"), suffix("%"), format(Precision(2))),
        }
    }
}
