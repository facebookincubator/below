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

#[derive(Clone, Debug, Default, BelowDecor)]
pub struct CgroupModel {
    #[bttr(title = "Name", width = 50)]
    pub name: String,
    #[bttr(title = "Full Path", width = 50)]
    pub full_path: String,
    pub depth: u32,
    pub cpu: Option<CgroupCpuModel>,
    pub memory: Option<CgroupMemoryModel>,
    pub io: Option<BTreeMap<String, CgroupIoModel>>,
    pub io_total: Option<CgroupIoModel>,
    pub pressure: Option<CgroupPressureModel>,
    pub children: BTreeSet<CgroupModel>,
    pub count: u32,
}

// We implement equality and ordering based on the cgroup name only so
// CgroupModel can be stored in a BTreeSet
impl Ord for CgroupModel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for CgroupModel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CgroupModel {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CgroupModel {}

impl CgroupModel {
    pub fn new(
        name: String,
        full_path: String,
        depth: u32,
        sample: &CgroupSample,
        last: Option<(&CgroupSample, Duration)>,
    ) -> CgroupModel {
        let (cpu, io, io_total) = if let Some((last, delta)) = last {
            // We have cumulative data, create cpu, io models
            let cpu = match (last.cpu_stat.as_ref(), sample.cpu_stat.as_ref()) {
                (Some(begin), Some(end)) => Some(CgroupCpuModel::new(begin, end, delta)),
                _ => None,
            };
            let io = match (last.io_stat.as_ref(), sample.io_stat.as_ref()) {
                (Some(begin), Some(end)) => Some(
                    end.iter()
                        .filter_map(|(device_name, end_io_stat)| {
                            begin.get(device_name).map(|begin_io_stat| {
                                (
                                    device_name.clone(),
                                    CgroupIoModel::new(&begin_io_stat, &end_io_stat, delta),
                                )
                            })
                        })
                        .collect::<BTreeMap<String, CgroupIoModel>>(),
                ),
                _ => None,
            };
            let io_total = io.as_ref().map(|io_map| {
                io_map
                    .iter()
                    .fold(CgroupIoModel::empty(), |acc, (_, model)| acc + model)
            });

            (cpu, io, io_total)
        } else {
            // No cumulative data
            (None, None, None)
        };

        let memory = Some(CgroupMemoryModel::new(sample, last));

        let pressure = sample
            .pressure
            .as_ref()
            .map(|p| CgroupPressureModel::new(p));

        // recursively calculate view of children
        // `children` is optional, but we treat it the same as an empty map
        let empty = BTreeMap::new();
        let children = sample
            .children
            .as_ref()
            .unwrap_or(&empty)
            .iter()
            .map(|(child_name, child_sample)| {
                CgroupModel::new(
                    child_name.clone(),
                    format!("{}/{}", full_path, child_name),
                    depth + 1,
                    &child_sample,
                    last.and_then(|(last, delta)| {
                        last.children
                            .as_ref()
                            .unwrap_or(&empty)
                            .get(child_name)
                            .map(|child_last| (child_last, delta))
                    }),
                )
            })
            .collect::<BTreeSet<CgroupModel>>();
        let nr_descendants: u32 = children.iter().fold(0, |acc, c| acc + c.count);
        CgroupModel {
            name,
            full_path,
            cpu,
            memory,
            io,
            io_total,
            pressure,
            children,
            count: nr_descendants + 1,
            depth,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct CgroupCpuModel {
    #[bttr(
        title = "CPU",
        width = 15,
        unit = "%",
        precision = 2,
        highlight_if = "is_cpu_significant($)"
    )]
    pub usage_pct: Option<f64>,
    #[bttr(
        title = "CPU User",
        width = 15,
        unit = "%",
        precision = 2,
        highlight_if = "is_cpu_significant($)"
    )]
    pub user_pct: Option<f64>,
    #[bttr(
        title = "CPU Sys",
        width = 15,
        unit = "%",
        precision = 2,
        highlight_if = "is_cpu_significant($)"
    )]
    pub system_pct: Option<f64>,
    #[bttr(title = "Nr Period", width = 15, unit = "/s", precision = 2)]
    pub nr_periods_per_sec: Option<f64>,
    #[bttr(title = "Nr Throttle", width = 15, unit = "/s", precision = 2)]
    pub nr_throttled_per_sec: Option<f64>,
    #[bttr(title = "Throttle", width = 15, unit = "%", precision = 2)]
    pub throttled_pct: Option<f64>,
}

