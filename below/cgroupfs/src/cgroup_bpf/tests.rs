// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! Whether the values BPF reports are the values the cgroupfs files report.
//!
//! These read real cgroups, so they need root, a cgroup2 hierarchy and a kernel
//! that loads the cgroup iterator. They are marked `#[test]` only in the VM test
//! target, which has all three; elsewhere they are compiled but not run, so a
//! change that breaks them still fails the ordinary build. There is no runtime
//! skip: where they run they must pass, rather than quietly testing nothing.
//!
//! A live host keeps moving while it is measured, so each field is compared the
//! strictest way that is sound for it:
//!
//! * settings the test wrote itself: compared exactly
//! * counters that only rise: read the file, take the snapshot, read the file
//!   again, and require the BPF value to land between the two reads
//! * every field: present on both sides, or absent on both sides
//!
//! Gauges that move in both directions, like memory.current and the memory.stat
//! sizes, cannot be pinned this way on a live tree. Comparing those exactly
//! needs a cgroup that is doing nothing, which is the fixture test still to
//! come.

// Without the feature these are ordinary functions that nothing calls, which is
// the point: they still have to compile, so a change that breaks them fails the
// ordinary build rather than waiting for a VM run.
#![cfg_attr(not(feature = "vmtest"), allow(dead_code))]

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use super::*;
use crate::CgroupReader;

/// Take one BPF snapshot of the whole tree, configured the way the driver
/// configures it.
fn bpf_snapshot() -> CgroupBpfSnapshot {
    // The skeleton borrows this, so it has to outlive the iterator below.
    let mut object = MaybeUninit::uninit();
    let mut iter = load_and_attach_iter(
        &mut object,
        Path::new(crate::DEFAULT_CG_ROOT),
        false,
        CgroupIterOrder::DescendantsPre,
    )
    .expect("failed to load and attach the cgroup iterator");
    iter.configure(&probe_host());
    iter.collect(&mut CputimeAdjuster::default())
        .expect("failed to traverse the cgroup tree")
}

/// A cgroup the test owns, so its values are known and nothing else changes
/// them. Removed on drop, including when a test fails.
struct FixtureCgroup {
    path: PathBuf,
}

impl FixtureCgroup {
    fn new(name: &str) -> Self {
        let path = Path::new(crate::DEFAULT_CG_ROOT).join(name);
        let _ = fs::remove_dir(&path);
        fs::create_dir(&path).expect("failed to create the fixture cgroup");
        Self { path }
    }

    fn write(&self, file: &str, value: &str) {
        fs::write(self.path.join(file), value)
            .unwrap_or_else(|e| panic!("failed to write {file}: {e}"));
    }

    fn reader(&self) -> CgroupReader<'static> {
        CgroupReader::new(self.path.clone()).expect("failed to open the fixture cgroup")
    }

    fn inode(&self) -> u64 {
        self.reader()
            .read_inode_number()
            .expect("failed to stat the fixture cgroup")
    }
}

