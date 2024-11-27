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

#[::below_derive::queriable_derives]
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
    pub cpus: BTreeMap<u32, SingleCpuModel>,
    #[queriable(subquery)]
    pub mem: MemoryModel,
    #[queriable(subquery)]
    pub vm: VmModel,
    #[queriable(subquery)]
    pub slab: Vec<SingleSlabModel>,
    #[queriable(subquery)]
    pub ksm: Option<KsmModel>,
    #[queriable(subquery)]
    pub disks: BTreeMap<String, SingleDiskModel>,
    #[queriable(subquery)]
    pub btrfs: Option<BTreeMap<String, BtrfsModel>>,
}

impl SystemModel {
    pub fn new(sample: &SystemSample, last: Option<(&SystemSample, Duration)>) -> SystemModel {
        let stat = ProcStatModel::new(&sample.stat);
        let total_cpu = match (
            last.and_then(|(last, _)| last.stat.total_cpu.as_ref()),
            sample.stat.total_cpu.as_ref(),
        ) {
            (Some(prev), Some(curr)) => SingleCpuModel::new(-1, prev, curr),
            _ => Default::default(),
        };

        let cpus: BTreeMap<u32, SingleCpuModel> = match (
            last.and_then(|(last, _)| last.stat.cpus_map.as_ref()),
            sample.stat.cpus_map.as_ref(),
        ) {
            (Some(prev), Some(curr)) => curr
                .iter()
                .map(|(idx, curr)| {
                    (
                        *idx,
                        prev.get(idx).map_or_else(Default::default, |prev| {
                            SingleCpuModel::new(*idx as i32, prev, curr)
                        }),
                    )
                })
                .collect(),
            (_, Some(curr)) => curr.keys().map(|idx| (*idx, Default::default())).collect(),
            _ => Default::default(),
        };

        let mem = MemoryModel::new(&sample.meminfo);
        let vm = last
            .map(|(last, duration)| VmModel::new(&last.vmstat, &sample.vmstat, duration))
            .unwrap_or_default();

        let mut slab = sample
            .slabinfo_vec
            .iter()
            .map(SingleSlabModel::new)
            .collect::<Vec<SingleSlabModel>>();

        let slab_total = slab.iter().fold(
            SingleSlabModel {
                name: Some(String::from("TOTAL")),
                ..Default::default()
            },
            |mut acc, slabinfo| {
                acc.active_objs = opt_add(acc.active_objs, slabinfo.active_objs);
                acc.num_objs = opt_add(acc.num_objs, slabinfo.num_objs);
                acc.num_slabs = opt_add(acc.num_slabs, slabinfo.num_slabs);
                acc.active_caches = opt_add(acc.active_caches, slabinfo.active_caches);
                acc.num_caches = opt_add(acc.num_caches, slabinfo.num_caches);
                acc.active_size = opt_add(acc.active_size, slabinfo.active_size);
                acc.total_size = opt_add(acc.total_size, slabinfo.total_size);
                acc
            },
        );
        slab.insert(0, slab_total);

        let ksm = sample.ksm.as_ref().map(KsmModel::new);

        let mut disks: BTreeMap<String, SingleDiskModel> = BTreeMap::new();
        sample.disks.iter().for_each(|(disk_name, end_disk_stat)| {
            disks.insert(
                disk_name.clone(),
                match last {
                    Some((last_sample, duration)) if last_sample.disks.contains_key(disk_name) => {
                        SingleDiskModel::new(
                            last_sample.disks.get(disk_name).unwrap(),
                            end_disk_stat,
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

        let mut btrfs: Option<BTreeMap<String, BtrfsModel>> = None;
        match &sample.btrfs {
            Some(b) => {
                let tmp_btrfs: BTreeMap<String, BtrfsModel> = b
                    .iter()
                    .map(|(dir_name, end_dir_stat)| {
                        (dir_name.clone(), BtrfsModel::new(end_dir_stat))
                    })
                    .collect();
                btrfs = Some(tmp_btrfs);
            }
            None => {}
        }

        SystemModel {
            hostname: sample.hostname.clone(),
            kernel_version: sample.kernel_version.clone(),
            os_release: sample.os_release.clone(),
            stat,
            total_cpu,
            cpus,
            mem,
            vm,
            slab,
            ksm,
            disks,
            btrfs,
        }
    }
}

impl Nameable for SystemModel {
    fn name() -> &'static str {
        "system"
    }
}

#[::below_derive::queriable_derives]
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

#[::below_derive::queriable_derives]
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
                    irq_pct: Some(irq_usec as f64 * 100.0 / total_usec as f64),
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

#[::below_derive::queriable_derives]
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
            total: meminfo.total,
            free: meminfo.free,
            available: meminfo.available,
            buffers: meminfo.buffers,
            cached: meminfo.cached,
            swap_cached: meminfo.swap_cached,
            active: meminfo.active,
            inactive: meminfo.inactive,
            anon: opt_add(meminfo.active_anon, meminfo.inactive_anon),
            file: opt_add(meminfo.active_file, meminfo.inactive_file),
            unevictable: meminfo.unevictable,
            mlocked: meminfo.mlocked,
            swap_total: meminfo.swap_total,
            swap_free: meminfo.swap_free,
            dirty: meminfo.dirty,
            writeback: meminfo.writeback,
            anon_pages: meminfo.anon_pages,
            mapped: meminfo.mapped,
            shmem: meminfo.shmem,
            kreclaimable: meminfo.kreclaimable,
            slab: meminfo.slab,
            slab_reclaimable: meminfo.slab_reclaimable,
            slab_unreclaimable: meminfo.slab_unreclaimable,
            kernel_stack: meminfo.kernel_stack,
            page_tables: meminfo.page_tables,
            anon_huge_pages_bytes: meminfo.anon_huge_pages,
            shmem_huge_pages_bytes: meminfo.shmem_huge_pages,
            file_huge_pages_bytes: meminfo.file_huge_pages,
            hugetlb: meminfo.hugetlb,
            cma_total: meminfo.cma_total,
            cma_free: meminfo.cma_free,
            vmalloc_total: meminfo.vmalloc_total,
            vmalloc_used: meminfo.vmalloc_used,
            vmalloc_chunk: meminfo.vmalloc_chunk,
            direct_map_4k: meminfo.direct_map_4k,
            direct_map_2m: meminfo.direct_map_2m,
            direct_map_1g: meminfo.direct_map_1g,
        }
    }
}

#[::below_derive::queriable_derives]
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
            oom_kill: end.oom_kill,
        }
    }
}

