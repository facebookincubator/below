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

/// Collection of all data local to the cgroup, e.g. its memory/io/cpu/pids usage.
/// Nothing about child cgroups or siblings, and therefore "Single" in its name.
#[::below_derive::queriable_derives]
pub struct SingleCgroupModel {
    pub name: String,
    pub full_path: String,
    pub inode_number: Option<u64>,
    #[queriable(ignore)]
    pub depth: u32,
    #[queriable(subquery)]
    #[queriable(preferred_name = props)]
    pub properties: Option<CgroupProperties>,
    #[queriable(subquery)]
    pub cpu: Option<CgroupCpuModel>,
    #[queriable(subquery)]
    #[queriable(preferred_name = mem)]
    pub memory: Option<CgroupMemoryModel>,
    #[queriable(subquery)]
    #[queriable(preferred_name = pids)]
    pub pids: Option<CgroupPidsModel>,
    #[queriable(subquery)]
    #[queriable(preferred_name = io_details)]
    pub io: Option<BTreeMap<String, CgroupIoModel>>,
    #[queriable(subquery)]
    #[queriable(preferred_name = io)]
    pub io_total: Option<CgroupIoModel>,
    #[queriable(subquery)]
    pub pressure: Option<CgroupPressureModel>,
    #[queriable(subquery)]
    pub cgroup_stat: Option<CgroupStatModel>,
    #[queriable(subquery)]
    #[queriable(preferred_name = mem_numa)]
    pub memory_numa_stat: Option<BTreeMap<u32, CgroupMemoryNumaModel>>,
}

/// A model that represents a cgroup subtree. Each instance is a node that uses
/// the "data" field to represent local data. Otherwise mixing hierarchy and
/// data makes it hard to define a Field Id type that queries nested cgroups.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CgroupModel {
    pub data: SingleCgroupModel,
    pub children: BTreeSet<CgroupModel>,
    /// Total number of cgroups under this subtree, including self
    pub count: u32,
    /// Indicate if such cgroup is created
    pub recreate_flag: bool,
}

/// Queries a specific SingleCgroupModel inside a CgroupModel tree.
/// Its String representation looks like this:
///     path:/system.slice/foo.service/.cpu.usage_pct
/// The path parameter starts with `path:` and ends with `/.`. This works
/// because SingleCgroupModelFieldId does not contain slash.
/// The path is used to drill into the Cgroup Model tree. If Vec empty, the
/// current CgroupModel is selected and queried with the subquery_id.
/// The path is optional in parsing and converting to String.
pub type CgroupModelFieldId = QueriableContainerFieldId<CgroupModel>;

#[derive(Clone, Debug, PartialEq)]
pub struct CgroupPath {
    pub path: Vec<String>,
}

impl std::fmt::Display for CgroupPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "path:/{}/", self.path.join("/"))
    }
}

impl FromStr for CgroupPath {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if !s.starts_with("path:/") {
            return Err(anyhow!("Path is not prefixed with `path:/`: {}", s));
        }
        Ok(Self {
            path: s["path:/".len()..]
                .split('/')
                .filter(|part| !part.is_empty())
                .map(|part| part.to_owned())
                .collect(),
        })
    }
}

impl QueriableContainer for CgroupModel {
    type Idx = CgroupPath;
    type SubqueryId = SingleCgroupModelFieldId;
    const IDX_PLACEHOLDER: &'static str = "[path:/<cgroup_path>/.]";
    fn split(s: &str) -> Option<(&str, &str)> {
        let idx_end = s.rfind("/.")?;
        Some((&s[..idx_end + 1], &s[idx_end + 2..]))
    }
    fn get_item(&self, idx: &Self::Idx) -> Option<&SingleCgroupModel> {
        let mut model = self;
        for part in idx.path.iter() {
            model = model.children.get(part.as_str())?;
        }
        Some(&model.data)
    }
}

impl core::borrow::Borrow<str> for CgroupModel {
    fn borrow(&self) -> &str {
        &self.data.name
    }
}

// We implement equality and ordering based on the cgroup name only so
// CgroupModel can be stored in a BTreeSet
impl Ord for CgroupModel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.name.cmp(&other.data.name)
    }
}

