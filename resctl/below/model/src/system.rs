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

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct SystemModel {
    pub hostname: String,
    pub kernel_version: Option<String>,
    pub os_release: Option<String>,
    #[queriable(subquery)]
    pub stat: ProcStatModel,
    #[queriable(subquery)]
    #[queriable(preferred_name = cpu)]
    pub total_cpu: SingleCpuModel,
    #[queriable(subquery)]
    pub cpus: Vec<SingleCpuModel>,
    #[queriable(subquery)]
    pub mem: MemoryModel,
    #[queriable(subquery)]
    pub vm: VmModel,
    #[queriable(ignore)]
    pub disks: BTreeMap<String, SingleDiskModel>,
}

impl SystemModel {
    pub fn new(sample: &SystemSample, last: Option<(&SystemSample, Duration)>) -> SystemModel {
        let stat = ProcStatModel::new(&sample.stat);
        let total_cpu = match (
            last.and_then(|(last, _)| last.stat.total_cpu.as_ref()),
            sample.stat.total_cpu.as_ref(),
        ) {
            (Some(prev), Some(curr)) => SingleCpuModel::new(-1, &prev, &curr),
            _ => Default::default(),
        };
        let cpus = match (
            last.and_then(|(last, _)| last.stat.cpus.as_ref()),
            sample.stat.cpus.as_ref(),
        ) {
            (Some(prev), Some(curr)) => std::iter::successors(Some(0), |idx| Some(idx + 1))
                .zip(prev.iter().zip(curr.iter()))
                .map(|(idx, (prev, curr))| SingleCpuModel::new(idx, prev, curr))
                .collect(),
            (_, Some(curr)) => curr.iter().map(|_| Default::default()).collect(),
            _ => Default::default(),
        };
        let mem = Some(MemoryModel::new(&sample.meminfo)).unwrap_or_default();
        let vm = last
            .map(|(last, duration)| VmModel::new(&last.vmstat, &sample.vmstat, duration))
            .unwrap_or_default();
        let mut disks: BTreeMap<String, SingleDiskModel> = BTreeMap::new();
        sample.disks.iter().for_each(|(disk_name, end_disk_stat)| {
            disks.insert(
                disk_name.clone(),
                match last {
                    Some((last_sample, duration)) if last_sample.disks.contains_key(disk_name) => {
                        SingleDiskModel::new(
                            last_sample.disks.get(disk_name).unwrap(),
                            &end_disk_stat,
                            duration,
                        )
                    }
                    _ => SingleDiskModel {
                        name: Some(disk_name.clone()),
                        ..Default::default()
                    },
                },
            );
        });

        SystemModel {
            hostname: sample.hostname.clone(),
            kernel_version: sample.kernel_version.clone(),
            os_release: sample.os_release.clone(),
            stat,
            total_cpu,
            cpus,
            mem,
            vm,
            disks,
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct ProcStatModel {
    pub total_interrupt_ct: Option<u64>,
    pub context_switches: Option<u64>,
    pub boot_time_epoch_secs: Option<u64>,
    pub total_processes: Option<u64>,
    pub running_processes: Option<u32>,
    pub blocked_processes: Option<u32>,
}

impl ProcStatModel {
    pub fn new(stat: &procfs::Stat) -> Self {
        ProcStatModel {
            total_interrupt_ct: stat.total_interrupt_count,
            context_switches: stat.context_switches,
            boot_time_epoch_secs: stat.boot_time_epoch_secs,
            total_processes: stat.total_processes,
            running_processes: stat.running_processes,
            blocked_processes: stat.blocked_processes,
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct SingleCpuModel {
    pub idx: i32,
    pub usage_pct: Option<f64>,
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
    pub idle_pct: Option<f64>,
    pub nice_pct: Option<f64>,
    pub iowait_pct: Option<f64>,
    pub irq_pct: Option<f64>,
    pub softirq_pct: Option<f64>,
    pub stolen_pct: Option<f64>,
    pub guest_pct: Option<f64>,
    pub guest_nice_pct: Option<f64>,
}

impl SingleCpuModel {
    pub fn new(idx: i32, begin: &procfs::CpuStat, end: &procfs::CpuStat) -> SingleCpuModel {
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
                    guest_usec: Some(prev_guest),
                    guest_nice_usec: Some(prev_guest_nice),
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
                    guest_usec: Some(curr_guest),
                    guest_nice_usec: Some(curr_guest_nice),
                },
            ) => {
                let idle_usec = curr_idle - prev_idle;
                let iowait_usec = curr_iowait - prev_iowait;
                let user_usec = curr_user - prev_user;
                let system_usec = curr_system - prev_system;
                let nice_usec = curr_nice - prev_nice;
                let irq_usec = curr_irq - prev_irq;
                let softirq_usec = curr_softirq - prev_softirq;
                let stolen_usec = curr_stolen - prev_stolen;
                let guest_usec = curr_guest - prev_guest;
                let guest_nice_usec = curr_guest_nice - prev_guest_nice;

                let busy_usec =
                    user_usec + system_usec + nice_usec + irq_usec + softirq_usec + stolen_usec;
                let total_usec = idle_usec + busy_usec + iowait_usec;
                SingleCpuModel {
                    idx,
                    usage_pct: Some(busy_usec as f64 * 100.0 / total_usec as f64),
                    user_pct: Some(user_usec as f64 * 100.0 / total_usec as f64),
                    idle_pct: Some(idle_usec as f64 * 100.0 / total_usec as f64),
                    system_pct: Some(system_usec as f64 * 100.0 / total_usec as f64),
                    nice_pct: Some(nice_usec as f64 * 100.0 / total_usec as f64),
                    iowait_pct: Some(iowait_usec as f64 * 100.0 / total_usec as f64),
                    irq_pct: Some(iowait_usec as f64 * 100.0 / total_usec as f64),
                    softirq_pct: Some(softirq_usec as f64 * 100.0 / total_usec as f64),
                    stolen_pct: Some(stolen_usec as f64 * 100.0 / total_usec as f64),
                    guest_pct: Some(guest_usec as f64 * 100.0 / total_usec as f64),
                    guest_nice_pct: Some(guest_nice_usec as f64 * 100.0 / total_usec as f64),
                }
            }
            _ => SingleCpuModel {
                idx,
                ..Default::default()
            },
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct MemoryModel {
    pub total: Option<u64>,
    pub free: Option<u64>,
    pub available: Option<u64>,
    pub buffers: Option<u64>,
    pub cached: Option<u64>,
    pub swap_cached: Option<u64>,
    pub active: Option<u64>,
    pub inactive: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub unevictable: Option<u64>,
    pub mlocked: Option<u64>,
    pub swap_total: Option<u64>,
    pub swap_free: Option<u64>,
    pub dirty: Option<u64>,
    pub writeback: Option<u64>,
    pub anon_pages: Option<u64>,
    pub mapped: Option<u64>,
    pub shmem: Option<u64>,
    pub kreclaimable: Option<u64>,
    pub slab: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub page_tables: Option<u64>,
    pub anon_huge_pages_bytes: Option<u64>,
    pub shmem_huge_pages_bytes: Option<u64>,
    pub file_huge_pages_bytes: Option<u64>,
    pub hugetlb: Option<u64>,
    pub free_huge_pages_bytes: Option<u64>,
    pub huge_page_size: Option<u64>,
    pub cma_total: Option<u64>,
    pub cma_free: Option<u64>,
    pub vmalloc_total: Option<u64>,
    pub vmalloc_used: Option<u64>,
    pub vmalloc_chunk: Option<u64>,
    pub direct_map_4k: Option<u64>,
    pub direct_map_2m: Option<u64>,
    pub direct_map_1g: Option<u64>,
}

impl MemoryModel {
    fn new(meminfo: &procfs::MemInfo) -> MemoryModel {
        MemoryModel {
            total: meminfo.total.map(|v| v as u64),
            free: meminfo.free.map(|v| v as u64),
            available: meminfo.available.map(|v| v as u64),
            buffers: meminfo.buffers.map(|v| v as u64),
            cached: meminfo.cached.map(|v| v as u64),
            swap_cached: meminfo.swap_cached.map(|v| v as u64),
            active: meminfo.active.map(|v| v as u64),
            inactive: meminfo.inactive.map(|v| v as u64),
            anon: opt_add(meminfo.active_anon, meminfo.inactive_anon).map(|v| v as u64),
            file: opt_add(meminfo.active_file, meminfo.inactive_file).map(|v| v as u64),
            unevictable: meminfo.unevictable.map(|v| v as u64),
            mlocked: meminfo.mlocked.map(|v| v as u64),
            swap_total: meminfo.swap_total.map(|v| v as u64),
            swap_free: meminfo.swap_free.map(|v| v as u64),
            dirty: meminfo.dirty.map(|v| v as u64),
            writeback: meminfo.writeback.map(|v| v as u64),
            anon_pages: meminfo.anon_pages.map(|v| v as u64),
            mapped: meminfo.mapped.map(|v| v as u64),
            shmem: meminfo.shmem.map(|v| v as u64),
            kreclaimable: meminfo.kreclaimable.map(|v| v as u64),
            slab: meminfo.slab.map(|v| v as u64),
            slab_reclaimable: meminfo.slab_reclaimable.map(|v| v as u64),
            slab_unreclaimable: meminfo.slab_unreclaimable.map(|v| v as u64),
            kernel_stack: meminfo.kernel_stack.map(|v| v as u64),
            page_tables: meminfo.page_tables.map(|v| v as u64),
            anon_huge_pages_bytes: meminfo.anon_huge_pages.map(|x| x as u64),
            shmem_huge_pages_bytes: meminfo.shmem_huge_pages.map(|x| x as u64),
            file_huge_pages_bytes: meminfo.file_huge_pages.map(|x| x as u64),
            hugetlb: meminfo.hugetlb.map(|x| x as u64),
            free_huge_pages_bytes: opt_multiply(meminfo.free_huge_pages, meminfo.huge_page_size)
                .map(|x| x as u64),
            huge_page_size: meminfo.huge_page_size.map(|v| v as u64),
            cma_total: meminfo.cma_total.map(|v| v as u64),
            cma_free: meminfo.cma_free.map(|v| v as u64),
            vmalloc_total: meminfo.vmalloc_total.map(|v| v as u64),
            vmalloc_used: meminfo.vmalloc_used.map(|v| v as u64),
            vmalloc_chunk: meminfo.vmalloc_chunk.map(|v| v as u64),
            direct_map_4k: meminfo.direct_map_4k.map(|v| v as u64),
            direct_map_2m: meminfo.direct_map_2m.map(|v| v as u64),
            direct_map_1g: meminfo.direct_map_1g.map(|v| v as u64),
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct VmModel {
    pub pgpgin_per_sec: Option<f64>,
    pub pgpgout_per_sec: Option<f64>,
    pub pswpin_per_sec: Option<f64>,
    pub pswpout_per_sec: Option<f64>,
    pub pgsteal_kswapd: Option<u64>,
    pub pgsteal_direct: Option<u64>,
    pub pgscan_kswapd: Option<u64>,
    pub pgscan_direct: Option<u64>,
    pub oom_kill: Option<u64>,
}

impl VmModel {
    fn new(begin: &procfs::VmStat, end: &procfs::VmStat, duration: Duration) -> VmModel {
        VmModel {
            pgpgin_per_sec: count_per_sec!(begin.pgpgin, end.pgpgin, duration),
            pgpgout_per_sec: count_per_sec!(begin.pgpgout, end.pgpgout, duration),
            pswpin_per_sec: count_per_sec!(begin.pswpin, end.pswpin, duration),
            pswpout_per_sec: count_per_sec!(begin.pswpout, end.pswpout, duration),
            pgsteal_kswapd: count_per_sec!(begin.pgsteal_kswapd, end.pgsteal_kswapd, duration, u64),
            pgsteal_direct: count_per_sec!(begin.pgsteal_direct, end.pgsteal_direct, duration, u64),
            pgscan_kswapd: count_per_sec!(begin.pgscan_kswapd, end.pgscan_kswapd, duration, u64),
            pgscan_direct: count_per_sec!(begin.pgscan_direct, end.pgscan_direct, duration, u64),
            oom_kill: end.oom_kill.map(|v| v as u64),
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct SingleDiskModel {
    pub name: Option<String>,
    pub read_bytes_per_sec: Option<f64>,
    pub write_bytes_per_sec: Option<f64>,
    pub discard_bytes_per_sec: Option<f64>,
    pub disk_total_bytes_per_sec: Option<f64>,
    pub read_completed: Option<u64>,
    pub read_merged: Option<u64>,
    pub read_sectors: Option<u64>,
    pub time_spend_read_ms: Option<u64>,
    pub write_completed: Option<u64>,
    pub write_merged: Option<u64>,
    pub write_sectors: Option<u64>,
    pub time_spend_write_ms: Option<u64>,
    pub discard_completed: Option<u64>,
    pub discard_merged: Option<u64>,
    pub discard_sectors: Option<u64>,
    pub time_spend_discard_ms: Option<u64>,
    pub major: Option<u64>,
    pub minor: Option<u64>,
}

impl Recursive for SingleDiskModel {
    fn get_depth(&self) -> usize {
        if self.minor == Some(0) { 0 } else { 1 }
    }
}

impl SingleDiskModel {
    fn new(
        begin: &procfs::DiskStat,
        end: &procfs::DiskStat,
        duration: Duration,
    ) -> SingleDiskModel {
        let read_bytes_per_sec =
            count_per_sec!(begin.read_sectors, end.read_sectors, duration).map(|val| val * 512.0);
        let write_bytes_per_sec =
            count_per_sec!(begin.write_sectors, end.write_sectors, duration).map(|val| val * 512.0);
        SingleDiskModel {
            name: end.name.clone(),
            read_bytes_per_sec,
            write_bytes_per_sec,
            discard_bytes_per_sec: count_per_sec!(
                begin.discard_sectors,
                end.discard_sectors,
                duration
            )
            .map(|val| val * 512.0),
            disk_total_bytes_per_sec: opt_add(read_bytes_per_sec, write_bytes_per_sec),
            read_completed: end.read_completed.map(|v| v as u64),
            read_merged: end.read_merged.map(|v| v as u64),
            read_sectors: end.read_sectors.map(|v| v as u64),
            time_spend_read_ms: end.time_spend_read_ms.map(|v| v as u64),
            write_completed: end.write_completed.map(|v| v as u64),
            write_merged: end.write_merged.map(|v| v as u64),
            write_sectors: end.write_sectors.map(|v| v as u64),
            time_spend_write_ms: end.time_spend_write_ms.map(|v| v as u64),
            discard_completed: end.discard_completed.map(|v| v as u64),
            discard_merged: end.discard_merged.map(|v| v as u64),
            discard_sectors: end.discard_sectors.map(|v| v as u64),
            time_spend_discard_ms: end.time_spend_discard_ms.map(|v| v as u64),
            major: end.major.map(|v| v as u64),
            minor: end.minor.map(|v| v as u64),
        }
    }
}
