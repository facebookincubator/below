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

//! Decide what the BPF program may read on this host, and tell it.
//!
//! Three steps, one per section below: check the host, decide from that, write
//! it to the program's maps. Every decision rests on a check, so a field left to
//! the file has a known cause.
//!
//! Three things reach the program: skip bits (a stat file BPF must not supply),
//! feature bits (an extra read to turn on) and item indices (where to find a
//! counter the program cannot name).

use std::ffi::OsStr;

use libbpf_rs::Btf;
use libbpf_rs::MapCore as _;
use libbpf_rs::MapFlags;
use libbpf_rs::MapMut;
use libbpf_rs::btf::types::Enum;
use libbpf_rs::btf::types::Func;

use super::CGROUP_BPF_FEATURE_HUGETLB;
use super::CGROUP_BPF_FEATURE_ZSWAP;
use super::CGROUP_BPF_ITEM_ABSENT;
use super::CGROUP_BPF_ITEM_NR_HUGETLB;
use super::CGROUP_BPF_ITEM_NUM;
use super::CGROUP_BPF_ITEM_PAGE_STATE;
use super::CGROUP_BPF_ITEM_PGREFILL;
use super::CGROUP_BPF_ITEM_PGSCAN_DIRECT;
use super::CGROUP_BPF_ITEM_PGSCAN_KHUGEPAGED;
use super::CGROUP_BPF_ITEM_PGSCAN_KSWAPD;
use super::CGROUP_BPF_ITEM_PGSCAN_PROACTIVE;
use super::CGROUP_BPF_ITEM_PGSTEAL_DIRECT;
use super::CGROUP_BPF_ITEM_PGSTEAL_KHUGEPAGED;
use super::CGROUP_BPF_ITEM_PGSTEAL_KSWAPD;
use super::CGROUP_BPF_ITEM_PGSTEAL_PROACTIVE;
use super::CGROUP_BPF_ITEM_VM_EVENT;
use super::CGROUP_FILES_MEM_EVENTS;
use super::CGROUP_FILES_MEM_STAT;
use super::cgroup_bpf_item_index;

// check the config in host
const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

/// What this host offers the BPF program. Facts only; the rules are below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct KernelSupport {
    /// False when the kernel's BTF could not be read, so nothing else was
    /// checked either.
    btf_readable: bool,
    /// The kernel exports every memcg read kfunc, which read all of memory.stat.
    memcg_kfuncs: bool,
    /// The kernel has memory.events' newer `sock_throttled` counter.
    sock_throttled: bool,
    /// The kernel was built with CONFIG_ZSWAP, so memory.stat has the zswap
    /// lines.
    zswap: bool,
    /// The cgroup2 mount accounts hugetlb, so memory.stat has that line. None
    /// when the mount table could not be read -- not the same as false.
    hugetlb_accounting: Option<bool>,
    /// Per `enum cgroup_bpf_item` slot: the kfunc to read that counter with, and
    /// its index. `ABSENT` when this kernel has neither.
    items: [(u32, i32); CGROUP_BPF_ITEM_NUM as usize],
}

/// The counters the program reads by index, with the enumerator to look up. The
/// slot comes from the shared header, so only the spelling lives here.
const ITEMS_READ_BY_INDEX: [(u32, &str); CGROUP_BPF_ITEM_NUM as usize] = [
    (CGROUP_BPF_ITEM_NR_HUGETLB, "NR_HUGETLB"),
    (CGROUP_BPF_ITEM_PGREFILL, "PGREFILL"),
    (CGROUP_BPF_ITEM_PGSCAN_KSWAPD, "PGSCAN_KSWAPD"),
    (CGROUP_BPF_ITEM_PGSCAN_DIRECT, "PGSCAN_DIRECT"),
    (CGROUP_BPF_ITEM_PGSCAN_KHUGEPAGED, "PGSCAN_KHUGEPAGED"),
    (CGROUP_BPF_ITEM_PGSCAN_PROACTIVE, "PGSCAN_PROACTIVE"),
    (CGROUP_BPF_ITEM_PGSTEAL_KSWAPD, "PGSTEAL_KSWAPD"),
    (CGROUP_BPF_ITEM_PGSTEAL_DIRECT, "PGSTEAL_DIRECT"),
    (CGROUP_BPF_ITEM_PGSTEAL_KHUGEPAGED, "PGSTEAL_KHUGEPAGED"),
    (CGROUP_BPF_ITEM_PGSTEAL_PROACTIVE, "PGSTEAL_PROACTIVE"),
];