impl CgroupCpuModel {
    pub fn new(
        begin: &cgroupfs::CpuStat,
        end: &cgroupfs::CpuStat,
        delta: Duration,
    ) -> CgroupCpuModel {
        CgroupCpuModel {
            usage_pct: usec_pct!(begin.usage_usec, end.usage_usec, delta),
            user_pct: usec_pct!(begin.user_usec, end.user_usec, delta),
            system_pct: usec_pct!(begin.system_usec, end.system_usec, delta),
            nr_periods_per_sec: count_per_sec!(begin.nr_periods, end.nr_periods, delta),
            nr_throttled_per_sec: count_per_sec!(begin.nr_throttled, end.nr_throttled, delta),
            throttled_pct: usec_pct!(begin.throttled_usec, end.throttled_usec, delta),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct CgroupIoModel {
    #[bttr(
        title = "Reads",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)"
    )]
    pub rbytes_per_sec: Option<f64>,
    #[bttr(
        title = "Writes",
        width = 11,
        unit = "/s",
        decorator = "convert_bytes($ as f64)"
    )]
    pub wbytes_per_sec: Option<f64>,
    #[bttr(title = "R I/O", unit = "/s")]
    pub rios_per_sec: Option<f64>,
    #[bttr(title = "W I/O", unit = "/s")]
    pub wios_per_sec: Option<f64>,
    #[bttr(title = "DBytes", unit = "/s")]
    pub dbytes_per_sec: Option<f64>,
    #[bttr(title = "D I/O", unit = "/s")]
    pub dios_per_sec: Option<f64>,
}

impl CgroupIoModel {
    pub fn new(begin: &cgroupfs::IoStat, end: &cgroupfs::IoStat, delta: Duration) -> CgroupIoModel {
        CgroupIoModel {
            rbytes_per_sec: count_per_sec!(begin.rbytes, end.rbytes, delta),
            wbytes_per_sec: count_per_sec!(begin.wbytes, end.wbytes, delta),
            rios_per_sec: count_per_sec!(begin.rios, end.rios, delta),
            wios_per_sec: count_per_sec!(begin.wios, end.wios, delta),
            dbytes_per_sec: count_per_sec!(begin.dbytes, end.dbytes, delta),
            dios_per_sec: count_per_sec!(begin.dios, end.dios, delta),
        }
    }

    pub fn empty() -> CgroupIoModel {
        // If io.stat file is empty, it means cgroup has no I/O at all. In that
        // case we default to zero instead of None.
        CgroupIoModel {
            rbytes_per_sec: Some(0.0),
            wbytes_per_sec: Some(0.0),
            rios_per_sec: Some(0.0),
            wios_per_sec: Some(0.0),
            dbytes_per_sec: Some(0.0),
            dios_per_sec: Some(0.0),
        }
    }
}

impl std::ops::Add<&CgroupIoModel> for CgroupIoModel {
    type Output = Self;