#[::below_derive::queriable_derives]
pub struct SingleSlabModel {
    pub name: Option<String>,
    pub active_objs: Option<u64>,
    pub num_objs: Option<u64>,
    pub obj_size: Option<u64>,
    pub obj_per_slab: Option<u64>,
    pub num_slabs: Option<u64>,
    pub active_caches: Option<u64>,
    pub num_caches: Option<u64>,
    pub active_size: Option<u64>,
    pub total_size: Option<u64>,
}

impl SingleSlabModel {
    fn new(slabinfo: &procfs::SlabInfo) -> SingleSlabModel {
        SingleSlabModel {
            name: slabinfo.name.clone(),
            active_objs: slabinfo.active_objs,
            num_objs: slabinfo.num_objs,
            obj_size: slabinfo.obj_size,
            obj_per_slab: slabinfo.obj_per_slab,
            num_slabs: slabinfo.num_slabs,
            active_caches: slabinfo.active_objs.map(
                |active_objs| {
                    if active_objs > 0 { 1 } else { 0 }
                },
            ),
            num_caches: Some(1),
            active_size: opt_multiply(slabinfo.obj_size, slabinfo.active_objs),
            total_size: opt_multiply(slabinfo.obj_size, slabinfo.num_objs),
        }
    }
}

#[::below_derive::queriable_derives]
pub struct KsmModel {
    pub advisor_max_cpu: Option<u64>,
    pub advisor_max_pages_to_scan: Option<u64>,
    pub advisor_min_pages_to_scan: Option<u64>,
    pub advisor_mode: Option<String>,
    pub advisor_target_scan_time: Option<u64>,
    pub full_scans: Option<u64>,
    pub general_profit: Option<i64>,
    pub ksm_zero_pages: Option<i64>,
    pub max_page_sharing: Option<u64>,
    pub merge_across_nodes: Option<u64>,
    pub pages_scanned: Option<u64>,
    pub pages_shared: Option<u64>,
    pub pages_sharing: Option<u64>,
    pub pages_skipped: Option<u64>,
    pub pages_to_scan: Option<u64>,
    pub pages_unshared: Option<u64>,
    pub pages_volatile: Option<u64>,
    pub run: Option<u64>,
    pub sleep_millisecs: Option<u64>,
    pub smart_scan: Option<u64>,
    pub stable_node_chains: Option<u64>,
    pub stable_node_chains_prune_millisecs: Option<u64>,
    pub stable_node_dups: Option<u64>,
    pub use_zero_pages: Option<u64>,
}

