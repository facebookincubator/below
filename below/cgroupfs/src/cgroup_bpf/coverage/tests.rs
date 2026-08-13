// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! What the startup probe decides: which stat files BPF may supply on a given
//! kernel, and which it has to leave to the file reader.

use super::*;

/// A host where the kernel offers everything the program wants. Each test
/// removes one capability, so the effect of that capability alone is what
/// gets asserted.
fn capable_host() -> KernelSupport {
    let mut support = KernelSupport {
        btf_readable: true,
        memcg_kfuncs: true,
        flush_kfunc: true,
        sock_throttled: false,
        zswap: true,
        hugetlb_accounting: Some(false),
        items: [(CGROUP_BPF_ITEM_ABSENT, 0); CGROUP_BPF_ITEM_NUM as usize],
    };
    for (slot, _) in ITEMS_READ_BY_INDEX {
        support.items[slot as usize] = (CGROUP_BPF_ITEM_VM_EVENT, 1);
    }
    support
}

fn skips_memory_stat(config: &BpfReadConfig) -> bool {
    config.skip_files & (1 << CGROUP_FILES_MEM_STAT) != 0
}

fn skips_memory_events(config: &BpfReadConfig) -> bool {
    config.skip_files & (1 << CGROUP_FILES_MEM_EVENTS) != 0
}

fn reads(config: &BpfReadConfig, feature: u32) -> bool {
    config.extra_reads & (1 << feature) != 0
}

/// Exercises `BpfReadConfig::from_support` deciding which stat files BPF may
/// supply. Each capability the kernel lacks sends exactly the files that depend
/// on it to the file reader, and leaves the rest to BPF.
#[test]
fn capabilities_decide_which_files_bpf_supplies() {
    // Everything available: BPF supplies both files.
    let all = BpfReadConfig::from_support(&capable_host());
    assert!(!skips_memory_stat(&all));
    assert!(!skips_memory_events(&all));

    // No kernel BTF: nothing could be checked, so neither file is trusted
    // to BPF.
    let config = BpfReadConfig::from_support(&KernelSupport {
        btf_readable: false,
        ..capable_host()
    });
    assert!(skips_memory_stat(&config) && skips_memory_events(&config));

    // memory.stat comes from the memcg kfuncs; without them the whole file
    // is read instead.
    let config = BpfReadConfig::from_support(&KernelSupport {
        memcg_kfuncs: false,
        ..capable_host()
    });
    assert!(skips_memory_stat(&config));
    assert!(
        !skips_memory_events(&config),
        "only memory.stat depends on them"
    );

    // The program does not read sock_throttled, so a kernel that has it
    // would leave the line missing -- read the file instead.
    let config = BpfReadConfig::from_support(&KernelSupport {
        sock_throttled: true,
        ..capable_host()
    });
    assert!(skips_memory_events(&config));
    assert!(!skips_memory_stat(&config));

    // zswap and hugetlb lines exist only under an option; read them only
    // where the file prints them.
    assert!(reads(&all, CGROUP_BPF_FEATURE_ZSWAP));
    let config = BpfReadConfig::from_support(&KernelSupport {
        zswap: false,
        ..capable_host()
    });
    assert!(!reads(&config, CGROUP_BPF_FEATURE_ZSWAP));
    assert!(
        !skips_memory_stat(&config),
        "zswap is per-field, not per-file"
    );
}

/// Exercises `from_support`'s hugetlb arm, which has three answers rather than
/// two. An unreadable mount table is not the same as accounting being off, and
/// reading it as off would drop the line on a host that prints it.
#[test]
fn unknown_hugetlb_accounting_is_not_the_same_as_off() {
    // Off: the file has no hugetlb line, so there is nothing to source.
    let config = BpfReadConfig::from_support(&KernelSupport {
        hugetlb_accounting: Some(false),
        ..capable_host()
    });
    assert!(!skips_memory_stat(&config));
    assert!(!reads(&config, CGROUP_BPF_FEATURE_HUGETLB));

    // On, and the counter can be found: read it.
    let config = BpfReadConfig::from_support(&KernelSupport {
        hugetlb_accounting: Some(true),
        ..capable_host()
    });
    assert!(reads(&config, CGROUP_BPF_FEATURE_HUGETLB));
    assert!(!skips_memory_stat(&config));

    // Unknown: fall back to the file rather than risk losing the line.
    let config = BpfReadConfig::from_support(&KernelSupport {
        hugetlb_accounting: None,
        ..capable_host()
    });
    assert!(skips_memory_stat(&config));

    // On, but this kernel has no such counter to read: also the file.
    let mut support = KernelSupport {
        hugetlb_accounting: Some(true),
        ..capable_host()
    };
    support.items[CGROUP_BPF_ITEM_NR_HUGETLB as usize] = (CGROUP_BPF_ITEM_ABSENT, 0);
    let config = BpfReadConfig::from_support(&support);
    assert!(skips_memory_stat(&config));
}

