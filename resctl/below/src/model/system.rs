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

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct SystemModel {
    #[bttr(title = "Hostname")]
    pub hostname: String,
    pub cpu: Option<CpuModel>,
    pub mem: Option<MemoryModel>,
    pub io: Option<IoModel>,
}

impl SystemModel {
    pub fn new(
        sample: &SystemSample,
        last: Option<(&SystemSample, Duration)>,
        process_sample: &procfs::PidMap,
        process_last: Option<(&procfs::PidMap, Duration)>,
    ) -> SystemModel {
        let cpu = last.and_then(|(last, _)| {
            match (last.stat.total_cpu.as_ref(), sample.stat.total_cpu.as_ref()) {
                (Some(begin), Some(end)) => Some(CpuModel::new(begin, end)),
                _ => None,
            }
        });

        let mem = Some(MemoryModel::new(&sample.meminfo));

        let io = process_last.map(|last| IoModel::new(process_sample, Some(last)));

        SystemModel {
            hostname: sample.hostname.clone(),
            cpu,
            mem,
            io,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct CpuModel {
    #[bttr(
        title = "Usage",
        width = 10,
        title_width = 7,
        unit = "%",
        precision = 2
    )]
    pub usage_pct: Option<f64>,
    #[bttr(title = "User", width = 10, title_width = 7, unit = "%", precision = 2)]
    pub user_pct: Option<f64>,
    #[bttr(
        title = "System",
        width = 10,
        title_width = 7,
        unit = "%",
        precision = 2
    )]
    pub system_pct: Option<f64>,
}

impl CpuModel {
    fn new(begin: &procfs::CpuStat, end: &procfs::CpuStat) -> CpuModel {
        match (begin, end) {
            // guest and guest_nice are ignored
            (
                procfs::CpuStat {
                    user_usec: Some(prev_user),
                    nice_usec: Some(prev_nice),
                    system_usec: Some(prev_system),
                    idle_usec: Some(prev_idle),
                    iowait_usec: Some(prev_iowait),
                    irq_usec: Some(prev_irq),
                    softirq_usec: Some(prev_softirq),
                    stolen_usec: Some(prev_stolen),
                    ..
                },
                procfs::CpuStat {
                    user_usec: Some(curr_user),
                    nice_usec: Some(curr_nice),
                    system_usec: Some(curr_system),
                    idle_usec: Some(curr_idle),
                    iowait_usec: Some(curr_iowait),
                    irq_usec: Some(curr_irq),
                    softirq_usec: Some(curr_softirq),
                    stolen_usec: Some(curr_stolen),
                    ..
                },
            ) => {
                let idle_usec = (curr_idle + curr_iowait) - (prev_idle + prev_iowait);
                let user_usec = curr_user - prev_user;
                let system_usec = curr_system - prev_system;
                let busy_usec =
                    user_usec + system_usec + (curr_nice + curr_irq + curr_softirq + curr_stolen)
                        - (prev_nice + prev_irq + prev_softirq + prev_stolen);
                let total_usec = idle_usec + busy_usec;
                CpuModel {
                    usage_pct: Some(busy_usec as f64 * 100.0 / total_usec as f64),
                    user_pct: Some(user_usec as f64 * 100.0 / total_usec as f64),
                    system_pct: Some(system_usec as f64 * 100.0 / total_usec as f64),
                }
            }
            _ => CpuModel {
                usage_pct: None,
                user_pct: None,
                system_pct: None,
            },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct MemoryModel {
    #[bttr(
        title = "Total",
        width = 10,
        title_width = 7,
        decorator = "convert_bytes($ as f64)"
    )]
    pub total: Option<u64>,
    #[bttr(
        title = "Free",
        width = 10,
        title_width = 7,
        decorator = "convert_bytes($ as f64)"
    )]
    pub free: Option<u64>,
    #[bttr(
        title = "Anon",
        width = 10,
        title_width = 7,
        decorator = "convert_bytes($ as f64)"
    )]
    pub anon: Option<u64>,
    #[bttr(
        title = "File",
        width = 10,
        title_width = 7,
        decorator = "convert_bytes($ as f64)"
    )]
    pub file: Option<u64>,
    #[bttr(
        title = "Huge Total",
        width = 10,
        title_width = 11,
        decorator = "convert_bytes($ as f64)"
    )]
    pub hugepage_total: Option<u64>,
    #[bttr(
        title = "Huge Free",
        width = 10,
        title_width = 10,
        decorator = "convert_bytes($ as f64)"
    )]
    pub hugepage_free: Option<u64>,
}

impl MemoryModel {
    fn new(meminfo: &procfs::MemInfo) -> MemoryModel {
        MemoryModel {
            total: meminfo.total.map(|v| v as u64),
            free: meminfo.free.map(|v| v as u64),
            anon: opt_add(meminfo.active_anon, meminfo.inactive_anon).map(|x| x as u64),
            file: opt_add(meminfo.active_file, meminfo.inactive_file).map(|x| x as u64),
            hugepage_total: opt_multiply(meminfo.total_huge_pages, meminfo.huge_page_size)
                .map(|x| x as u64),
            hugepage_free: opt_multiply(meminfo.free_huge_pages, meminfo.huge_page_size)
                .map(|x| x as u64),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct IoModel {
    #[bttr(
        title = "Reads",
        width = 10,
        title_width = 7,
        decorator = "convert_bytes($)",
        unit = "/s"
    )]
    pub rbytes_per_sec: Option<f64>,
    #[bttr(
        title = "Writes",
        width = 10,
        title_width = 7,
        decorator = "convert_bytes($)",
        unit = "/s"
    )]
    pub wbytes_per_sec: Option<f64>,
}

impl IoModel {
    fn new(sample: &procfs::PidMap, last: Option<(&procfs::PidMap, Duration)>) -> IoModel {
        let mut rbytes = 0.0;
        let mut wbytes = 0.0;

        let process_model = ProcessModel::new(sample, last);
        for (_, spm) in process_model.processes.iter() {
            rbytes += spm
                .io
                .as_ref()
                .map_or(0.0, |io| io.rbytes_per_sec.map_or(0.0, |n| n));

            wbytes += spm
                .io
                .as_ref()
                .map_or(0.0, |io| io.wbytes_per_sec.map_or(0.0, |n| n));
        }

        IoModel {
            rbytes_per_sec: Some(rbytes),
            wbytes_per_sec: Some(wbytes),
        }
    }
}
