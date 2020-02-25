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

use std::fs::File;
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;

use procfs_thrift::types::PidState;

use crate::ProcReader;
use crate::PAGE_SIZE;

struct TestProcfs {
    tempdir: TempDir,
}

impl TestProcfs {
    fn new() -> TestProcfs {
        TestProcfs {
            tempdir: TempDir::new().expect("Failed to create tempdir"),
        }
    }

    fn path(&self) -> &Path {
        self.tempdir.path()
    }

    fn get_reader(&self) -> ProcReader {
        ProcReader::new_with_custom_procfs(self.path().to_path_buf())
    }

    fn create_file_with_content_full_path<P: AsRef<Path>>(&self, path: P, content: &[u8]) {
        let mut file =
            File::create(&path).expect(&format!("Failed to create {}", path.as_ref().display()));
        file.write_all(content)
            .expect(&format!("Failed to write to {}", path.as_ref().display()));
    }

    fn create_file_with_content<P: AsRef<Path>>(&self, p: P, content: &[u8]) {
        let path = self.path().join(p);
        self.create_file_with_content_full_path(path, content);
    }

    fn create_pid_file_with_content<P: AsRef<Path>>(&self, pid: u32, p: P, content: &[u8]) {
        let pid_dir = self.path().join(pid.to_string());
        if !pid_dir.exists() {
            std::fs::create_dir(&pid_dir).expect("Failed to create pid dir");
        }
        let path = pid_dir.join(p);
        self.create_file_with_content_full_path(path, content);
    }
}

#[test]
fn test_stat_success() {
    let stat = b"cpu  152068189 10802578 74452328 5513630980 5288390 0 1767719 0 0 0
cpu0 5444440 452370 3076696 230319654 224331 0 336368 0 0 0
cpu1 8448465 464353 3348782 227381351 220817 0 117305 0 0 0
cpu2 8026518 464055 3282421 227922992 221338 0 74157 0 0 0
cpu3 7343223 445316 3208287 228716633 217921 0 59636 0 0 0
cpu4 7032469 451903 3164894 229075253 215581 0 53305 0 0 0
cpu5 6710964 444291 3129304 229436472 216684 0 48777 0 0 0
cpu6 6369833 453695 3099787 229781771 223354 0 48604 0 0 0
cpu7 6265491 439547 3080483 229939368 214955 0 46759 0 0 0
cpu8 6142526 438021 3075538 230049431 227041 0 46250 0 0 0
cpu9 6124418 452468 3062529 230073290 216237 0 45647 0 0 0
cpu10 6253875 450223 3049983 229894105 220705 0 45386 0 0 0
cpu11 6153930 444146 3056614 230060355 213563 0 45135 0 0 0
cpu12 6206970 458147 3197631 228603207 242081 0 302601 0 0 0
cpu13 6094689 445538 3066545 230075722 219005 0 45676 0 0 0
cpu14 5950313 443723 3056300 230247332 214160 0 45070 0 0 0
cpu15 6098008 461004 3057722 230065810 221169 0 45598 0 0 0
cpu16 5858941 461330 3064073 230307586 207675 0 45956 0 0 0
cpu17 6030100 442988 3038059 230181264 219816 0 45341 0 0 0
cpu18 5917745 448756 3050060 230279037 215704 0 45064 0 0 0
cpu19 6042834 433710 3058862 230151162 214883 0 45179 0 0 0
cpu20 5804757 444950 3057184 230372774 224962 0 46132 0 0 0
cpu21 5935027 455253 3052007 230250433 209736 0 45325 0 0 0
cpu22 5874699 446579 3061270 230279298 235842 0 45808 0 0 0
cpu23 5937943 460201 3057286 230166670 230819 0 42628 0 0 0
intr 29638874355 54 9 0 0 14325 0 0 0 1 0 19 0 5 0 0 0 0 0 0 0 0 0 0 0 0 14 0 109094370 0 464199486 9840 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
ctxt 48203489122
btime 1569873858
processes 105984108
procs_running 4
procs_blocked 0
softirq 15518280031 0 3506477306 138437 1090758178 117192437 0 24543 800441241 93 1413313204
";

    let procfs = TestProcfs::new();
    procfs.create_file_with_content("stat", stat);
    let reader = procfs.get_reader();
    let stat = reader.read_stat().expect("Failed to read stat file");
    let total_cpu = stat.total_cpu.expect("Did not read total cpu");

    assert_eq!(total_cpu.user_usec, Some(1520681890000));
    assert_eq!(total_cpu.nice_usec, Some(108025780000));
    assert_eq!(total_cpu.system_usec, Some(744523280000));
    assert_eq!(total_cpu.idle_usec, Some(55136309800000));
    assert_eq!(total_cpu.iowait_usec, Some(52883900000));
    assert_eq!(total_cpu.irq_usec, Some(0));
    assert_eq!(total_cpu.softirq_usec, Some(17677190000));
    assert_eq!(total_cpu.stolen_usec, Some(0));
    assert_eq!(total_cpu.guest_usec, Some(0));

    assert_eq!(total_cpu.guest_nice_usec, Some(0));

    let cpu23 = &stat.cpus.expect("Failed to read cpus")[23];
    assert_eq!(cpu23.user_usec, Some(59379430000));

    assert_eq!(stat.total_interrupt_count, Some(29638874355));
    assert_eq!(stat.context_switches, Some(48203489122));
    assert_eq!(stat.boot_time_epoch_secs, Some(1569873858));
    assert_eq!(stat.total_processes, Some(105984108));
    assert_eq!(stat.running_processes, Some(4));
    assert_eq!(stat.blocked_processes, Some(0));
}