impl PartialOrd for CgroupModel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CgroupModel {
    fn eq(&self, other: &Self) -> bool {
        self.data.name == other.data.name
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
        let last_if_inode_matches =
            last.and_then(|(s, d)| match (s.inode_number, sample.inode_number) {
                (Some(prev_inode), Some(current_inode)) if prev_inode == current_inode => {
                    Some((s, d))
                }
                (None, None) => Some((s, d)),
                _ => None,
            });
        let properties = Some(CgroupProperties::new(sample));
        let (cpu, io, io_total, recreate_flag) = if let Some((last, delta)) = last_if_inode_matches
        {
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
                                    CgroupIoModel::new(begin_io_stat, end_io_stat, delta),
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

            (cpu, io, io_total, false)
        } else {
            // No cumulative data or inode number is different
            (None, None, None, last.is_some())
        };

        let memory = Some(CgroupMemoryModel::new(sample, last));

        let pids = Some(CgroupPidsModel::new(sample));

        let pressure = sample.pressure.as_ref().map(CgroupPressureModel::new);

        let cgroup_stat = sample.cgroup_stat.as_ref().map(CgroupStatModel::new);

        let memory_numa_stat = {
            sample.memory_numa_stat.as_ref().map(|end_numa_nodes| {
                let begin_numa_nodes = last_if_inode_matches.and_then(|(s, d)| {
                    s.memory_numa_stat
                        .as_ref()
                        .map(|numa_nodes| (numa_nodes, d))
                });
                end_numa_nodes
                    .iter()
                    .map(|(node_id, stat)| {
                        let begin_numa_stat = begin_numa_nodes
                            .and_then(|(nodes, d)| nodes.get(node_id).map(|stat| (stat, d)));
                        (*node_id, CgroupMemoryNumaModel::new(stat, begin_numa_stat))
                    })
                    .collect()
            })
        };

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
                    child_sample,
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
            data: SingleCgroupModel {
                name,
                full_path,
                inode_number: sample.inode_number.map(|ino| ino as u64),
                properties,
                cpu,
                memory,
                pids,
                io,
                io_total,
                pressure,
                depth,
                cgroup_stat,
                memory_numa_stat,
            },
            children,
            count: nr_descendants + 1,
            recreate_flag,
        }
    }

    pub fn aggr_top_level_val(mut self) -> Self {
        self.data.memory = self.children.iter().fold(Default::default(), |acc, model| {
            opt_add(acc, model.data.memory.clone())
        });
        self
    }
}

impl Nameable for CgroupModel {
    fn name() -> &'static str {
        "cgroup"
    }
}

impl Recursive for SingleCgroupModel {
    fn get_depth(&self) -> usize {
        self.depth as usize
    }
}

impl Nameable for SingleCgroupModel {
    fn name() -> &'static str {
        "cgroup"
    }
}