impl Drop for FixtureCgroup {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

/// What one cgroup's files say, read as a group so a snapshot can be taken
/// between two of these.
struct FileRead {
    cpu: Option<crate::CpuStat>,
    events: Option<crate::MemoryEvents>,
    memory_stat: Option<crate::MemoryStat>,
    current: Option<u64>,
    min: Option<i64>,
    low: Option<i64>,
    high: Option<i64>,
    max: Option<i64>,
}

fn read_all(reader: &CgroupReader<'_>) -> FileRead {
    FileRead {
        cpu: reader.read_cpu_stat().ok(),
        events: reader.read_memory_events().ok(),
        memory_stat: reader.read_memory_stat().ok(),
        current: reader.read_memory_current().ok(),
        min: reader.read_memory_min().ok(),
        low: reader.read_memory_low().ok(),
        high: reader.read_memory_high().ok(),
        max: reader.read_memory_max().ok(),
    }
}

/// The memory.stat counters that only ever rise. Everything else in the file is
/// a gauge that moves both ways, which a live tree cannot pin down; those are
/// checked for presence here and compared by value on an idle cgroup.
fn rising_memory_stat_counters(stat: &crate::MemoryStat) -> [(&'static str, Option<u64>); 17] {
    [
        ("pgfault", stat.pgfault),
        ("pgmajfault", stat.pgmajfault),
        ("pgrefill", stat.pgrefill),
        ("pgscan", stat.pgscan),
        ("pgsteal", stat.pgsteal),
        ("pgactivate", stat.pgactivate),
        ("pgdeactivate", stat.pgdeactivate),
        ("pglazyfree", stat.pglazyfree),
        ("pglazyfreed", stat.pglazyfreed),
        ("thp_fault_alloc", stat.thp_fault_alloc),
        ("thp_collapse_alloc", stat.thp_collapse_alloc),
        ("workingset_refault_anon", stat.workingset_refault_anon),
        ("workingset_refault_file", stat.workingset_refault_file),
        ("workingset_activate_anon", stat.workingset_activate_anon),
        ("workingset_activate_file", stat.workingset_activate_file),
        ("workingset_restore_anon", stat.workingset_restore_anon),
        ("workingset_restore_file", stat.workingset_restore_file),
    ]
}

/// Which fields a memory.stat has at all, by name, so the two sources can be
/// checked to agree on that without naming the fields here.
fn present_memory_stat_fields(stat: &crate::MemoryStat) -> Vec<String> {
    serde_json::to_value(stat)
        .expect("MemoryStat serializes")
        .as_object()
        .expect("MemoryStat is an object")
        .iter()
        .filter(|(_, v)| !v.is_null())
        .map(|(k, _)| k.clone())
        .collect()
}

/// Check a counter that only rises against the two file reads that bracket the
/// snapshot. Landing outside that window is a real disagreement, not movement.
fn assert_between(
    name: &str,
    cgroup: &str,
    before: Option<u64>,
    bpf: Option<u64>,
    after: Option<u64>,
) {
    assert_eq!(
        bpf.is_some(),
        before.is_some(),
        "{cgroup}: {name} present in one source and not the other"
    );
    let (Some(bpf), Some(before), Some(after)) = (bpf, before, after) else {
        return;
    };
    assert!(
        bpf >= before && bpf <= after,
        "{cgroup}: {name} = {bpf} from BPF, outside the file's {before}..={after}"
    );
}

/// Exercises the whole read path end to end on this host: loading the program,
/// attaching the iterator, and draining a snapshot. It is a preamble rather than
/// a test of any one function -- if the environment cannot do this, say so here
/// instead of letting the later tests pass by comparing nothing.
#[cfg_attr(feature = "vmtest", test)]
fn environment_is_bpf_capable() {
    assert_eq!(
        // SAFETY: geteuid() reads the calling process's effective uid and
        // cannot fail or touch memory.
        unsafe { libc::geteuid() },
        0,
        "these tests need root to load BPF and to create a cgroup"
    );
    assert!(
        Path::new(crate::DEFAULT_CG_ROOT)
            .join("cgroup.controllers")
            .exists(),
        "no cgroup2 hierarchy at {}",
        crate::DEFAULT_CG_ROOT
    );
    assert!(
        probe_host().can_flush_rstat(),
        "kernel exports no rstat flush kfunc, so BPF would never be used here"
    );
    assert!(
        !bpf_snapshot().is_empty(),
        "the iterator ran but reported no cgroups"
    );
}

/// Exercises reading the memory limits through BPF, against a cgroup whose
/// limits the test set itself. Four distinct values mean two of them being
/// swapped fails, and the values are checked against both the file and the
/// numbers written, so the two sides being wrong in the same way also fails.
#[cfg_attr(feature = "vmtest", test)]
fn configured_values_match_the_file_exactly() {
    let fixture = FixtureCgroup::new("below_bpf_parity_test");
    fixture.write("memory.min", "1048576");
    fixture.write("memory.low", "2097152");
    fixture.write("memory.high", "4194304");
    fixture.write("memory.max", "8388608");
    fixture.write("memory.oom.group", "1");

    let reader = fixture.reader();
    let snapshot = bpf_snapshot();
    let stat = snapshot
        .get(&fixture.inode())
        .expect("the fixture cgroup is missing from the snapshot");

    let from_file = |v: crate::Result<i64>| Some(v.expect("failed to read the file"));
    assert_eq!(stat.memory_min, from_file(reader.read_memory_min()));
    assert_eq!(stat.memory_low, from_file(reader.read_memory_low()));
    assert_eq!(stat.memory_high, from_file(reader.read_memory_high()));
    assert_eq!(stat.memory_max, from_file(reader.read_memory_max()));
    assert_eq!(
        stat.memory_oom_group,
        Some(
            reader
                .read_memory_oom_group()
                .expect("failed to read memory.oom.group")
        )
    );

    assert_eq!(stat.memory_min, Some(1048576));
    assert_eq!(stat.memory_low, Some(2097152));
    assert_eq!(stat.memory_high, Some(4194304));
    assert_eq!(stat.memory_max, Some(8388608));
}

/// Exercises the BPF read of cpu.stat, memory.events and the memory limits
/// against the same cgroups' files, over every cgroup on the host. This is the
/// broad check: hundreds of cgroups with genuinely varied values, which no
/// fixture reaches. It also checks the join by cgroup id lines the two sources
/// up on the same cgroup.
#[cfg_attr(feature = "vmtest", test)]
fn values_match_the_file_across_the_live_tree() {
    let root = CgroupReader::root().expect("failed to open the cgroup root");
    let mut cgroups = vec![(String::from("/"), root)];
    let mut next = 0;
    while next < cgroups.len() {
        // Collected before pushing, so the borrow of the parent ends first.
        let children: Vec<_> = match cgroups[next].1.child_cgroup_iter() {
            Ok(children) => children
                .map(|child| (child.name().to_string_lossy().into_owned(), child))
                .collect(),
            Err(_) => Vec::new(),
        };
        cgroups.extend(children);
        next += 1;
    }
    assert!(cgroups.len() > 1, "found no cgroups to compare");

    // File, then BPF, then file again, so a counter that rose while the
    // snapshot was taken is bracketed rather than called a mismatch.
    let before: Vec<_> = cgroups.iter().map(|(_, r)| read_all(r)).collect();
    let snapshot = bpf_snapshot();
    let after: Vec<_> = cgroups.iter().map(|(_, r)| read_all(r)).collect();

    let mut compared = 0;
    let mut memory_stat_compared = 0;
    for (i, (name, reader)) in cgroups.iter().enumerate() {
        let Ok(inode) = reader.read_inode_number() else {
            continue;
        };
        // Absent means past the map's capacity, or created after the traversal.
        let Some(stat) = snapshot.get(&inode) else {
            continue;
        };

        if let (Some(bpf), Some(b), Some(a)) = (&stat.cpu_stat, &before[i].cpu, &after[i].cpu) {
            assert_between(
                "cpu.usage_usec",
                name,
                b.usage_usec,
                bpf.usage_usec,
                a.usage_usec,
            );
            assert_between(
                "cpu.nr_periods",
                name,
                b.nr_periods,
                bpf.nr_periods,
                a.nr_periods,
            );
            assert_between(
                "cpu.nr_throttled",
                name,
                b.nr_throttled,
                bpf.nr_throttled,
                a.nr_throttled,
            );
            assert_between(
                "cpu.throttled_usec",
                name,
                b.throttled_usec,
                bpf.throttled_usec,
                a.throttled_usec,
            );
            // The user/system split is scaled, and the kernel clamps it against
            // its own history where a fresh snapshot has none, so the halves are
            // not comparable one by one. Their sum is, to within the rounding:
            // the two halves add up to runtime exactly in nanoseconds, but each
            // is divided by 1000 on its own, so the two truncations can lose a
            // microsecond between them. cpu.stat is printed the same way.
            if let (Some(user), Some(system), Some(usage)) =
                (bpf.user_usec, bpf.system_usec, bpf.usage_usec)
            {
                let lost = usage.saturating_sub(user + system);
                assert!(
                    user + system <= usage && lost <= 1,
                    "{name}: the cpu split adds up to {} against a usage of {usage}",
                    user + system
                );
            }
            compared += 1;
        }

        if let (Some(bpf), Some(b), Some(a)) =
            (&stat.memory_events, &before[i].events, &after[i].events)
        {
            assert_between("memory.events.low", name, b.low, bpf.low, a.low);
            assert_between("memory.events.high", name, b.high, bpf.high, a.high);
            assert_between("memory.events.max", name, b.max, bpf.max, a.max);
            assert_between("memory.events.oom", name, b.oom, bpf.oom, a.oom);
            assert_between(
                "memory.events.oom_kill",
                name,
                b.oom_kill,
                bpf.oom_kill,
                a.oom_kill,
            );
            compared += 1;
        }

        // memory.current is a gauge, so a live tree cannot pin its value down.
        // Whether it is there at all can be pinned: BPF supplying it where the
        // file has none, or the reverse, is a real disagreement.
        assert_eq!(
            stat.memory_current.is_some(),
            before[i].current.is_some(),
            "{name}: memory.current present in one source and not the other"
        );

        // memory.stat, whenever BPF supplied it. Whether it does is decided by
        // the startup probe from the kernel's own BTF, so this compares on any
        // kernel that has the memcg kfuncs and quietly does nothing on one that
        // does not -- no version is assumed either way.
        if let (Some(bpf), Some(b), Some(a)) = (
            &stat.memory_stat,
            &before[i].memory_stat,
            &after[i].memory_stat,
        ) {
            assert_eq!(
                present_memory_stat_fields(bpf),
                present_memory_stat_fields(b),
                "{name}: memory.stat has different fields from BPF than from the file"
            );
            for ((field, bpf_value), ((_, b_value), (_, a_value))) in
                rising_memory_stat_counters(bpf).into_iter().zip(
                    rising_memory_stat_counters(b)
                        .into_iter()
                        .zip(rising_memory_stat_counters(a)),
                )
            {
                assert_between(field, name, b_value, bpf_value, a_value);
            }
            memory_stat_compared += 1;
            compared += 1;
        }

        // Limits are settings, so they are equal. If one changed between the
        // two file reads the field is skipped rather than reported.
        for (field, file_before, bpf_value, file_after) in [
            ("memory.min", before[i].min, stat.memory_min, after[i].min),
            ("memory.low", before[i].low, stat.memory_low, after[i].low),
            (
                "memory.high",
                before[i].high,
                stat.memory_high,
                after[i].high,
            ),
            ("memory.max", before[i].max, stat.memory_max, after[i].max),
        ] {
            if file_before == file_after && file_before.is_some() {
                assert_eq!(
                    bpf_value, file_before,
                    "{name}: {field} disagrees with the file"
                );
                compared += 1;
            }
        }
    }

    assert!(
        compared > 0,
        "walked the tree but compared nothing, so the join by cgroup id is broken"
    );

    // memory.stat is the largest part of the record and the one BPF cannot
    // always supply. Rather than guess from a kernel version, ask the probe what
    // it decided for this host: if it said BPF supplies memory.stat, the loop
    // above has to have compared it somewhere. That is what stops it passing by
    // never running.
    let supplies_memory_stat =
        BpfReadConfig::from_support(&probe_host()).skip_files & (1 << CGROUP_FILES_MEM_STAT) == 0;
    // Say which way it went. Whether a kernel has the memcg kfuncs is not
    // something to read off its version number, and this is the only place that
    // knows, so it is worth having in the log rather than inferring later.
    eprintln!(
        "memory.stat served by BPF here: {supplies_memory_stat}; compared on \
         {memory_stat_compared} cgroups"
    );
    assert_eq!(
        supplies_memory_stat,
        memory_stat_compared > 0,
        "the probe says BPF supplies memory.stat here: {supplies_memory_stat}, \
         but it was compared on {memory_stat_compared} cgroups"
    );
}

/// Exercises the program's `level == 0` guards, which skip the root cgroup. The
/// root has no cpu.stat or memory files to fall back to, so anything BPF
/// reported for it would be a value the file cannot confirm.
#[cfg_attr(feature = "vmtest", test)]
fn root_cgroup_is_left_to_the_file() {
    let root = CgroupReader::root().expect("failed to open the cgroup root");
    let inode = root.read_inode_number().expect("failed to stat the root");
    let snapshot = bpf_snapshot();
    let stat = snapshot
        .get(&inode)
        .expect("the root cgroup is missing from the snapshot");

    assert!(stat.cpu_stat.is_none(), "the root's cpu came from BPF");
    assert!(stat.memory_current.is_none());
    assert!(stat.memory_stat.is_none());
    assert!(stat.memory_events.is_none());
    assert!(stat.memory_min.is_none());

    // cgroup.stat is read off the cgroup itself, and the root has that file.
    assert!(
        stat.cgroup_stat.is_some(),
        "the root should still have cgroup.stat"
    );
}