#[test]
fn test_meminfo_success() {
    let meminfo = b"MemTotal:       58603192 kB
MemFree:         5298784 kB
MemAvailable:   48603448 kB
Buffers:            3152 kB
Cached:         38116592 kB
SwapCached:        88940 kB
Active:         22739928 kB
Inactive:       22218120 kB
Active(anon):    4396572 kB
Inactive(anon):  2459820 kB
Active(file):   18343356 kB
Inactive(file): 19758300 kB
Unevictable:       14680 kB
Mlocked:           14660 kB
SwapTotal:      20971512 kB
SwapFree:       18537912 kB
Dirty:             40896 kB
Writeback:             0 kB
AnonPages:       6844752 kB
Mapped:           967552 kB
Shmem:             14384 kB
KReclaimable:    5873416 kB
Slab:            7564396 kB
SReclaimable:    5873416 kB
SUnreclaim:      1690980 kB
KernelStack:       58976 kB
PageTables:       157164 kB
NFS_Unstable:          0 kB
Bounce:                0 kB
WritebackTmp:          0 kB
CommitLimit:    50273108 kB
Committed_AS:   37020012 kB
VmallocTotal:   34359738367 kB
VmallocUsed:      229472 kB
VmallocChunk:          0 kB
Percpu:           179616 kB
HardwareCorrupted:     0 kB
AnonHugePages:     49152 kB
ShmemHugePages:        0 kB
ShmemPmdMapped:        0 kB
FileHugePages:      6144 kB
FilePmdMapped:      6144 kB
CmaTotal:              0 kB
CmaFree:               0 kB
HugePages_Total:       0
HugePages_Free:        0
HugePages_Rsvd:        0
HugePages_Surp:        0
Hugepagesize:       2048 kB
Hugetlb:               0 kB
DirectMap4k:    19445616 kB
DirectMap2M:    40323072 kB
DirectMap1G:     2097152 kB
";
    let procfs = TestProcfs::new();
    procfs.create_file_with_content("meminfo", meminfo);
    let reader = procfs.get_reader();

    let meminfo = reader.read_meminfo().expect("Failed to read meminfo");

    assert_eq!(meminfo.total, Some(58603192 * 1024));
    assert_eq!(meminfo.free, Some(5298784 * 1024));
    assert_eq!(meminfo.available, Some(48603448 * 1024));
    assert_eq!(meminfo.buffers, Some(3152 * 1024));
    assert_eq!(meminfo.cached, Some(38116592 * 1024));
    assert_eq!(meminfo.swap_cached, Some(88940 * 1024));
    assert_eq!(meminfo.active, Some(22739928 * 1024));
    assert_eq!(meminfo.inactive, Some(22218120 * 1024));
    assert_eq!(meminfo.active_anon, Some(4396572 * 1024));
    assert_eq!(meminfo.inactive_anon, Some(2459820 * 1024));
    assert_eq!(meminfo.active_file, Some(18343356 * 1024));
    assert_eq!(meminfo.inactive_file, Some(19758300 * 1024));
    assert_eq!(meminfo.unevictable, Some(14680 * 1024));
    assert_eq!(meminfo.mlocked, Some(14660 * 1024));
    assert_eq!(meminfo.swap_total, Some(20971512 * 1024));
    assert_eq!(meminfo.swap_free, Some(18537912 * 1024));
    assert_eq!(meminfo.dirty, Some(40896 * 1024));
    assert_eq!(meminfo.writeback, Some(0 * 1024));
    assert_eq!(meminfo.anon_pages, Some(6844752 * 1024));
    assert_eq!(meminfo.mapped, Some(967552 * 1024));
    assert_eq!(meminfo.shmem, Some(14384 * 1024));
    assert_eq!(meminfo.kreclaimable, Some(5873416 * 1024));
    assert_eq!(meminfo.slab, Some(7564396 * 1024));
    assert_eq!(meminfo.slab_reclaimable, Some(5873416 * 1024));
    assert_eq!(meminfo.slab_unreclaimable, Some(1690980 * 1024));
    assert_eq!(meminfo.kernel_stack, Some(58976 * 1024));
    assert_eq!(meminfo.page_tables, Some(157164 * 1024));
    assert_eq!(meminfo.anon_huge_pages, Some(49152 * 1024));
    assert_eq!(meminfo.shmem_huge_pages, Some(0 * 1024));
    assert_eq!(meminfo.file_huge_pages, Some(6144 * 1024));
    assert_eq!(meminfo.total_huge_pages, Some(0));
    assert_eq!(meminfo.free_huge_pages, Some(0));
}

