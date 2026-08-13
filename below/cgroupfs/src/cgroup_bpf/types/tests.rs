// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! What a BPF record decodes into: which groups appear, where each counter
//! lands, and how the cpu user/system split behaves over successive samples.

use super::*;

/// A record with every group marked valid, so a test can clear the one bit
/// it is about. `Default` zeroes the counters, which decode as real zeros.
fn valid_record() -> CgroupBpfRecord {
    CgroupBpfRecord {
        valid: (1 << CGROUP_FILES_CPU_USAGE)
            | (1 << CGROUP_FILES_CPU_THROTTLE)
            | (1 << CGROUP_FILES_MEM_CURRENT)
            | (1 << CGROUP_FILES_MEM_STAT)
            | (1 << CGROUP_FILES_MEM_EVENTS)
            | (1 << CGROUP_FILES_MEM_LIMITS)
            | (1 << CGROUP_FILES_CGROUP_STAT),
        ..Default::default()
    }
}

fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).expect("stat structs serialize")
}

fn decode(rec: &CgroupBpfRecord) -> CgroupBpfStat {
    decode_record(rec, &mut CputimeAdjuster::default())
}

/// Exercises `decode_record` on a record the program could not fill. Nothing
/// may be invented from it: every group must come out empty, and that emptiness
/// is what sends the collector to the files.
#[test]
fn unset_valid_bits_yield_no_data() {
    let stat = decode(&CgroupBpfRecord::default());
    assert!(stat.cpu_stat.is_none());
    assert!(stat.memory_current.is_none());
    assert!(stat.memory_stat.is_none());
    assert!(stat.memory_events.is_none());
    assert!(stat.memory_events_local.is_none());
    assert!(stat.cgroup_stat.is_none());
    assert!(stat.memory_min.is_none());
    assert!(stat.memory_low.is_none());
    assert!(stat.memory_high.is_none());
    assert!(stat.memory_max.is_none());
    assert!(stat.memory_oom_group.is_none());
}

/// Exercises `decode_record`'s per-group gating, one valid bit at a time. A bit
/// unlocks its own group and nothing else, so a group the kernel could not
/// supply cannot be filled from another group's leftovers.
#[test]
fn each_valid_bit_unlocks_only_its_own_group() {
    let only = |bit: u32| {
        decode(&CgroupBpfRecord {
            valid: 1 << bit,
            ..Default::default()
        })
    };

    let cpu = only(CGROUP_FILES_CPU_USAGE);
    assert!(cpu.cpu_stat.is_some());
    assert!(cpu.memory_current.is_none() && cpu.memory_stat.is_none());
    assert!(cpu.cgroup_stat.is_none() && cpu.memory_events.is_none());

    let cur = only(CGROUP_FILES_MEM_CURRENT);
    assert!(cur.memory_current.is_some());
    assert!(cur.cpu_stat.is_none() && cur.memory_stat.is_none());

    let events = only(CGROUP_FILES_MEM_EVENTS);
    assert!(events.memory_events.is_some() && events.memory_events_local.is_some());
    assert!(events.memory_min.is_none() && events.cpu_stat.is_none());

    let limits = only(CGROUP_FILES_MEM_LIMITS);
    assert!(limits.memory_min.is_some() && limits.memory_oom_group.is_some());
    assert!(limits.memory_events.is_none() && limits.memory_current.is_none());

    let cg = only(CGROUP_FILES_CGROUP_STAT);
    assert!(cg.cgroup_stat.is_some());
    assert!(cg.cpu_stat.is_none() && cg.memory_stat.is_none());
}

/// Exercises `CputimeAdjuster::adjust` splitting raw cpu times into the pair
/// cpu.stat prints. It follows the kernel's `cputime_adjust`: the scaled halves
/// sum to runtime, and the zero cases go to user, not system.
#[test]
fn cpu_times_match_the_kernels_split() {
    let mut adj = CputimeAdjuster::default();

    // No ticks yet. The kernel gives all runtime to user; reporting it as
    // system would invert what cpu.stat prints.
    assert_eq!(adj.adjust(1, 0, 0, 100), (100, 0));
    // Only user ticks -> still all user.
    assert_eq!(adj.adjust(2, 5, 0, 100), (100, 0));
    // Only system ticks -> all system.
    assert_eq!(adj.adjust(3, 0, 5, 100), (0, 100));

    // Both present: scale to runtime, keeping the ratio, and sum exactly.
    let (user, system) = adj.adjust(4, 300, 100, 1000);
    assert_eq!(system, 250);
    assert_eq!(user, 750);
    assert_eq!(user + system, 1000);

    // stime * rtime overflows u64 without a wider intermediate.
    let big = u64::MAX / 2;
    let (user, system) = adj.adjust(5, big, big, big);
    assert_eq!(user + system, big);
}

