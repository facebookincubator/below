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

// TODO(T91718069): Remove Thrift structs completely

use crate::types::*;
use std::convert::TryInto;

impl From<cgroupfs_thrift::CpuStat> for CpuStat {
    fn from(cpu_stat: cgroupfs_thrift::CpuStat) -> Self {
        Self {
            usage_usec: cpu_stat.usage_usec.map(|x| x.try_into().unwrap()),
            user_usec: cpu_stat.user_usec.map(|x| x.try_into().unwrap()),
            system_usec: cpu_stat.system_usec.map(|x| x.try_into().unwrap()),
            nr_periods: cpu_stat.nr_periods.map(|x| x.try_into().unwrap()),
            nr_throttled: cpu_stat.nr_throttled.map(|x| x.try_into().unwrap()),
            throttled_usec: cpu_stat.throttled_usec.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<cgroupfs_thrift::IoStat> for IoStat {
    fn from(io_stat: cgroupfs_thrift::IoStat) -> Self {
        Self {
            rbytes: io_stat.rbytes.map(|x| x.try_into().unwrap()),
            wbytes: io_stat.wbytes.map(|x| x.try_into().unwrap()),
            rios: io_stat.rios.map(|x| x.try_into().unwrap()),
            wios: io_stat.wios.map(|x| x.try_into().unwrap()),
            dbytes: io_stat.dbytes.map(|x| x.try_into().unwrap()),
            dios: io_stat.dios.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<cgroupfs_thrift::MemoryStat> for MemoryStat {
    fn from(memory_stat: cgroupfs_thrift::MemoryStat) -> Self {
        Self {
            anon: memory_stat.anon.map(|x| x.try_into().unwrap()),
            file: memory_stat.file.map(|x| x.try_into().unwrap()),
            kernel_stack: memory_stat.kernel_stack.map(|x| x.try_into().unwrap()),
            slab: memory_stat.slab.map(|x| x.try_into().unwrap()),
            sock: memory_stat.sock.map(|x| x.try_into().unwrap()),
            shmem: memory_stat.shmem.map(|x| x.try_into().unwrap()),
            file_mapped: memory_stat.file_mapped.map(|x| x.try_into().unwrap()),
            file_dirty: memory_stat.file_dirty.map(|x| x.try_into().unwrap()),
            file_writeback: memory_stat.file_writeback.map(|x| x.try_into().unwrap()),
            anon_thp: memory_stat.anon_thp.map(|x| x.try_into().unwrap()),
            inactive_anon: memory_stat.inactive_anon.map(|x| x.try_into().unwrap()),
            active_anon: memory_stat.active_anon.map(|x| x.try_into().unwrap()),
            inactive_file: memory_stat.inactive_file.map(|x| x.try_into().unwrap()),
            active_file: memory_stat.active_file.map(|x| x.try_into().unwrap()),
            unevictable: memory_stat.unevictable.map(|x| x.try_into().unwrap()),
            slab_reclaimable: memory_stat.slab_reclaimable.map(|x| x.try_into().unwrap()),
            slab_unreclaimable: memory_stat
                .slab_unreclaimable
                .map(|x| x.try_into().unwrap()),
            pgfault: memory_stat.pgfault.map(|x| x.try_into().unwrap()),
            pgmajfault: memory_stat.pgmajfault.map(|x| x.try_into().unwrap()),
            workingset_refault: memory_stat
                .workingset_refault
                .map(|x| x.try_into().unwrap()),
            workingset_activate: memory_stat
                .workingset_activate
                .map(|x| x.try_into().unwrap()),
            workingset_nodereclaim: memory_stat
                .workingset_nodereclaim
                .map(|x| x.try_into().unwrap()),
            pgrefill: memory_stat.pgrefill.map(|x| x.try_into().unwrap()),
            pgscan: memory_stat.pgscan.map(|x| x.try_into().unwrap()),
            pgsteal: memory_stat.pgsteal.map(|x| x.try_into().unwrap()),
            pgactivate: memory_stat.pgactivate.map(|x| x.try_into().unwrap()),
            pgdeactivate: memory_stat.pgdeactivate.map(|x| x.try_into().unwrap()),
            pglazyfree: memory_stat.pglazyfree.map(|x| x.try_into().unwrap()),
            pglazyfreed: memory_stat.pglazyfreed.map(|x| x.try_into().unwrap()),
            thp_fault_alloc: memory_stat.thp_fault_alloc.map(|x| x.try_into().unwrap()),
            thp_collapse_alloc: memory_stat
                .thp_collapse_alloc
                .map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<cgroupfs_thrift::PressureMetrics> for PressureMetrics {
    fn from(pressure_metrics: cgroupfs_thrift::PressureMetrics) -> Self {
        Self {
            avg10: pressure_metrics.avg10,
            avg60: pressure_metrics.avg60,
            avg300: pressure_metrics.avg300,
            total: pressure_metrics.total.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<cgroupfs_thrift::CpuPressure> for CpuPressure {
    fn from(cpu_pressure: cgroupfs_thrift::CpuPressure) -> Self {
        Self {
            some: cpu_pressure.some.into(),
        }
    }
}

impl From<cgroupfs_thrift::IoPressure> for IoPressure {
    fn from(io_pressure: cgroupfs_thrift::IoPressure) -> Self {
        Self {
            some: io_pressure.some.into(),
            full: io_pressure.full.into(),
        }
    }
}

impl From<cgroupfs_thrift::MemoryPressure> for MemoryPressure {
    fn from(memory_pressure: cgroupfs_thrift::MemoryPressure) -> Self {
        Self {
            some: memory_pressure.some.into(),
            full: memory_pressure.full.into(),
        }
    }
}

impl From<cgroupfs_thrift::Pressure> for Pressure {
    fn from(pressure: cgroupfs_thrift::Pressure) -> Self {
        Self {
            cpu: pressure.cpu.into(),
            io: pressure.io.into(),
            memory: pressure.memory.into(),
        }
    }
}

impl From<cgroupfs_thrift::MemoryEvents> for MemoryEvents {
    fn from(memory_events: cgroupfs_thrift::MemoryEvents) -> Self {
        Self {
            low: memory_events.low.map(|x| x.try_into().unwrap()),
            high: memory_events.high.map(|x| x.try_into().unwrap()),
            max: memory_events.max.map(|x| x.try_into().unwrap()),
            oom: memory_events.oom.map(|x| x.try_into().unwrap()),
            oom_kill: memory_events.oom_kill.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<CpuStat> for cgroupfs_thrift::CpuStat {
    fn from(cpu_stat: CpuStat) -> Self {
        Self {
            usage_usec: cpu_stat.usage_usec.map(|x| x.try_into().unwrap()),
            user_usec: cpu_stat.user_usec.map(|x| x.try_into().unwrap()),
            system_usec: cpu_stat.system_usec.map(|x| x.try_into().unwrap()),
            nr_periods: cpu_stat.nr_periods.map(|x| x.try_into().unwrap()),
            nr_throttled: cpu_stat.nr_throttled.map(|x| x.try_into().unwrap()),
            throttled_usec: cpu_stat.throttled_usec.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<IoStat> for cgroupfs_thrift::IoStat {
    fn from(io_stat: IoStat) -> Self {
        Self {
            rbytes: io_stat.rbytes.map(|x| x.try_into().unwrap()),
            wbytes: io_stat.wbytes.map(|x| x.try_into().unwrap()),
            rios: io_stat.rios.map(|x| x.try_into().unwrap()),
            wios: io_stat.wios.map(|x| x.try_into().unwrap()),
            dbytes: io_stat.dbytes.map(|x| x.try_into().unwrap()),
            dios: io_stat.dios.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<MemoryStat> for cgroupfs_thrift::MemoryStat {
    fn from(memory_stat: MemoryStat) -> Self {
        Self {
            anon: memory_stat.anon.map(|x| x.try_into().unwrap()),
            file: memory_stat.file.map(|x| x.try_into().unwrap()),
            kernel_stack: memory_stat.kernel_stack.map(|x| x.try_into().unwrap()),
            slab: memory_stat.slab.map(|x| x.try_into().unwrap()),
            sock: memory_stat.sock.map(|x| x.try_into().unwrap()),
            shmem: memory_stat.shmem.map(|x| x.try_into().unwrap()),
            file_mapped: memory_stat.file_mapped.map(|x| x.try_into().unwrap()),
            file_dirty: memory_stat.file_dirty.map(|x| x.try_into().unwrap()),
            file_writeback: memory_stat.file_writeback.map(|x| x.try_into().unwrap()),
            anon_thp: memory_stat.anon_thp.map(|x| x.try_into().unwrap()),
            inactive_anon: memory_stat.inactive_anon.map(|x| x.try_into().unwrap()),
            active_anon: memory_stat.active_anon.map(|x| x.try_into().unwrap()),
            inactive_file: memory_stat.inactive_file.map(|x| x.try_into().unwrap()),
            active_file: memory_stat.active_file.map(|x| x.try_into().unwrap()),
            unevictable: memory_stat.unevictable.map(|x| x.try_into().unwrap()),
            slab_reclaimable: memory_stat.slab_reclaimable.map(|x| x.try_into().unwrap()),
            slab_unreclaimable: memory_stat
                .slab_unreclaimable
                .map(|x| x.try_into().unwrap()),
            pgfault: memory_stat.pgfault.map(|x| x.try_into().unwrap()),
            pgmajfault: memory_stat.pgmajfault.map(|x| x.try_into().unwrap()),
            workingset_refault: memory_stat
                .workingset_refault
                .map(|x| x.try_into().unwrap()),
            workingset_activate: memory_stat
                .workingset_activate
                .map(|x| x.try_into().unwrap()),
            workingset_nodereclaim: memory_stat
                .workingset_nodereclaim
                .map(|x| x.try_into().unwrap()),
            pgrefill: memory_stat.pgrefill.map(|x| x.try_into().unwrap()),
            pgscan: memory_stat.pgscan.map(|x| x.try_into().unwrap()),
            pgsteal: memory_stat.pgsteal.map(|x| x.try_into().unwrap()),
            pgactivate: memory_stat.pgactivate.map(|x| x.try_into().unwrap()),
            pgdeactivate: memory_stat.pgdeactivate.map(|x| x.try_into().unwrap()),
            pglazyfree: memory_stat.pglazyfree.map(|x| x.try_into().unwrap()),
            pglazyfreed: memory_stat.pglazyfreed.map(|x| x.try_into().unwrap()),
            thp_fault_alloc: memory_stat.thp_fault_alloc.map(|x| x.try_into().unwrap()),
            thp_collapse_alloc: memory_stat
                .thp_collapse_alloc
                .map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<PressureMetrics> for cgroupfs_thrift::PressureMetrics {
    fn from(pressure_metrics: PressureMetrics) -> Self {
        Self {
            avg10: pressure_metrics.avg10,
            avg60: pressure_metrics.avg60,
            avg300: pressure_metrics.avg300,
            total: pressure_metrics.total.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<CpuPressure> for cgroupfs_thrift::CpuPressure {
    fn from(cpu_pressure: CpuPressure) -> Self {
        Self {
            some: cpu_pressure.some.into(),
        }
    }
}

impl From<IoPressure> for cgroupfs_thrift::IoPressure {
    fn from(io_pressure: IoPressure) -> Self {
        Self {
            some: io_pressure.some.into(),
            full: io_pressure.full.into(),
        }
    }
}

impl From<MemoryPressure> for cgroupfs_thrift::MemoryPressure {
    fn from(memory_pressure: MemoryPressure) -> Self {
        Self {
            some: memory_pressure.some.into(),
            full: memory_pressure.full.into(),
        }
    }
}

impl From<Pressure> for cgroupfs_thrift::Pressure {
    fn from(pressure: Pressure) -> Self {
        Self {
            cpu: pressure.cpu.into(),
            io: pressure.io.into(),
            memory: pressure.memory.into(),
        }
    }
}

impl From<MemoryEvents> for cgroupfs_thrift::MemoryEvents {
    fn from(memory_events: MemoryEvents) -> Self {
        Self {
            low: memory_events.low.map(|x| x.try_into().unwrap()),
            high: memory_events.high.map(|x| x.try_into().unwrap()),
            max: memory_events.max.map(|x| x.try_into().unwrap()),
            oom: memory_events.oom.map(|x| x.try_into().unwrap()),
            oom_kill: memory_events.oom_kill.map(|x| x.try_into().unwrap()),
        }
    }
}