#[test]
fn test_vmstat_success() {
    let vmstat = b"nr_free_pages 1091519
nr_zone_inactive_anon 629826
nr_zone_active_anon 1135006
nr_zone_inactive_file 4297925
nr_zone_active_file 5209569
nr_zone_unevictable 3671
nr_zone_write_pending 5053
nr_mlock 3666
nr_page_table_pages 40627
nr_kernel_stack 60688
nr_bounce 0
nr_free_cma 0
numa_hit 53871133588
numa_miss 0
numa_foreign 0
numa_interleave 4030327584
numa_local 53871133588
numa_other 0
nr_inactive_anon 629826
nr_active_anon 1135006
nr_inactive_file 4297925
nr_active_file 5209569
nr_unevictable 3671
nr_slab_reclaimable 1673011
nr_slab_unreclaimable 416090
nr_isolated_anon 0
nr_isolated_file 0
workingset_nodes 116449
workingset_refault 353022586
workingset_activate 138352977
workingset_restore 78012002
workingset_nodereclaim 12530649
nr_anon_pages 1761357
nr_mapped 219713
nr_file_pages 9534635
nr_dirty 5053
nr_writeback 0
nr_writeback_temp 0
nr_shmem 3593
nr_shmem_hugepages 0
nr_shmem_pmdmapped 0
nr_file_hugepages 3
nr_file_pmdmapped 3
nr_anon_transparent_hugepages 24
nr_unstable 0
nr_vmscan_write 3637794
nr_vmscan_immediate_reclaim 170180
nr_dirtied 4183721266
nr_written 3831980784
nr_kernel_misc_reclaimable 0
nr_dirty_threshold 2098385
nr_dirty_background_threshold 1047911
pgpgin 5245063123
pgpgout 13772335013
pswpin 2090956
pswpout 3637759
pgalloc_dma 0
pgalloc_dma32 1102828262
pgalloc_normal 55950688975
pgalloc_movable 0
allocstall_dma 0
allocstall_dma32 0
allocstall_normal 2848
allocstall_movable 63605
pgskip_dma 0
pgskip_dma32 0
pgskip_normal 0
pgskip_movable 0
pgfree 57608585956
pgactivate 3075441394
pgdeactivate 795490969
pglazyfree 1554548
pgfault 62574548442
pgmajfault 15472187
pglazyfreed 0
pgrefill 820237987
pgsteal_kswapd 1709049230
pgsteal_direct 5652651
pgscan_kswapd 1743683511
pgscan_direct 5877901
pgscan_direct_throttle 0
zone_reclaim_failed 0
pginodesteal 18572761
slabs_scanned 2323559533
kswapd_inodesteal 16409209
kswapd_low_wmark_hit_quickly 222842
kswapd_high_wmark_hit_quickly 16537
pageoutrun 350010
pgrotated 3602381
drop_pagecache 3
drop_slab 3
oom_kill 0
pgmigrate_success 552898079
pgmigrate_fail 638123
compact_migrate_scanned 1760793733
compact_free_scanned 8884040431
compact_isolated 1107787743
compact_stall 80260
compact_fail 78220
compact_success 2040
compact_daemon_wake 127036
compact_daemon_migrate_scanned 231407204
compact_daemon_free_scanned 1747990014
htlb_buddy_alloc_success 0
htlb_buddy_alloc_fail 0
unevictable_pgs_culled 902152601
unevictable_pgs_scanned 0
unevictable_pgs_rescued 902134607
unevictable_pgs_mlocked 902207719
unevictable_pgs_munlocked 902136748
unevictable_pgs_cleared 67305
unevictable_pgs_stranded 67305
thp_fault_alloc 290
thp_fault_fallback 397
thp_collapse_alloc 118618
thp_collapse_alloc_failed 37230
thp_file_alloc 0
thp_file_mapped 6
thp_split_page 5
thp_split_page_failed 0
thp_deferred_split_page 300
thp_split_pmd 5
thp_split_pud 0
thp_zero_page_alloc 0
thp_zero_page_alloc_failed 0
thp_swpout 0
thp_swpout_fallback 5
balloon_inflate 0
balloon_deflate 0
balloon_migrate 0
swap_ra 222407
swap_ra_hit 139012
";

    let procfs = TestProcfs::new();
    procfs.create_file_with_content("vmstat", vmstat);
    let reader = procfs.get_reader();
    let vmstat = reader.read_vmstat().expect("Failed to read vmstat file");

    assert_eq!(vmstat.pgpgin, Some(5245063123));
    assert_eq!(vmstat.pgpgout, Some(13772335013));
    assert_eq!(vmstat.pswpin, Some(2090956));
    assert_eq!(vmstat.pswpout, Some(3637759));
    assert_eq!(vmstat.pgsteal_kswapd, Some(1709049230));
    assert_eq!(vmstat.pgsteal_direct, Some(5652651));
    assert_eq!(vmstat.pgscan_kswapd, Some(1743683511));
    assert_eq!(vmstat.pgscan_direct, Some(5877901));
    assert_eq!(vmstat.oom_kill, Some(0));
}