/// Check the running kernel and the cgroup2 mount. Called once at startup; only
/// the mount option could change, and only on a remount.
fn probe_host() -> KernelSupport {
    let mut support = KernelSupport {
        hugetlb_accounting: read_hugetlb_accounting(),
        ..Default::default()
    };
    let Ok(btf) = Btf::from_vmlinux() else {
        return support;
    };
    support.btf_readable = true;
    support.memcg_kfuncs = has_memcg_kfuncs(&btf);
    support.sock_throttled = btf_enum_has_item(&btf, "memcg_memory_event", "MEMCG_SOCK_THROTTLED");
    // CONFIG_ZSWAP drops the ZSWPIN event, so its absence stands for the option.
    support.zswap = btf_enum_has_item(&btf, "vm_event_item", "ZSWPIN");
    for (slot, name) in ITEMS_READ_BY_INDEX {
        support.items[slot as usize] = find_item(&btf, name);
    }
    support
}

// The BTF lookups: the same question in three shapes, and the only place that
// knows how a counter is spelled.

/// Find one counter in whichever enum this kernel keeps it in. Linux 7.1 moved
/// the reclaim counters from `vm_event_item` to `node_stat_item`.
fn find_item(btf: &Btf<'_>, name: &str) -> (u32, i32) {
    if let Some(index) = btf_enum_item_value(btf, "node_stat_item", name) {
        (CGROUP_BPF_ITEM_PAGE_STATE, index as i32)
    } else if let Some(index) = btf_enum_item_value(btf, "vm_event_item", name) {
        (CGROUP_BPF_ITEM_VM_EVENT, index as i32)
    } else {
        (CGROUP_BPF_ITEM_ABSENT, 0)
    }
}

/// True when the kernel exports every memcg read kfunc.
fn has_memcg_kfuncs(btf: &Btf<'_>) -> bool {
    [
        "bpf_get_mem_cgroup",
        "bpf_put_mem_cgroup",
        "bpf_mem_cgroup_page_state",
        "bpf_mem_cgroup_vm_events",
    ]
    .iter()
    .all(|name| btf.type_by_name::<Func<'_>>(name).is_some())
}

/// Find an enumerator's value: both whether this kernel tracks that counter and
/// the index to read it by. Unlike the program's `bpf_core_enum_value_exists`,
/// this also reaches names the vendored header cannot spell.
fn btf_enum_item_value(btf: &Btf<'_>, enum_name: &str, item: &str) -> Option<i64> {
    let e = btf.type_by_name::<Enum<'_>>(enum_name)?;
    e.iter()
        .find(|m| m.name == Some(OsStr::new(item)))
        .map(|m| m.value)
}

/// True when the named enum has this enumerator.
fn btf_enum_has_item(btf: &Btf<'_>, enum_name: &str, item: &str) -> bool {
    btf_enum_item_value(btf, enum_name, item).is_some()
}

/// Read whether the cgroup2 mount accounts hugetlb, or None when the mount table
/// cannot be read. The one fact that does not come from BTF.
fn read_hugetlb_accounting() -> Option<bool> {
    std::fs::read_to_string(MOUNTINFO_PATH)
        .ok()
        .map(|mountinfo| mountinfo_has_hugetlb_accounting(&mountinfo))
}

/// True when the cgroup2 hierarchy accounts hugetlb to memcg. There is one such
/// hierarchy, so any cgroup2 line answers for all of it.
fn mountinfo_has_hugetlb_accounting(mountinfo: &str) -> bool {
    mountinfo.lines().any(|line| {
        // "<mount fields> - <fstype> <source> <super options>".
        let Some((_, fs)) = line.split_once(" - ") else {
            return false;
        };
        let mut fields = fs.split_whitespace();
        if fields.next() != Some("cgroup2") {
            return false;
        }
        let Some(options) = fields.nth(1) else {
            return false;
        };
        options
            .split(',')
            .any(|opt| opt == "memory_hugetlb_accounting")
    })
}

/// What the BPF program should read here: the stat files to leave alone, the
/// extra reads to turn on, and where to find the counters it cannot name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BpfReadConfig {
    pub(crate) skip_files: u32,
    pub(crate) extra_reads: u32,
    pub(crate) items: [(u32, i32); CGROUP_BPF_ITEM_NUM as usize],
}

