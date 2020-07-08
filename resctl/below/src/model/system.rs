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
    #[bttr(title = "Hostname", width = 20)]
    pub hostname: String,
    pub cpu: CpuModel,
    pub mem: MemoryModel,
    pub vm: VmModel,
    pub disks: BTreeMap<String, SingleDiskModel>,
}

impl SystemModel {
    pub fn new(sample: &SystemSample, last: Option<(&SystemSample, Duration)>) -> SystemModel {
        let cpu = last
            .map(|(last, _)| CpuModel::new(&last.stat, &sample.stat))
            .unwrap_or_default();

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
            cpu,
            mem,
            vm,
            disks,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct CpuModel {
    pub total_cpu: Option<SingleCpuModel>,
    pub cpus: Option<Vec<SingleCpuModel>>,
    #[bttr(title = "Total Interrupt")]
    pub total_interrupt_ct: Option<i64>,
    #[bttr(title = "Context Switch")]
    pub context_switches: Option<i64>,
    #[bttr(title = "Boot Time Epoch", unit = " s")]
    pub boot_time_epoch_secs: Option<i64>,
    #[bttr(title = "Total Procs")]
    pub total_processes: Option<i64>,
    #[bttr(title = "Running Procs")]
    pub running_processes: Option<i32>,
    #[bttr(title = "Blocked Procs")]
    pub blocked_processes: Option<i32>,
}

impl CpuModel {
    pub fn new(begin: &procfs::Stat, end: &procfs::Stat) -> CpuModel {
        CpuModel {
            total_cpu: match (&begin.total_cpu, &end.total_cpu) {
                (Some(prev), Some(curr)) => Some(SingleCpuModel::new(-1, &prev, &curr)),
                _ => None,
            },
            cpus: match (&begin.cpus, &end.cpus) {
                (Some(prev), Some(curr)) => {
                    let mut idx: i32 = 0;
                    Some(
                        prev.iter()
                            .zip(curr.iter())
                            .map(|(prev, curr)| {
                                let res = SingleCpuModel::new(idx, prev, curr);
                                idx += 1;
                                res
                            })
                            .collect(),
                    )
                }
                (_, Some(curr)) => Some(curr.iter().map(|_| Default::default()).collect()),
                _ => None,
            },
            total_interrupt_ct: end.total_interrupt_count,
            context_switches: end.context_switches,
            boot_time_epoch_secs: end.boot_time_epoch_secs,
            total_processes: end.total_processes,
            running_processes: end.running_processes,
            blocked_processes: end.blocked_processes,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct SingleCpuModel {
    #[bttr(
        title = "idx",
        width = 10,
        decorator = "if self.idx == -1 { \"total\".into() } else { self.idx.to_string() }"
    )]
    pub idx: i32,
    #[bttr(title = "Usage", unit = "%", width = 10, precision = 2)]
    pub usage_pct: Option<f64>,
    #[bttr(title = "User", unit = "%", width = 10, precision = 2)]
    pub user_pct: Option<f64>,
    #[bttr(title = "Idle", unit = "%", width = 10, precision = 2)]
    pub idle_pct: Option<f64>,
    #[bttr(title = "System", unit = "%", width = 10, precision = 2)]
    pub system_pct: Option<f64>,
    #[bttr(title = "Nice", unit = "%", width = 10, precision = 2)]
    pub nice_pct: Option<f64>,
    #[bttr(title = "IOWait", unit = "%", width = 10, precision = 2)]
    pub iowait_pct: Option<f64>,
    #[bttr(title = "Irq", unit = "%", width = 10, precision = 2)]
    pub irq_pct: Option<f64>,
    #[bttr(title = "SoftIrq", unit = "%", width = 10, precision = 2)]
    pub softirq_pct: Option<f64>,
    #[bttr(title = "Stolen", unit = "%", width = 10, precision = 2)]
    pub stolen_pct: Option<f64>,
    #[bttr(title = "Guest", unit = "%", width = 10, precision = 2)]
    pub guest_pct: Option<f64>,
    #[bttr(title = "Guest Nice", unit = "%", width = 10, precision = 2)]
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

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct MemoryModel {
    #[bttr(title = "Total", decorator = "convert_bytes($ as f64)", width = 20)]
    pub total: Option<u64>,
    #[bttr(title = "Free", decorator = "convert_bytes($ as f64)", width = 20)]
    pub free: Option<u64>,
    #[bttr(title = "Available", width = 20, decorator = "convert_bytes($ as f64)")]
    pub available: Option<u64>,
    #[bttr(title = "Buffers", width = 20, decorator = "convert_bytes($ as f64)")]
    pub buffers: Option<u64>,
    #[bttr(title = "Cached", width = 20, decorator = "convert_bytes($ as f64)")]
    pub cached: Option<u64>,
    #[bttr(
        title = "Swap Cached",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub swap_cached: Option<u64>,
    #[bttr(title = "Active", width = 20, decorator = "convert_bytes($ as f64)")]
    pub active: Option<u64>,
    #[bttr(title = "Inactive", width = 20, decorator = "convert_bytes($ as f64)")]
    pub inactive: Option<u64>,
    #[bttr(title = "Anon", width = 20, decorator = "convert_bytes($ as f64)")]
    pub anon: Option<u64>,
    #[bttr(title = "File", width = 20, decorator = "convert_bytes($ as f64)")]
    pub file: Option<u64>,
    #[bttr(
        title = "Unevictable",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub unevictable: Option<u64>,
    #[bttr(title = "Mlocked", width = 20, decorator = "convert_bytes($ as f64)")]
    pub mlocked: Option<u64>,
    #[bttr(
        title = "Swap Total",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub swap_total: Option<u64>,
    #[bttr(title = "Swap Free", width = 20, decorator = "convert_bytes($ as f64)")]
    pub swap_free: Option<u64>,
    #[bttr(title = "Dirty", width = 20, decorator = "convert_bytes($ as f64)")]
    pub dirty: Option<u64>,
    #[bttr(title = "Writeback", width = 20, decorator = "convert_bytes($ as f64)")]
    pub writeback: Option<u64>,
    #[bttr(
        title = "Anon Pages",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub anon_pages: Option<u64>,
    #[bttr(title = "Mappedn", width = 20, decorator = "convert_bytes($ as f64)")]
    pub mapped: Option<u64>,
    #[bttr(title = "Shmem", width = 20, decorator = "convert_bytes($ as f64)")]
    pub shmem: Option<u64>,
    #[bttr(
        title = "Kreclaimable",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub kreclaimable: Option<u64>,
    #[bttr(title = "Slab", width = 20, decorator = "convert_bytes($ as f64)")]
    pub slab: Option<u64>,
    #[bttr(
        title = "Slab Reclaimable",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub slab_reclaimable: Option<u64>,
    #[bttr(
        title = "Slab Unreclaimable",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub slab_unreclaimable: Option<u64>,
    #[bttr(
        title = "Kernel Stack",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub kernel_stack: Option<u64>,
    #[bttr(
        title = "Page Tables",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub page_tables: Option<u64>,
    #[bttr(
        title = "Anon Huge Pages",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub anon_huge_pages_bytes: Option<u64>,
    #[bttr(
        title = "Shmem Huge Pages",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub shmem_huge_pages_bytes: Option<u64>,
    #[bttr(
        title = "File Huge Pages",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub file_huge_pages_bytes: Option<u64>,
    #[bttr(
        title = "Total Huge Pages",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub total_huge_pages_bytes: Option<u64>,
    #[bttr(
        title = "Free Huge Pages",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub free_huge_pages_bytes: Option<u64>,
    #[bttr(
        title = "Huge Page Size",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub huge_page_size: Option<u64>,
    #[bttr(title = "Cma Total", width = 20, decorator = "convert_bytes($ as f64)")]
    pub cma_total: Option<u64>,
    #[bttr(title = "Cma Free", width = 20, decorator = "convert_bytes($ as f64)")]
    pub cma_free: Option<u64>,
    #[bttr(
        title = "Vmalloc Total",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub vmalloc_total: Option<u64>,
    #[bttr(
        title = "Vmalloc Used",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub vmalloc_used: Option<u64>,
    #[bttr(
        title = "Vmalloc Chunk",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub vmalloc_chunk: Option<u64>,
    #[bttr(
        title = "Direct Map 4K",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub direct_map_4k: Option<u64>,
    #[bttr(
        title = "Direct Map 2M",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
    pub direct_map_2m: Option<u64>,
    #[bttr(
        title = "Direct Map 1G",
        width = 20,
        decorator = "convert_bytes($ as f64)"
    )]
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
            total_huge_pages_bytes: opt_multiply(meminfo.total_huge_pages, meminfo.huge_page_size)
                .map(|x| x as u64),
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

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct VmModel {
    #[bttr(
        title = "Page In",
        width = 20,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub pgpgin_per_sec: Option<f64>,
    #[bttr(
        title = "Page Out",
        width = 20,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub pgpgout_per_sec: Option<f64>,
    #[bttr(
        title = "Swap In",
        width = 20,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub pswpin_per_sec: Option<f64>,
    #[bttr(
        title = "Swap Out",
        width = 20,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub pswpout_per_sec: Option<f64>,
    #[bttr(title = "Pgsteal Kswapd", width = 20, unit = " pages")]
    pub pgsteal_kswapd: Option<u64>,
    #[bttr(title = "Pgsteal Direct", width = 20, unit = " pages")]
    pub pgsteal_direct: Option<u64>,
    #[bttr(title = "Pgscan Kswapd", width = 20, unit = " pages")]
    pub pgscan_kswapd: Option<u64>,
    #[bttr(title = "Pgscan Direct", width = 20, unit = " pages")]
    pub pgscan_direct: Option<u64>,
    #[bttr(title = "OOM Kills", width = 20)]
    pub oom_kill: Option<u64>,
}

impl VmModel {
    fn new(begin: &procfs::VmStat, end: &procfs::VmStat, duration: Duration) -> VmModel {
        VmModel {
            pgpgin_per_sec: count_per_sec!(begin.pgpgin, end.pgpgin, duration),
            pgpgout_per_sec: count_per_sec!(begin.pgpgout, end.pgpgout, duration),
            pswpin_per_sec: count_per_sec!(begin.pswpin, end.pswpin, duration),
            pswpout_per_sec: count_per_sec!(begin.pswpout, end.pswpout, duration),
            pgsteal_kswapd: end.pgsteal_kswapd.map(|v| v as u64),
            pgsteal_direct: end.pgsteal_direct.map(|v| v as u64),
            pgscan_kswapd: end.pgscan_kswapd.map(|v| v as u64),
            pgscan_direct: end.pgscan_direct.map(|v| v as u64),
            oom_kill: end.oom_kill.map(|v| v as u64),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, BelowDecor)]
pub struct SingleDiskModel {
    #[bttr(
        title = "Name",
        width = 15,
        depth = "self.depth * 3",
        prefix = "get_prefix(self.collapse)"
    )]
    pub name: Option<String>,
    #[bttr(
        title = "Read",
        width = 10,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub read_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "Write",
        width = 10,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub write_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "Discard",
        width = 10,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub discard_bytes_per_sec: Option<f64>,
    #[bttr(
        title = "Disk",
        width = 10,
        decorator = "convert_bytes($ as f64)",
        unit = "/s"
    )]
    pub disk_total_bytes_per_sec: Option<f64>,
    #[bttr(title = "Read Completed", width = 15)]
    pub read_completed: Option<u64>,
    #[bttr(title = "Read Merged", width = 15)]
    pub read_merged: Option<u64>,
    #[bttr(title = "Read Sectors", width = 15)]
    pub read_sectors: Option<u64>,
    #[bttr(title = "Time Spend Read", width = 15, unit = " ms")]
    pub time_spend_read_ms: Option<u64>,
    #[bttr(title = "Write Completed", width = 15)]
    pub write_completed: Option<u64>,
    #[bttr(title = "Write Merged", width = 15)]
    pub write_merged: Option<u64>,
    #[bttr(title = "Write Sectors", width = 15)]
    pub write_sectors: Option<u64>,
    #[bttr(title = "Time Spend Write", width = 15, unit = " ms")]
    pub time_spend_write_ms: Option<u64>,
    #[bttr(title = "Discard Completed", width = 15)]
    pub discard_completed: Option<u64>,
    #[bttr(title = "Discard Merged", width = 15)]
    pub discard_merged: Option<u64>,
    #[bttr(title = "Discard Sectors", width = 15)]
    pub discard_sectors: Option<u64>,
    #[bttr(title = "Time Spend Discard", width = 15, unit = " ms")]
    pub time_spend_discard_ms: Option<u64>,
    #[bttr(title = "Major", width = 7)]
    pub major: Option<u64>,
    #[bttr(title = "Minor", width = 7)]
    pub minor: Option<u64>,
    pub collapse: bool,
    pub depth: usize,
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
            collapse: false,
            depth: if end.minor == Some(0) { 0 } else { 1 },
        }
    }
}