    fn add(self, other: &Self) -> Self {
        Self {
            rbytes_per_sec: opt_add(self.rbytes_per_sec, other.rbytes_per_sec),
            wbytes_per_sec: opt_add(self.wbytes_per_sec, other.wbytes_per_sec),
            rios_per_sec: opt_add(self.rios_per_sec, other.rios_per_sec),
            wios_per_sec: opt_add(self.wios_per_sec, other.wios_per_sec),
            dbytes_per_sec: opt_add(self.dbytes_per_sec, other.dbytes_per_sec),
            dios_per_sec: opt_add(self.dios_per_sec, other.dios_per_sec),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct CgroupMemoryModel {
    #[bttr(title = "Memory", width = 11, decorator = "convert_bytes($ as f64)")]
    pub total: Option<u64>,
    #[bttr(title = "Memory Swap")]
    pub swap: Option<u64>,
    #[bttr(title = "Anon")]
    pub anon: Option<u64>,
    #[bttr(title = "File")]
    pub file: Option<u64>,
    #[bttr(title = "Kernel Stack")]
    pub kernel_stack: Option<u64>,
    #[bttr(title = "Slab")]
    pub slab: Option<u64>,
    #[bttr(title = "Sock")]
    pub sock: Option<u64>,
    #[bttr(title = "Shmem")]
    pub shmem: Option<u64>,
    #[bttr(title = "File Mapped")]
    pub file_mapped: Option<u64>,
    #[bttr(title = "File Dirty")]
    pub file_dirty: Option<u64>,
    #[bttr(title = "File Writeback")]
    pub file_writeback: Option<u64>,
    #[bttr(title = "Anon Thp")]
    pub anon_thp: Option<u64>,
    #[bttr(title = "Inactive Anon")]
    pub inactive_anon: Option<u64>,
    #[bttr(title = "Active Anon")]
    pub active_anon: Option<u64>,
    #[bttr(title = "Inactive File")]
    pub inactive_file: Option<u64>,
    #[bttr(title = "Active File")]
    pub active_file: Option<u64>,
    #[bttr(title = "Unevictable")]
    pub unevictable: Option<u64>,
    #[bttr(title = "Slab Reclaimable")]
    pub slab_reclaimable: Option<u64>,
    #[bttr(title = "Slab Unreclaimable")]
    pub slab_unreclaimable: Option<u64>,
    #[bttr(title = "Pgfault")]
    pub pgfault: Option<u64>,
    #[bttr(title = "Pgmajfault")]
    pub pgmajfault: Option<u64>,
    #[bttr(title = "Workingset Refault")]
    pub workingset_refault: Option<u64>,
    #[bttr(title = "Workingset Activate")]
    pub workingset_activate: Option<u64>,
    #[bttr(title = "Workingset Nodereclaim")]
    pub workingset_nodereclaim: Option<u64>,
    #[bttr(title = "Pgrefill")]
    pub pgrefill: Option<u64>,
    #[bttr(title = "Pgscan")]
    pub pgscan: Option<u64>,
    #[bttr(title = "Pgsteal")]
    pub pgsteal: Option<u64>,
    #[bttr(title = "Pgactivate")]
    pub pgactivate: Option<u64>,
    #[bttr(title = "Pgdeactivate")]
    pub pgdeactivate: Option<u64>,
    #[bttr(title = "Pglazyfree")]
    pub pglazyfree: Option<u64>,
    #[bttr(title = "Pglazyfreed")]
    pub pglazyfreed: Option<u64>,
    #[bttr(title = "THP Fault Alloc")]
    pub thp_fault_alloc: Option<u64>,
    #[bttr(title = "THP Collapse Alloc")]
    pub thp_collapse_alloc: Option<u64>,
    #[bttr(title = "Memory.High")]
    pub memory_high: Option<i64>,
}

impl CgroupMemoryModel {
    pub fn new(
        sample: &CgroupSample,
        last: Option<(&CgroupSample, Duration)>,
    ) -> CgroupMemoryModel {
        let mut model = CgroupMemoryModel {
            total: sample.memory_current.map(|v| v as u64),
            swap: sample.memory_swap_current.map(|v| v as u64),
            memory_high: sample.memory_high,
            ..Default::default()
        };
        if let Some(stat) = &sample.memory_stat {
            model.anon = stat.anon.map(|v| v as u64);
            model.file = stat.file.map(|v| v as u64);
            model.kernel_stack = stat.kernel_stack.map(|v| v as u64);
            model.slab = stat.slab.map(|v| v as u64);
            model.sock = stat.sock.map(|v| v as u64);
            model.shmem = stat.shmem.map(|v| v as u64);
            model.file_mapped = stat.file_mapped.map(|v| v as u64);
            model.file_dirty = stat.file_dirty.map(|v| v as u64);
            model.file_writeback = stat.file_writeback.map(|v| v as u64);
            model.anon_thp = stat.anon_thp.map(|v| v as u64);
            model.inactive_anon = stat.inactive_anon.map(|v| v as u64);
            model.active_anon = stat.active_anon.map(|v| v as u64);
            model.inactive_file = stat.inactive_file.map(|v| v as u64);
            model.active_file = stat.active_file.map(|v| v as u64);
            model.unevictable = stat.unevictable.map(|v| v as u64);
            model.slab_reclaimable = stat.slab_reclaimable.map(|v| v as u64);
            model.slab_unreclaimable = stat.slab_unreclaimable.map(|v| v as u64);

            if let Some((
                CgroupSample {
                    memory_stat: Some(last_stat),
                    ..
                },
                delta,
            )) = last
            {
                model.pgfault = count_per_sec!(last_stat.pgfault, stat.pgfault, delta, u64);
                model.pgmajfault =
                    count_per_sec!(last_stat.pgmajfault, stat.pgmajfault, delta, u64);
                model.workingset_refault = count_per_sec!(
                    last_stat.workingset_refault,
                    stat.workingset_refault,
                    delta,
                    u64
                );
                model.workingset_activate = count_per_sec!(
                    last_stat.workingset_activate,
                    stat.workingset_activate,
                    delta,
                    u64
                );
                model.workingset_nodereclaim = count_per_sec!(
                    last_stat.workingset_nodereclaim,
                    stat.workingset_nodereclaim,
                    delta,
                    u64
                );
                model.pgrefill = count_per_sec!(last_stat.pgrefill, stat.pgrefill, delta, u64);
                model.pgscan = count_per_sec!(last_stat.pgscan, stat.pgscan, delta, u64);
                model.pgsteal = count_per_sec!(last_stat.pgsteal, stat.pgsteal, delta, u64);
                model.pgactivate =
                    count_per_sec!(last_stat.pgactivate, stat.pgactivate, delta, u64);
                model.pgdeactivate =
                    count_per_sec!(last_stat.pgdeactivate, stat.pgdeactivate, delta, u64);
                model.pglazyfree =
                    count_per_sec!(last_stat.pglazyfree, stat.pglazyfree, delta, u64);
                model.pglazyfreed =
                    count_per_sec!(last_stat.pglazyfreed, stat.pglazyfreed, delta, u64);
                model.thp_fault_alloc =
                    count_per_sec!(last_stat.thp_fault_alloc, stat.thp_fault_alloc, delta, u64);
                model.thp_collapse_alloc = count_per_sec!(
                    last_stat.thp_collapse_alloc,
                    stat.thp_collapse_alloc,
                    delta,
                    u64
                );
            }
        }

        model
    }
}

fn is_pressure_significant(p: f64) -> bool {
    p > 40.0
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct CgroupPressureModel {
    #[bttr(
        title = "CPU Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        highlight_if = "is_pressure_significant($)"
    )]
    pub cpu_some_pct: Option<f64>,
    #[bttr(
        title = "I/O Some Pressure",
        highlight_if = "is_pressure_significant($)"
    )]
    pub io_some_pct: Option<f64>,
    #[bttr(
        title = "I/O Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        highlight_if = "is_pressure_significant($)"
    )]
    pub io_full_pct: Option<f64>,
    #[bttr(
        title = "Mem Some Pressure",
        highlight_if = "is_pressure_significant($)"
    )]
    pub memory_some_pct: Option<f64>,
    #[bttr(
        title = "Mem Pressure",
        width = 15,
        unit = "%",
        precision = 2,
        highlight_if = "is_pressure_significant($)"
    )]
    pub memory_full_pct: Option<f64>,
}

impl CgroupPressureModel {
    fn new(pressure: &cgroupfs::Pressure) -> CgroupPressureModel {
        // Use avg10 instead of calculating pressure with the total metric. If
        // elapsed time between reading pressure total and recording time is too
        // long, pressure could exceed 100%.
        CgroupPressureModel {
            cpu_some_pct: pressure.cpu.some.avg10,
            io_some_pct: pressure.io.some.avg10,
            io_full_pct: pressure.io.full.avg10,
            memory_some_pct: pressure.memory.some.avg10,
            memory_full_pct: pressure.memory.full.avg10,
        }
    }
}