/// Exercises `CputimeAdjuster::adjust` over successive samples of one cgroup.
/// The split it reports must never go backwards: scaling alone is not monotonic,
/// and with stime unchanged and utime growing the scaled system time falls,
/// which below reports as no reading at all.
#[test]
fn cpu_times_never_go_backwards() {
    let mut adj = CputimeAdjuster::default();

    // Without a clamp this pair would fall from 100_000 to 97_619.
    let (_, first) = adj.adjust(1, 1_000_000, 100_000, 1_100_000);
    let (_, second) = adj.adjust(1, 2_000_000, 100_000, 2_050_000);
    assert!(
        second >= first,
        "system time went backwards: {first} -> {second}"
    );

    // Monotonic across a long run of user-only growth, for both halves.
    let mut adj = CputimeAdjuster::default();
    let (mut last_user, mut last_system) = (0, 0);
    for i in 1..200u64 {
        let (user, system) = adj.adjust(7, i * 1_000_000, 100_000, i * 1_050_000);
        assert!(user >= last_user, "user went backwards at {i}");
        assert!(system >= last_system, "system went backwards at {i}");
        (last_user, last_system) = (user, system);
    }
}

/// Exercises `CputimeAdjuster`'s per-cgroup memory and `forget_dead`. Each
/// cgroup is clamped against its own history, and that history does not outlive
/// the cgroup.
#[test]
fn cpu_times_are_clamped_per_cgroup() {
    let mut adj = CputimeAdjuster::default();

    // A busy cgroup's history must not clamp a different, idle one.
    adj.adjust(1, 0, 1_000_000, 1_000_000);
    assert_eq!(adj.adjust(2, 0, 0, 10), (10, 0));

    // Runtime that has not passed what was already reported repeats it,
    // rather than letting the pair drop.
    let first = adj.adjust(3, 100, 100, 1000);
    assert_eq!(adj.adjust(3, 100, 100, 500), first);

    // A cgroup missing from a snapshot is forgotten, so a later cgroup that
    // reuses the id starts clean.
    let mut live = CgroupBpfSnapshot::new();
    live.insert(1, CgroupBpfStat::default());
    adj.forget_dead(&live);
    assert_eq!(adj.adjust(3, 0, 0, 10), (10, 0));
}

/// Exercises `decode_memory_stat`'s field mapping. Every counter must land
/// where the file puts it, and a distinct value per field catches two lines
/// being swapped in the hand-written mapping.
#[test]
fn memory_stat_fields_decode_by_name() {
    let rec = CgroupBpfRecord {
        ms_anon: 1,
        ms_file: 2,
        ms_kernel: 3,
        ms_kernel_stack: 4,
        ms_sock: 5,
        ms_shmem: 6,
        ms_file_mapped: 7,
        ms_file_dirty: 8,
        ms_file_writeback: 9,
        ms_inactive_anon: 10,
        ms_active_anon: 11,
        ms_inactive_file: 12,
        ms_active_file: 13,
        ms_unevictable: 14,
        ms_slab_reclaimable: 15,
        ms_slab_unreclaimable: 16,
        ms_pgfault: 17,
        ms_pgmajfault: 18,
        ..valid_record()
    };
    let stat = decode(&rec).memory_stat.expect("memory.stat is valid");
    assert_eq!(stat.anon, Some(1));
    assert_eq!(stat.file, Some(2));
    assert_eq!(stat.kernel, Some(3));
    assert_eq!(stat.kernel_stack, Some(4));
    assert_eq!(stat.sock, Some(5));
    assert_eq!(stat.shmem, Some(6));
    assert_eq!(stat.file_mapped, Some(7));
    assert_eq!(stat.file_dirty, Some(8));
    assert_eq!(stat.file_writeback, Some(9));
    assert_eq!(stat.inactive_anon, Some(10));
    assert_eq!(stat.active_anon, Some(11));
    assert_eq!(stat.inactive_file, Some(12));
    assert_eq!(stat.active_file, Some(13));
    assert_eq!(stat.unevictable, Some(14));
    assert_eq!(stat.slab_reclaimable, Some(15));
    assert_eq!(stat.slab_unreclaimable, Some(16));
    assert_eq!(stat.pgfault, Some(17));
    assert_eq!(stat.pgmajfault, Some(18));
    // slab is the file's sum of the two components, not a counter of its own.
    assert_eq!(stat.slab, Some(31));
}