#[::below_derive::queriable_derives]
pub struct CgroupCpuModel {
    pub usage_pct: Option<f64>,
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
    pub nr_periods_per_sec: Option<f64>,
    pub nr_throttled_per_sec: Option<f64>,
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

#[::below_derive::queriable_derives]
pub struct CgroupStatModel {
    pub nr_descendants: Option<u32>,
    pub nr_dying_descendants: Option<u32>,
}

impl CgroupStatModel {
    pub fn new(cgroup_stat: &cgroupfs::CgroupStat) -> CgroupStatModel {
        CgroupStatModel {
            nr_descendants: cgroup_stat.nr_descendants,
            nr_dying_descendants: cgroup_stat.nr_dying_descendants,
        }
    }
}

#[::below_derive::queriable_derives]
pub struct CgroupIoModel {
    pub rbytes_per_sec: Option<f64>,
    pub wbytes_per_sec: Option<f64>,
    pub rios_per_sec: Option<f64>,
    pub wios_per_sec: Option<f64>,
    pub dbytes_per_sec: Option<f64>,
    pub dios_per_sec: Option<f64>,
    pub rwbytes_per_sec: Option<f64>,
    pub cost_usage_pct: Option<f64>,
    pub cost_wait_pct: Option<f64>,
    pub cost_indebt_pct: Option<f64>,
    pub cost_indelay_pct: Option<f64>,
}

impl CgroupIoModel {
    pub fn new(begin: &cgroupfs::IoStat, end: &cgroupfs::IoStat, delta: Duration) -> CgroupIoModel {
        let rbytes_per_sec = count_per_sec!(begin.rbytes, end.rbytes, delta);
        let wbytes_per_sec = count_per_sec!(begin.wbytes, end.wbytes, delta);
        let rwbytes_per_sec = opt_add(rbytes_per_sec, wbytes_per_sec);
        CgroupIoModel {
            rbytes_per_sec,
            wbytes_per_sec,
            rios_per_sec: count_per_sec!(begin.rios, end.rios, delta),
            wios_per_sec: count_per_sec!(begin.wios, end.wios, delta),
            dbytes_per_sec: count_per_sec!(begin.dbytes, end.dbytes, delta),
            dios_per_sec: count_per_sec!(begin.dios, end.dios, delta),
            rwbytes_per_sec,
            cost_usage_pct: usec_pct!(begin.cost_usage, end.cost_usage, delta),
            cost_wait_pct: usec_pct!(begin.cost_wait, end.cost_wait, delta),
            cost_indebt_pct: usec_pct!(begin.cost_indebt, end.cost_indebt, delta),
            cost_indelay_pct: usec_pct!(begin.cost_indelay, end.cost_indelay, delta),
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
            rwbytes_per_sec: Some(0.0),
            cost_usage_pct: Some(0.0),
            cost_wait_pct: Some(0.0),
            cost_indebt_pct: Some(0.0),
            cost_indelay_pct: Some(0.0),
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
            rwbytes_per_sec: opt_add(self.rwbytes_per_sec, other.rwbytes_per_sec),
            cost_usage_pct: opt_add(self.cost_usage_pct, other.cost_usage_pct),
            cost_wait_pct: opt_add(self.cost_wait_pct, other.cost_wait_pct),
            cost_indebt_pct: opt_add(self.cost_indebt_pct, other.cost_indebt_pct),
            cost_indelay_pct: opt_add(self.cost_indelay_pct, other.cost_indelay_pct),
        }
    }
}

#[::below_derive::queriable_derives]
pub struct CgroupMemoryModel {
    pub total: Option<u64>,
    pub swap: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub kernel: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub slab: Option<u64>,
    pub sock: Option<u64>,
    pub shmem: Option<u64>,
    pub zswap: Option<u64>,
    pub zswapped: Option<u64>,
    pub file_mapped: Option<u64>,
    pub file_dirty: Option<u64>,
    pub file_writeback: Option<u64>,
    pub anon_thp: Option<u64>,
    pub inactive_anon: Option<u64>,
    pub active_anon: Option<u64>,
    pub inactive_file: Option<u64>,
    pub active_file: Option<u64>,
    pub unevictable: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    pub pgfault: Option<u64>,
    pub pgmajfault: Option<u64>,
    pub workingset_refault_anon: Option<u64>,
    pub workingset_refault_file: Option<u64>,
    pub workingset_activate_anon: Option<u64>,
    pub workingset_activate_file: Option<u64>,
    pub workingset_restore_anon: Option<u64>,
    pub workingset_restore_file: Option<u64>,
    pub workingset_nodereclaim: Option<u64>,
    pub pgrefill: Option<u64>,
    pub pgscan: Option<u64>,
    pub pgsteal: Option<u64>,
    pub pgactivate: Option<u64>,
    pub pgdeactivate: Option<u64>,
    pub pglazyfree: Option<u64>,
    pub pglazyfreed: Option<u64>,
    pub thp_fault_alloc: Option<u64>,
    pub thp_collapse_alloc: Option<u64>,
    pub events_low: Option<u64>,
    pub events_high: Option<u64>,
    pub events_max: Option<u64>,
    pub events_oom: Option<u64>,
    pub events_oom_kill: Option<u64>,
    pub events_local_low: Option<u64>,
    pub events_local_high: Option<u64>,
    pub events_local_max: Option<u64>,
    pub events_local_oom: Option<u64>,
    pub events_local_oom_kill: Option<u64>,
}

impl std::ops::Add for CgroupMemoryModel {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            total: opt_add(self.total, other.total),
            swap: opt_add(self.swap, other.swap),
            anon: opt_add(self.anon, other.anon),
            file: opt_add(self.file, other.file),
            kernel: opt_add(self.kernel, other.kernel),
            kernel_stack: opt_add(self.kernel_stack, other.kernel_stack),
            slab: opt_add(self.slab, other.slab),
            sock: opt_add(self.sock, other.sock),
            shmem: opt_add(self.shmem, other.shmem),
            zswap: opt_add(self.zswap, other.zswap),
            zswapped: opt_add(self.zswapped, other.zswapped),
            file_mapped: opt_add(self.file_mapped, other.file_mapped),
            file_dirty: opt_add(self.file_dirty, other.file_dirty),
            file_writeback: opt_add(self.file_writeback, other.file_writeback),
            anon_thp: opt_add(self.anon_thp, other.anon_thp),
            inactive_anon: opt_add(self.inactive_anon, other.inactive_anon),
            active_anon: opt_add(self.active_anon, other.active_anon),
            inactive_file: opt_add(self.inactive_file, other.inactive_file),
            active_file: opt_add(self.active_file, other.active_file),
            unevictable: opt_add(self.unevictable, other.unevictable),
            slab_reclaimable: opt_add(self.slab_reclaimable, other.slab_reclaimable),
            slab_unreclaimable: opt_add(self.slab_unreclaimable, other.slab_unreclaimable),
            pgfault: opt_add(self.pgfault, other.pgfault),
            pgmajfault: opt_add(self.pgmajfault, other.pgmajfault),
            workingset_refault_anon: opt_add(
                self.workingset_refault_anon,
                other.workingset_refault_anon,
            ),
            workingset_refault_file: opt_add(
                self.workingset_refault_file,
                other.workingset_refault_file,
            ),
            workingset_activate_anon: opt_add(
                self.workingset_activate_anon,
                other.workingset_activate_anon,
            ),
            workingset_activate_file: opt_add(
                self.workingset_activate_file,
                other.workingset_activate_file,
            ),
            workingset_restore_anon: opt_add(
                self.workingset_restore_anon,
                other.workingset_restore_anon,
            ),
            workingset_restore_file: opt_add(
                self.workingset_restore_file,
                other.workingset_restore_file,
            ),
            workingset_nodereclaim: opt_add(
                self.workingset_nodereclaim,
                other.workingset_nodereclaim,
            ),
            pgrefill: opt_add(self.pgrefill, other.pgrefill),
            pgscan: opt_add(self.pgscan, other.pgscan),
            pgsteal: opt_add(self.pgsteal, other.pgsteal),
            pgactivate: opt_add(self.pgactivate, other.pgactivate),
            pgdeactivate: opt_add(self.pgdeactivate, other.pgdeactivate),
            pglazyfree: opt_add(self.pglazyfree, other.pglazyfree),
            pglazyfreed: opt_add(self.pglazyfreed, other.pglazyfreed),
            thp_fault_alloc: opt_add(self.thp_fault_alloc, other.thp_fault_alloc),
            thp_collapse_alloc: opt_add(self.thp_collapse_alloc, other.thp_collapse_alloc),
            events_low: opt_add(self.events_low, other.events_low),
            events_high: opt_add(self.events_high, other.events_high),
            events_max: opt_add(self.events_max, other.events_max),
            events_oom: opt_add(self.events_oom, other.events_oom),
            events_oom_kill: opt_add(self.events_oom_kill, other.events_oom_kill),
            events_local_low: opt_add(self.events_local_low, other.events_local_low),
            events_local_high: opt_add(self.events_local_high, other.events_local_high),
            events_local_max: opt_add(self.events_local_max, other.events_local_max),
            events_local_oom: opt_add(self.events_local_oom, other.events_local_oom),
            events_local_oom_kill: opt_add(self.events_local_oom_kill, other.events_local_oom_kill),
        }
    }
}