#[test]
fn test_pid_stat() {
    let uptime = b"1631826.55 37530838.66";
    let stat = b"74718 (((bash process)) S 44786 74718 74718 34820 3561868 4194304 31346884 614468259 3 23315 14474 10887 1967513 339861 20 0 1 0 102803 224440320 12725 18446744073709551615 93972706258944 93972707333076 140732465518320 0 0 0 65536 3670020 1266777851 0 0 0 17 12 0 0 7 0 0 93972709432552 93972709479876 93972709523456 140732465525073 140732465525079 140732465525079 140732465528814 0";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(74718, "stat", stat);
    procfs.create_file_with_content("uptime", uptime);
    let reader = procfs.get_reader();
    let pidstat = reader
        .read_pid_stat(74718)
        .expect("Failed to read pid stat file");

    assert_eq!(pidstat.pid, Some(74718));
    assert_eq!(pidstat.comm, Some("((bash process)".to_string()));
    assert_eq!(pidstat.state, Some(PidState::SLEEPING));
    assert_eq!(pidstat.ppid, Some(44786));
    assert_eq!(pidstat.pgrp, Some(74718));
    assert_eq!(pidstat.session, Some(74718));
    assert_eq!(pidstat.minflt, Some(31346884));
    assert_eq!(pidstat.majflt, Some(3));
    assert_eq!(pidstat.user_usecs, Some(144740000));
    assert_eq!(pidstat.system_usecs, Some(108870000));
    assert_eq!(pidstat.num_threads, Some(1));
    assert_eq!(pidstat.running_secs, Some(1631827 /* rounded up */ - 1028));
    assert_eq!(pidstat.rss_bytes, Some(12725 * *PAGE_SIZE));
    assert_eq!(pidstat.processor, Some(12));
}