/// Exercises `from_support`'s rule for the reclaim counters. below reads the
/// aggregates and the kernel sums whichever components it has, so one component
/// per aggregate is enough -- but pgrefill is read on its own and has to be
/// found on its own.
#[test]
fn reclaim_counters_decide_memory_stat() {
    let with = |slots: &[u32]| {
        let mut support = capable_host();
        support.items = [(CGROUP_BPF_ITEM_ABSENT, 0); CGROUP_BPF_ITEM_NUM as usize];
        for slot in slots {
            support.items[*slot as usize] = (CGROUP_BPF_ITEM_PAGE_STATE, 1);
        }
        BpfReadConfig::from_support(&support)
    };

    // One component of each aggregate, plus pgrefill, is enough. This host
    // has no *_PROACTIVE counters and must still use BPF.
    let config = with(&[
        CGROUP_BPF_ITEM_PGREFILL,
        CGROUP_BPF_ITEM_PGSCAN_KSWAPD,
        CGROUP_BPF_ITEM_PGSTEAL_KSWAPD,
    ]);
    assert!(!skips_memory_stat(&config));

    // pgrefill missing -> the file, even with both aggregates available.
    let config = with(&[
        CGROUP_BPF_ITEM_PGSCAN_KSWAPD,
        CGROUP_BPF_ITEM_PGSTEAL_KSWAPD,
    ]);
    assert!(skips_memory_stat(&config));

    // An aggregate with no component at all -> the file.
    let config = with(&[CGROUP_BPF_ITEM_PGREFILL, CGROUP_BPF_ITEM_PGSCAN_KSWAPD]);
    assert!(skips_memory_stat(&config));
}

/// Exercises `mountinfo_has_hugetlb_accounting`, which reads the cgroup2 mount
/// options. It depends on two positions in the line: the filesystem type and the
/// super options, both after the " - " separator.
#[test]
fn mount_options_decide_hugetlb() {
    let with_option = "31 23 0:27 / /sys/fs/cgroup rw,nosuid,nodev,noexec,relatime \
         shared:9 - cgroup2 cgroup2 rw,nsdelegate,memory_hugetlb_accounting";
    let without = "31 23 0:27 / /sys/fs/cgroup rw,nosuid,nodev,noexec,relatime \
         shared:9 - cgroup2 cgroup2 rw,nsdelegate,memory_recursiveprot";
    assert!(mountinfo_has_hugetlb_accounting(with_option));
    assert!(!mountinfo_has_hugetlb_accounting(without));

    // Found among many mounts, and the cgroup2 line is the one that answers.
    let table = format!("22 1 0:5 / /proc rw shared:1 - proc proc rw\n{with_option}\n");
    assert!(mountinfo_has_hugetlb_accounting(&table));

    // The option must be on a cgroup2 mount, not any filesystem that happens
    // to name it.
    let other_fs = "31 23 0:27 / /mnt rw shared:9 - tmpfs tmpfs rw,memory_hugetlb_accounting";
    assert!(!mountinfo_has_hugetlb_accounting(other_fs));

    // A whole option, not a prefix of a longer one.
    let longer = "31 23 0:27 / /sys/fs/cgroup rw shared:9 - cgroup2 cgroup2 \
         rw,memory_hugetlb_accounting_v2";
    assert!(!mountinfo_has_hugetlb_accounting(longer));

    // Lines that do not parse are not accounting, and must not panic.
    assert!(!mountinfo_has_hugetlb_accounting(""));
    assert!(!mountinfo_has_hugetlb_accounting("no separator here"));
    assert!(!mountinfo_has_hugetlb_accounting(
        "31 23 0:27 / /x rw - cgroup2"
    ));
}