impl CgroupMemoryModel {
    pub fn new(
        sample: &CgroupSample,
        last: Option<(&CgroupSample, Duration)>,
    ) -> CgroupMemoryModel {
        let mut model = CgroupMemoryModel {
            total: sample.memory_current.map(|v| v as u64),
            swap: sample.memory_swap_current.map(|v| v as u64),
            zswap: sample.memory_zswap_current.map(|v| v as u64),
            ..Default::default()
        };
        if let Some(events) = &sample.memory_events {
            model.events_low = events.low;
            model.events_high = events.high;
            model.events_max = events.max;
            model.events_oom = events.oom;
            model.events_oom_kill = events.oom_kill;
        }
        if let Some(events_local) = &sample.memory_events_local {
            model.events_local_low = events_local.low;
            model.events_local_high = events_local.high;
            model.events_local_max = events_local.max;
            model.events_local_oom = events_local.oom;
            model.events_local_oom_kill = events_local.oom_kill;
        }
        if let Some(stat) = &sample.memory_stat {
            model.anon = stat.anon;
            model.file = stat.file;
            model.kernel = stat.kernel;
            model.kernel_stack = stat.kernel_stack;
            model.slab = stat.slab;
            model.sock = stat.sock;
            model.shmem = stat.shmem;
            // May be set by sample.memory_zswap_current
            if model.zswap.is_none() {
                model.zswap = stat.zswap;
            }
            model.zswapped = stat.zswapped;
            model.file_mapped = stat.file_mapped;
            model.file_dirty = stat.file_dirty;
            model.file_writeback = stat.file_writeback;
            model.anon_thp = stat.anon_thp;
            model.inactive_anon = stat.inactive_anon;
            model.active_anon = stat.active_anon;
            model.inactive_file = stat.inactive_file;
            model.active_file = stat.active_file;
            model.unevictable = stat.unevictable;
            model.slab_reclaimable = stat.slab_reclaimable;
            model.slab_unreclaimable = stat.slab_unreclaimable;

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
                model.workingset_refault_anon = count_per_sec!(
                    last_stat.workingset_refault_anon,
                    stat.workingset_refault_anon,
                    delta,
                    u64
                );
                model.workingset_refault_file = count_per_sec!(
                    last_stat.workingset_refault_file,
                    stat.workingset_refault_file,
                    delta,
                    u64
                );
                model.workingset_activate_anon = count_per_sec!(
                    last_stat.workingset_activate_anon,
                    stat.workingset_activate_anon,
                    delta,
                    u64
                );
                model.workingset_activate_file = count_per_sec!(
                    last_stat.workingset_activate_file,
                    stat.workingset_activate_file,
                    delta,
                    u64
                );
                model.workingset_restore_anon = count_per_sec!(
                    last_stat.workingset_restore_anon,
                    stat.workingset_restore_anon,
                    delta,
                    u64
                );
                model.workingset_restore_file = count_per_sec!(
                    last_stat.workingset_restore_file,
                    stat.workingset_restore_file,
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

#[::below_derive::queriable_derives]
pub struct CgroupPidsModel {
    pub tids_current: Option<u64>,
}

impl std::ops::Add for CgroupPidsModel {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            tids_current: opt_add(self.tids_current, other.tids_current),
        }
    }
}

impl CgroupPidsModel {
    pub fn new(sample: &CgroupSample) -> Self {
        let tids_current = sample.tids_current;
        CgroupPidsModel { tids_current }
    }
}

#[::below_derive::queriable_derives]
pub struct CgroupPressureModel {
    pub cpu_some_pct: Option<f64>,
    pub cpu_full_pct: Option<f64>,
    pub io_some_pct: Option<f64>,
    pub io_full_pct: Option<f64>,
    pub memory_some_pct: Option<f64>,
    pub memory_full_pct: Option<f64>,
}

impl CgroupPressureModel {
    fn new(pressure: &cgroupfs::Pressure) -> CgroupPressureModel {
        // Use avg10 instead of calculating pressure with the total metric. If
        // elapsed time between reading pressure total and recording time is too
        // long, pressure could exceed 100%.
        CgroupPressureModel {
            cpu_some_pct: pressure.cpu.some.avg10,
            cpu_full_pct: pressure.cpu.full.as_ref().and_then(|f| f.avg10),
            io_some_pct: pressure.io.some.avg10,
            io_full_pct: pressure.io.full.avg10,
            memory_some_pct: pressure.memory.some.avg10,
            memory_full_pct: pressure.memory.full.avg10,
        }
    }
}
#[::below_derive::queriable_derives]
pub struct CgroupMemoryNumaModel {
    pub total: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub pagetables: Option<u64>,
    pub shmem: Option<u64>,
    pub file_mapped: Option<u64>,
    pub file_dirty: Option<u64>,
    pub file_writeback: Option<u64>,
    pub swapcached: Option<u64>,
    pub anon_thp: Option<u64>,
    pub file_thp: Option<u64>,
    pub shmem_thp: Option<u64>,
    pub inactive_anon: Option<u64>,
    pub active_anon: Option<u64>,
    pub inactive_file: Option<u64>,
    pub active_file: Option<u64>,
    pub unevictable: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    pub workingset_refault_anon: Option<f64>,
    pub workingset_refault_file: Option<f64>,
    pub workingset_activate_anon: Option<f64>,
    pub workingset_activate_file: Option<f64>,
    pub workingset_restore_anon: Option<f64>,
    pub workingset_restore_file: Option<f64>,
    pub workingset_nodereclaim: Option<f64>,
}

impl CgroupMemoryNumaModel {
    pub fn new(
        begin: &cgroupfs::MemoryNumaStat,
        last: Option<(&cgroupfs::MemoryNumaStat, Duration)>,
    ) -> CgroupMemoryNumaModel {
        let mut model = CgroupMemoryNumaModel {
            total: None,
            anon: begin.anon,
            file: begin.file,
            kernel_stack: begin.kernel_stack,
            pagetables: begin.pagetables,
            shmem: begin.shmem,
            file_mapped: begin.file_mapped,
            file_dirty: begin.file_dirty,
            file_writeback: begin.file_writeback,
            swapcached: begin.swapcached,
            anon_thp: begin.anon_thp,
            file_thp: begin.file_thp,
            shmem_thp: begin.shmem_thp,
            inactive_anon: begin.inactive_anon,
            active_anon: begin.active_anon,
            inactive_file: begin.inactive_file,
            active_file: begin.active_file,
            unevictable: begin.unevictable,
            slab_reclaimable: begin.slab_reclaimable,
            slab_unreclaimable: begin.slab_unreclaimable,
            ..Default::default()
        };
        if let (Some(anon), Some(file), Some(kernel_stack), Some(pagetables)) =
            (model.anon, model.file, model.kernel_stack, model.pagetables)
        {
            model.total = Some(
                anon.saturating_add(file)
                    .saturating_add(kernel_stack)
                    .saturating_add(pagetables),
            );
        }

        if let Some((l, delta)) = last {
            model.workingset_refault_anon = count_per_sec!(
                begin.workingset_refault_anon,
                l.workingset_refault_anon,
                delta
            );
            model.workingset_refault_file = count_per_sec!(
                begin.workingset_refault_file,
                l.workingset_refault_file,
                delta
            );
            model.workingset_activate_anon = count_per_sec!(
                begin.workingset_activate_anon,
                l.workingset_activate_anon,
                delta
            );
            model.workingset_activate_file = count_per_sec!(
                begin.workingset_activate_file,
                l.workingset_activate_file,
                delta
            );
            model.workingset_restore_anon = count_per_sec!(
                begin.workingset_restore_anon,
                l.workingset_restore_anon,
                delta
            );
            model.workingset_restore_file = count_per_sec!(
                begin.workingset_restore_file,
                l.workingset_restore_file,
                delta
            );
            model.workingset_nodereclaim = count_per_sec!(
                begin.workingset_nodereclaim,
                l.workingset_nodereclaim,
                delta
            );
        }
        model
    }
}

/// Cgroup properties. Without any cgroup configuration changes, these should
/// typically be static.
#[::below_derive::queriable_derives]
pub struct CgroupProperties {
    pub cgroup_controllers: Option<BTreeSet<String>>,
    pub cgroup_subtree_control: Option<BTreeSet<String>>,
    pub tids_max: Option<i64>,
    pub memory_min: Option<i64>,
    pub memory_low: Option<i64>,
    pub memory_high: Option<i64>,
    pub memory_max: Option<i64>,
    pub memory_swap_max: Option<i64>,
    pub memory_zswap_max: Option<i64>,
    pub cpu_weight: Option<u32>,
    pub cpu_max_usec: Option<i64>,
    pub cpu_max_period_usec: Option<u64>,
    pub cpuset_cpus: Option<cgroupfs::Cpuset>,
    pub cpuset_cpus_effective: Option<cgroupfs::Cpuset>,
    pub cpuset_mems: Option<cgroupfs::MemNodes>,
    pub cpuset_mems_effective: Option<cgroupfs::MemNodes>,
}

impl CgroupProperties {
    pub fn new(sample: &CgroupSample) -> Self {
        Self {
            cgroup_controllers: sample.cgroup_controllers.clone(),
            cgroup_subtree_control: sample.cgroup_subtree_control.clone(),
            tids_max: sample.tids_max,
            memory_min: sample.memory_min,
            memory_low: sample.memory_low,
            memory_high: sample.memory_high,
            memory_max: sample.memory_max,
            memory_swap_max: sample.memory_swap_max,
            memory_zswap_max: sample.memory_zswap_max,
            cpu_weight: sample.cpu_weight,
            cpu_max_usec: sample.cpu_max.as_ref().map(|v| v.max_usec),
            cpu_max_period_usec: sample.cpu_max.as_ref().map(|v| v.period_usec),
            cpuset_cpus: sample.cpuset_cpus.clone(),
            cpuset_cpus_effective: sample.cpuset_cpus_effective.clone(),
            cpuset_mems: sample.cpuset_mems.clone(),
            cpuset_mems_effective: sample.cpuset_mems_effective.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn query_nested_cgroup() {
        let model_json = r#"
        {
            "data": { "name": "<root>", "full_path": "", "depth": 0 },
            "count": 4,
            "recreate_flag": false,
            "children": [
                {
                    "data": { "name": "system.slice", "full_path": "/system.slice", "depth": 1 },
                    "count": 2,
                    "recreate_flag": false,
                    "children": [
                        {
                            "data": { "name": "foo.service", "full_path": "/system.slice/foo.service", "depth": 2 },
                            "count": 1,
                            "recreate_flag": false,
                            "children": []
                        }
                    ]
                },
                {
                    "data": { "name": ".hidden.slice", "full_path": "/.hidden.slice", "depth": 1 },
                    "count": 1,
                    "recreate_flag": false,
                    "children": []
                }
            ]
        }
        "#;
        let model: CgroupModel =
            serde_json::from_str(model_json).expect("Failed to deserialize cgroup model JSON");
        for (field_id, expected) in &[
            // Ignore consecutive slashes
            ("path:///////.name", Some("<root>")),
            ("path:/system.slice/.full_path", Some("/system.slice")),
            (
                "path:/system.slice/foo.service/.full_path",
                Some("/system.slice/foo.service"),
            ),
            // Allow path param to contain "/."
            ("path:/.hidden.slice/.full_path", Some("/.hidden.slice")),
            // Non-existent cgroups
            ("path:/no_such.slice/.full_path", None),
            ("path:/system.slice/no_such.service/.full_path", None),
        ] {
            assert_eq!(
                model.query(
                    &CgroupModelFieldId::from_str(field_id)
                        .map_err(|e| format!("Failed to parse field id {}: {:?}", field_id, e))
                        .unwrap()
                ),
                expected.map(|s| Field::Str(s.to_string()))
            );
        }
    }

    #[test]
    fn query_model() {
        let model_json = r#"
        {
            "name": "foo.service",
            "full_path": "/system.slice/foo.service",
            "depth": 1,
            "io": {
                "sda": {
                    "rbytes_per_sec": 42
                }
            }
        }
        "#;
        let model: SingleCgroupModel = serde_json::from_str(model_json).unwrap();
        assert_eq!(
            model.query(
                &SingleCgroupModelFieldId::from_str("io_details.sda.rbytes_per_sec").unwrap()
            ),
            Some(Field::F64(42.0))
        );
    }
}