#[test]
fn test_pid_io() {
    let io = b"rchar: 1065638765191
wchar: 330982500707
syscr: 138384532
syscw: 27652984
read_bytes: 22577841152
write_bytes: 284070445056
cancelled_write_bytes: 5431947264
";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(1024, "io", io);
    let reader = procfs.get_reader();
    let pidio = reader
        .read_pid_io(1024)
        .expect("Failed to read pid stat file");

    assert_eq!(pidio.rbytes, Some(22577841152));
    assert_eq!(pidio.wbytes, Some(284070445056));
}

#[test]
fn test_pid_cgroup() {
    let cgroup = b"0::/user.slice/user-119756.slice/session-3.scope
";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(1024, "cgroup", cgroup);
    let reader = procfs.get_reader();
    let cgroup = reader
        .read_pid_cgroup(1024)
        .expect("Failed to read pid cgroup file");

    assert_eq!(cgroup, "/user.slice/user-119756.slice/session-3.scope");
}

#[test]
fn test_read_all_pids() {
    let io = b"rchar: 1065638765191
wchar: 330982500707
syscr: 138384532
syscw: 27652984
read_bytes: 22577841152
write_bytes: 284070445056
cancelled_write_bytes: 5431947264
";

    let stat = b"74718 (bash) S 44786 74718 74718 34820 3561868 4194304 31346884 614468259 3 23315 14474 10887 1967513 339861 20 0 1 0 102803 224440320 12725 18446744073709551615 93972706258944 93972707333076 140732465518320 0 0 0 65536 3670020 1266777851 0 0 0 17 12 0 0 7 0 0 93972709432552 93972709479876 93972709523456 140732465525073 140732465525079 140732465525079 140732465528814 0";

    let cgroup = b"0::/user.slice/user-119756.slice/session-3.scope
";
    let uptime = b"1631826.45 37530838.66";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(1024, "stat", stat);
    procfs.create_pid_file_with_content(1024, "io", io);
    procfs.create_pid_file_with_content(1024, "cgroup", cgroup);
    procfs.create_pid_file_with_content(1025, "stat", stat);
    procfs.create_pid_file_with_content(1025, "io", io);
    procfs.create_pid_file_with_content(1025, "cgroup", cgroup);
    procfs.create_file_with_content("uptime", uptime);
    let reader = procfs.get_reader();

    let pidmap = reader.read_all_pids().expect("Failed to get all pids");

    assert_eq!(pidmap[&1024].stat.comm, Some("bash".to_string()));
    assert_eq!(
        pidmap[&1025].cgroup,
        "/user.slice/user-119756.slice/session-3.scope".to_string()
    );
}