impl KsmModel {
    fn new(ksm: &procfs::Ksm) -> KsmModel {
        KsmModel {
            advisor_max_cpu: ksm.advisor_max_cpu,
            advisor_max_pages_to_scan: ksm.advisor_max_pages_to_scan,
            advisor_min_pages_to_scan: ksm.advisor_min_pages_to_scan,
            advisor_mode: ksm.advisor_mode.clone(),
            advisor_target_scan_time: ksm.advisor_target_scan_time,
            full_scans: ksm.full_scans,
            general_profit: ksm.general_profit,
            ksm_zero_pages: ksm.ksm_zero_pages,
            max_page_sharing: ksm.max_page_sharing,
            merge_across_nodes: ksm.merge_across_nodes,
            pages_scanned: ksm.pages_scanned,
            pages_shared: ksm.pages_shared,
            pages_sharing: ksm.pages_sharing,
            pages_skipped: ksm.pages_skipped,
            pages_to_scan: ksm.pages_to_scan,
            pages_unshared: ksm.pages_unshared,
            pages_volatile: ksm.pages_volatile,
            run: ksm.run,
            sleep_millisecs: ksm.sleep_millisecs,
            smart_scan: ksm.smart_scan,
            stable_node_chains: ksm.stable_node_chains,
            stable_node_chains_prune_millisecs: ksm.stable_node_chains_prune_millisecs,
            stable_node_dups: ksm.stable_node_dups,
            use_zero_pages: ksm.use_zero_pages,
        }
    }
}

#[::below_derive::queriable_derives]
pub struct SingleDiskModel {
    pub name: Option<String>,
    pub disk_usage: Option<f32>,
    pub partition_size: Option<u64>,
    pub filesystem_type: Option<String>,
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
            disk_usage: end.disk_usage,
            partition_size: end.partition_size,
            filesystem_type: end.filesystem_type.clone(),
            read_bytes_per_sec,
            write_bytes_per_sec,
            discard_bytes_per_sec: count_per_sec!(
                begin.discard_sectors,
                end.discard_sectors,
                duration
            )
            .map(|val| val * 512.0),
            disk_total_bytes_per_sec: opt_add(read_bytes_per_sec, write_bytes_per_sec),
            read_completed: end.read_completed,
            read_merged: end.read_merged,
            read_sectors: end.read_sectors,
            time_spend_read_ms: end.time_spend_read_ms,
            write_completed: end.write_completed,
            write_merged: end.write_merged,
            write_sectors: end.write_sectors,
            time_spend_write_ms: end.time_spend_write_ms,
            discard_completed: end.discard_completed,
            discard_merged: end.discard_merged,
            discard_sectors: end.discard_sectors,
            time_spend_discard_ms: end.time_spend_discard_ms,
            major: end.major,
            minor: end.minor,
        }
    }
}

impl Nameable for SingleDiskModel {
    fn name() -> &'static str {
        "disk"
    }
}

#[::below_derive::queriable_derives]
pub struct BtrfsModel {
    pub name: Option<String>,
    pub disk_fraction: Option<f64>,
    pub disk_bytes: Option<u64>,
}

impl BtrfsModel {
    fn new(end: &btrfs::BtrfsStat) -> BtrfsModel {
        BtrfsModel {
            name: end.name.clone(),
            disk_fraction: end.disk_fraction,
            disk_bytes: end.disk_bytes,
        }
    }
}

impl Nameable for BtrfsModel {
    fn name() -> &'static str {
        "btrfs"
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn query_model() {
        let model_json = r#"
        {
            "hostname": "example.com",
            "stat": {},
            "total_cpu": {
                "idx": -1
            },
            "cpus": {},
            "mem": {},
            "vm": {},
            "slab": [],
            "ksm": {},
            "disks": {
                "sda": {
                    "name": "sda",
                    "read_bytes_per_sec": 42
                }
            },
            "btrfs": {}
        }
        "#;
        let model: SystemModel = serde_json::from_str(model_json).unwrap();
        assert_eq!(
            model.query(&SystemModelFieldId::from_str("disks.sda.read_bytes_per_sec").unwrap()),
            Some(Field::F64(42.0))
        );
    }
}