/// Exercises `decode_memory_stat`'s sentinel for a counter the kernel does not
/// track, which the program marks with u64::MAX. It has to decode to no reading,
/// while a tracked zero stays a zero.
#[test]
fn unavailable_counters_decode_to_none() {
    let rec = CgroupBpfRecord {
        ms_anon: u64::MAX,
        ms_file: 0,
        ms_zswap: u64::MAX,
        ms_slab_reclaimable: u64::MAX,
        ms_slab_unreclaimable: 8,
        ..valid_record()
    };
    let stat = decode(&rec).memory_stat.expect("memory.stat is valid");
    assert_eq!(stat.anon, None, "untracked counter must not read as 0");
    assert_eq!(stat.file, Some(0), "a tracked zero is a real value");
    assert_eq!(stat.zswap, None);
    // The sum needs both halves; one missing makes the total unknown, not
    // partial.
    assert_eq!(stat.slab, None);
}

/// Exercises the memory limit decode: the file's "max" sentinel and its
/// page-to-byte scaling. Four distinct values catch a transposition among the
/// four limits.
#[test]
fn limits_use_the_max_sentinel_and_page_scaling() {
    let page = *PAGE_SIZE;
    let rec = CgroupBpfRecord {
        mem_min: 1,
        mem_low: 2,
        mem_high: 3,
        mem_max: page_counter_max(),
        mem_oom_group: 1,
        memory_current_pages: 4,
        ..valid_record()
    };
    let stat = decode(&rec);
    assert_eq!(stat.memory_min, Some(page as i64));
    assert_eq!(stat.memory_low, Some(2 * page as i64));
    assert_eq!(stat.memory_high, Some(3 * page as i64));
    // "max" in the file, which below spells -1.
    assert_eq!(stat.memory_max, Some(-1));
    assert_eq!(stat.memory_oom_group, Some(1));
    assert_eq!(stat.memory_current, Some(4 * page as i64));

    // Anything at or above the sentinel is "max", not a huge byte count.
    let rec = CgroupBpfRecord {
        mem_min: u64::MAX,
        ..valid_record()
    };
    assert_eq!(decode(&rec).memory_min, Some(-1));
}

/// Exercises which fields a decoded struct leaves empty, which is a contract
/// with the file reader: anything BPF cannot supply must be visibly absent, so
/// the rules in coverage.rs stay honest. Compared through serde, so a new field
/// joins the check without being named here.
#[test]
fn decoded_presence_matches_the_contract() {
    let empty = |value: &serde_json::Value| -> Vec<String> {
        value
            .as_object()
            .expect("stat structs serialize as objects")
            .iter()
            .filter(|(_, v)| v.is_null())
            .map(|(k, _)| k.clone())
            .collect()
    };
    let stat = decode(&valid_record());

    // Every counter the program filled is present.
    let cpu = to_json(stat.cpu_stat.expect("cpu.stat is valid"));
    assert_eq!(empty(&cpu), Vec::<String>::new());
    let cgroup = to_json(stat.cgroup_stat.expect("cgroup.stat is valid"));
    assert_eq!(empty(&cgroup), Vec::<String>::new());
    let memory = to_json(stat.memory_stat.expect("memory.stat is valid"));
    assert_eq!(
        empty(&memory),
        Vec::<String>::new(),
        "a memory.stat field with no source would silently lose the line"
    );

    // sock_throttled is the one events field BPF does not read; where the
    // kernel has it, coverage.rs sends the whole file to the file reader.
    let events = to_json(stat.memory_events.expect("memory.events is valid"));
    assert_eq!(empty(&events), vec!["sock_throttled".to_owned()]);
    let local = to_json(
        stat.memory_events_local
            .expect("memory.events.local is valid"),
    );
    assert_eq!(empty(&local), vec!["sock_throttled".to_owned()]);
}