impl BpfReadConfig {
    /// Check this host and decide from it what BPF should read.
    pub(crate) fn detect() -> Self {
        Self::from_support(&probe_host())
    }

    /// The decision rules, over what the host was found to support.
    fn from_support(support: &KernelSupport) -> Self {
        let mut config = Self {
            items: support.items,
            ..Default::default()
        };

        // Nothing was checked, so leave both files that depend on a check to the
        // file reader. The program needs BTF to load, so this should not happen.
        if !support.btf_readable {
            config.skip_files |= (1 << CGROUP_FILES_MEM_STAT) | (1 << CGROUP_FILES_MEM_EVENTS);
            return config;
        }

        // below reads the reclaim aggregates, not the components, and the kernel
        // sums whichever components it has -- so one match per aggregate is
        // enough. pgrefill is read directly, so it must be found on its own.
        let found = |slot: u32| config.items[slot as usize].0 != CGROUP_BPF_ITEM_ABSENT;
        let any_of = |slots: [u32; 4]| slots.into_iter().any(found);
        let reclaim_readable = found(CGROUP_BPF_ITEM_PGREFILL)
            && any_of([
                CGROUP_BPF_ITEM_PGSCAN_KSWAPD,
                CGROUP_BPF_ITEM_PGSCAN_DIRECT,
                CGROUP_BPF_ITEM_PGSCAN_KHUGEPAGED,
                CGROUP_BPF_ITEM_PGSCAN_PROACTIVE,
            ])
            && any_of([
                CGROUP_BPF_ITEM_PGSTEAL_KSWAPD,
                CGROUP_BPF_ITEM_PGSTEAL_DIRECT,
                CGROUP_BPF_ITEM_PGSTEAL_KHUGEPAGED,
                CGROUP_BPF_ITEM_PGSTEAL_PROACTIVE,
            ]);
        if !support.memcg_kfuncs || !reclaim_readable {
            config.skip_files |= 1 << CGROUP_FILES_MEM_STAT;
        }

        // The program does not read `sock_throttled`, so where the kernel has it
        // the whole file goes to the file reader.
        if support.sock_throttled {
            config.skip_files |= 1 << CGROUP_FILES_MEM_EVENTS;
        }

        // zswap and hugetlb are printed only under an option, though the kernel
        // tracks them either way, so read them only when the file has them.
        if support.zswap {
            config.extra_reads |= 1 << CGROUP_BPF_FEATURE_ZSWAP;
        }
        // Not knowing means memory.stat goes to the file, rather than risk
        // losing the line.
        match support.hugetlb_accounting {
            Some(false) => {}
            Some(true) if found(CGROUP_BPF_ITEM_NR_HUGETLB) => {
                config.extra_reads |= 1 << CGROUP_BPF_FEATURE_HUGETLB;
            }
            _ => config.skip_files |= 1 << CGROUP_FILES_MEM_STAT,
        }

        config
    }

    /// Write the masks and the item indices, before the first traversal.
    pub(crate) fn write_to(
        &self,
        skip_groups: &MapMut<'_>,
        features: &MapMut<'_>,
        item_index: &MapMut<'_>,
    ) {
        write_entry(skip_groups, 0, &self.skip_files.to_ne_bytes());
        write_entry(features, 0, &self.extra_reads.to_ne_bytes());
        for (slot, (source, index)) in self.items.iter().enumerate() {
            // struct cgroup_bpf_item_index, packed by hand so the layout the
            // program reads is stated here; the assertion catches it changing.
            const _: () = assert!(std::mem::size_of::<cgroup_bpf_item_index>() == 8);
            let mut value = [0u8; 8];
            value[..4].copy_from_slice(&source.to_ne_bytes());
            value[4..].copy_from_slice(&index.to_ne_bytes());
            write_entry(item_index, slot as u32, &value);
        }
    }
}

/// Put one value in a map, dropping the error: a write can only fail if the
/// program is not loaded, and then there is nothing to configure.
fn write_entry(map: &MapMut<'_>, key: u32, value: &[u8]) {
    let _ = map.update(&key.to_ne_bytes(), value, MapFlags::ANY);
}
