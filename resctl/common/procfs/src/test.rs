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
use std::os::unix::fs::symlink;
use std::path::Path;

use tempfile::TempDir;

use crate::types::*;
use crate::NetReader;
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

    fn create_dir<P: AsRef<Path>>(&self, p: P) {
        let path = self.path().join(p);
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|_| panic!("Failed to create dir {}", path.display()))
    }

    fn create_file_with_content_full_path<P: AsRef<Path>>(&self, path: P, content: &[u8]) {
        let mut file = File::create(&path)
            .unwrap_or_else(|_| panic!("Failed to create {}", path.as_ref().display()));
        file.write_all(content)
            .unwrap_or_else(|_| panic!("Failed to write to {}", path.as_ref().display()));
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

    fn create_pid_file_with_link<P: AsRef<Path>>(&self, pid: u32, src: P, dst: P) -> String {
        self.create_pid_file_with_content(pid, src.as_ref(), b"");
        let pid_dir = self.path().join(pid.to_string());
        let src_path = pid_dir.join(src);
        let dst_path = pid_dir.join(dst);
        symlink(&src_path, &dst_path).expect("Fail to create pid symlink");
        src_path.to_string_lossy().into_owned()
    }

    fn get_net_reader(&self) -> NetReader {
        let iface_dir = self.path().join("iface");
        if !iface_dir.exists() {
            std::fs::create_dir(&iface_dir).expect("Failed to create iface dir");
        }
        NetReader::new_with_custom_path(iface_dir, self.path().to_path_buf())
            .expect("Fail to construct Net Reader")
    }

    fn create_net_stat_file_with_content<P: AsRef<Path>>(
        &self,
        interface: &str,
        p: P,
        content: usize,
    ) {
        let interface_dir = self.path().join(interface);
        if !interface_dir.exists() {
            std::fs::create_dir(&interface_dir).expect("Failed to create interface dir");
        }
        let iface_dir = self.path().join("iface");
        if !iface_dir.exists() {
            std::fs::create_dir(&iface_dir).expect("Failed to create iface dir");
        }
        let iface_link = iface_dir.join(interface);
        if !iface_link.exists() {
            symlink(&interface_dir, &iface_link).unwrap_or_else(|e| {
                panic!(
                    "Fail to create symlink {} -> {}: {}",
                    interface_dir.to_string_lossy(),
                    iface_link.to_string_lossy(),
                    e
                )
            });
        }
        let interface_dir = interface_dir.join("statistics");
        if !interface_dir.exists() {
            std::fs::create_dir(&interface_dir).expect("Failed to create statistics dir");
        }
        let path = interface_dir.join(p);
        self.create_file_with_content_full_path(path, content.to_string().as_bytes());
    }
}

#[test]
fn test_kernel_version() {
    let version = b"1.2.3";
    let procfs = TestProcfs::new();
    procfs.create_dir("sys/kernel");
    procfs.create_file_with_content("sys/kernel/osrelease", version);
    let reader = procfs.get_reader();
    let kernel_version = reader
        .read_kernel_version()
        .expect("Fail to read kernel version");
    assert_eq!(kernel_version, "1.2.3");
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
    assert_eq!(meminfo.huge_page_size, Some(2048 * 1024));
    assert_eq!(meminfo.cma_total, Some(0));
    assert_eq!(meminfo.cma_free, Some(0));
    assert_eq!(meminfo.vmalloc_total, Some(34_359_738_367 * 1024));
    assert_eq!(meminfo.vmalloc_used, Some(229_472 * 1024));
    assert_eq!(meminfo.vmalloc_chunk, Some(0));
    assert_eq!(meminfo.direct_map_4k, Some(19_445_616 * 1024));
    assert_eq!(meminfo.direct_map_2m, Some(40_323_072 * 1024));
    assert_eq!(meminfo.direct_map_1g, Some(2_097_152 * 1024));
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

    assert_eq!(vmstat.pgpgin, Some(5_245_063_123));
    assert_eq!(vmstat.pgpgout, Some(13_772_335_013));
    assert_eq!(vmstat.pswpin, Some(2_090_956));
    assert_eq!(vmstat.pswpout, Some(3_637_759));
    assert_eq!(vmstat.pgsteal_kswapd, Some(1_709_049_230));
    assert_eq!(vmstat.pgsteal_direct, Some(5_652_651));
    assert_eq!(vmstat.pgscan_kswapd, Some(1_743_683_511));
    assert_eq!(vmstat.pgscan_direct, Some(5_877_901));
    assert_eq!(vmstat.oom_kill, Some(0));
}

#[test]
fn test_disk_stat() {
    let diskstats = b"   1       0 ram0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       1 ram1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       2 ram2 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       3 ram3 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       4 ram4 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       5 ram5 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       6 ram6 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       7 ram7 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       8 ram8 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1       9 ram9 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1      10 ram10 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1      11 ram11 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1      12 ram12 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1      13 ram13 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1      14 ram14 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    1      15 ram15 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
  253       0 vda 187110061 6006969 23225661674 128112391 136557913 12023946 28151760010 615065070 0 107730702 623152538 1 2 3 4
  253       1 vda1 15333 522 288946 4125 1707 2227 253642 3073 0 5343 3060 0 0 0 0
  253       2 vda2 1183986 94095 10301816 266679 2457101 1248583 29645480 3253603 0 1556514 2531673 0 0 0 0
  253       3 vda3 185910515 5912352 23215062392 127841533 132254952 10773136 28121859920 611595170 0 106665419 620613687 0 0 0 0
    7       0 loop0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       1 loop1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       2 loop2 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       3 loop3 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       4 loop4 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       5 loop5 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       6 loop6 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       7 loop7 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       8 loop8 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7       9 loop9 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      10 loop10 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      11 loop11 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      12 loop12 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      13 loop13 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      14 loop14 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      15 loop15 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      16 loop16 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      17 loop17 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      18 loop18 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      19 loop19 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      20 loop20 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      21 loop21 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      22 loop22 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      23 loop23 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      24 loop24 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      25 loop25 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      26 loop26 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      27 loop27 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      28 loop28 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      29 loop29 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      30 loop30 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      31 loop31 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      32 loop32 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      33 loop33 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      34 loop34 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      35 loop35 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      36 loop36 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      37 loop37 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      38 loop38 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      39 loop39 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      40 loop40 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      41 loop41 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      42 loop42 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      43 loop43 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      44 loop44 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      45 loop45 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      46 loop46 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      47 loop47 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      48 loop48 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      49 loop49 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      50 loop50 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      51 loop51 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      52 loop52 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      53 loop53 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      54 loop54 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      55 loop55 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      56 loop56 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      57 loop57 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      58 loop58 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      59 loop59 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      60 loop60 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      61 loop61 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      62 loop62 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      63 loop63 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      64 loop64 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      65 loop65 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      66 loop66 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      67 loop67 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      68 loop68 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      69 loop69 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      70 loop70 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      71 loop71 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      72 loop72 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      73 loop73 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      74 loop74 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      75 loop75 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      76 loop76 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      77 loop77 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      78 loop78 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      79 loop79 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      80 loop80 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      81 loop81 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      82 loop82 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      83 loop83 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      84 loop84 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      85 loop85 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      86 loop86 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      87 loop87 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      88 loop88 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      89 loop89 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      90 loop90 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      91 loop91 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      92 loop92 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      93 loop93 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      94 loop94 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      95 loop95 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      96 loop96 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      97 loop97 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      98 loop98 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7      99 loop99 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     100 loop100 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     101 loop101 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     102 loop102 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     103 loop103 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     104 loop104 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     105 loop105 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     106 loop106 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     107 loop107 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     108 loop108 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     109 loop109 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     110 loop110 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     111 loop111 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     112 loop112 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     113 loop113 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     114 loop114 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     115 loop115 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     116 loop116 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     117 loop117 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     118 loop118 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     119 loop119 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     120 loop120 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     121 loop121 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     122 loop122 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     123 loop123 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     124 loop124 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     125 loop125 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     126 loop126 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    7     127 loop127 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0";

    let procfs = TestProcfs::new();
    procfs.create_file_with_content("diskstats", diskstats);
    let reader = procfs.get_reader();
    let diskmap = reader
        .read_disk_stats()
        .expect("Failed to read diskstats file");
    let vda_stat = diskmap.get("vda").expect("Fail to get vda");
    assert_eq!(vda_stat.name, Some("vda".into()));
    assert_eq!(vda_stat.read_completed, Some(187_110_061));
    assert_eq!(vda_stat.read_merged, Some(6_006_969));
    assert_eq!(vda_stat.read_sectors, Some(23_225_661_674));
    assert_eq!(vda_stat.time_spend_read_ms, Some(128_112_391));
    assert_eq!(vda_stat.write_completed, Some(136_557_913));
    assert_eq!(vda_stat.write_merged, Some(12_023_946));
    assert_eq!(vda_stat.write_sectors, Some(28_151_760_010));
    assert_eq!(vda_stat.time_spend_write_ms, Some(615_065_070));
    assert_eq!(vda_stat.discard_completed, Some(1));
    assert_eq!(vda_stat.discard_merged, Some(2));
    assert_eq!(vda_stat.discard_sectors, Some(3));
    assert_eq!(vda_stat.time_spend_discard_ms, Some(4));

    let vda_stat = diskmap.get("vda1").expect("Fail to get vda1");
    assert_eq!(vda_stat.name, Some("vda1".into()));
    assert_eq!(vda_stat.read_completed, Some(15333));
    assert_eq!(vda_stat.read_merged, Some(522));
    assert_eq!(vda_stat.read_sectors, Some(288_946));
    assert_eq!(vda_stat.time_spend_read_ms, Some(4125));
    assert_eq!(vda_stat.write_completed, Some(1707));
    assert_eq!(vda_stat.write_merged, Some(2227));
    assert_eq!(vda_stat.write_sectors, Some(253_642));
    assert_eq!(vda_stat.time_spend_write_ms, Some(3073));
    assert_eq!(vda_stat.discard_completed, Some(0));
    assert_eq!(vda_stat.discard_merged, Some(0));
    assert_eq!(vda_stat.discard_sectors, Some(0));
    assert_eq!(vda_stat.time_spend_discard_ms, Some(0));

    let vda_stat = diskmap.get("vda2").expect("Fail to get vda2");
    assert_eq!(vda_stat.name, Some("vda2".into()));
    assert_eq!(vda_stat.read_completed, Some(1_183_986));
    assert_eq!(vda_stat.read_merged, Some(94095));
    assert_eq!(vda_stat.read_sectors, Some(10_301_816));
    assert_eq!(vda_stat.time_spend_read_ms, Some(266_679));
    assert_eq!(vda_stat.write_completed, Some(2_457_101));
    assert_eq!(vda_stat.write_merged, Some(1_248_583));
    assert_eq!(vda_stat.write_sectors, Some(29_645_480));
    assert_eq!(vda_stat.time_spend_write_ms, Some(3_253_603));
    assert_eq!(vda_stat.discard_completed, Some(0));
    assert_eq!(vda_stat.discard_merged, Some(0));
    assert_eq!(vda_stat.discard_sectors, Some(0));
    assert_eq!(vda_stat.time_spend_discard_ms, Some(0));

    let vda_stat = diskmap.get("vda3").expect("Fail to get vda3");
    assert_eq!(vda_stat.name, Some("vda3".into()));
    assert_eq!(vda_stat.read_completed, Some(185_910_515));
    assert_eq!(vda_stat.read_merged, Some(5_912_352));
    assert_eq!(vda_stat.read_sectors, Some(23_215_062_392));
    assert_eq!(vda_stat.time_spend_read_ms, Some(127_841_533));
    assert_eq!(vda_stat.write_completed, Some(132_254_952));
    assert_eq!(vda_stat.write_merged, Some(10_773_136));
    assert_eq!(vda_stat.write_sectors, Some(28_121_859_920));
    assert_eq!(vda_stat.time_spend_write_ms, Some(611_595_170));
    assert_eq!(vda_stat.discard_completed, Some(0));
    assert_eq!(vda_stat.discard_merged, Some(0));
    assert_eq!(vda_stat.discard_sectors, Some(0));
    assert_eq!(vda_stat.time_spend_discard_ms, Some(0));
}

#[test]
fn test_pid_stat() {
    let uptime = b"1631826.55 37530838.66";
    let stat = b"74718 (((bash process)) D 44786 74718 74718 34820 3561868 4194304 31346884 614468259 3 23315 14474 10887 1967513 339861 20 0 1 0 102803 224440320 12725 18446744073709551615 93972706258944 93972707333076 140732465518320 0 0 0 65536 3670020 1266777851 0 0 0 17 12 0 0 7 0 0 93972709432552 93972709479876 93972709523456 140732465525073 140732465525079 140732465525079 140732465528814 0";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(74718, "stat", stat);
    procfs.create_file_with_content("uptime", uptime);
    let reader = procfs.get_reader();
    let pidstat = reader
        .read_pid_stat(74718)
        .expect("Failed to read pid stat file");

    assert_eq!(pidstat.pid, Some(74718));
    assert_eq!(pidstat.comm, Some("((bash process)".to_string()));
    assert_eq!(pidstat.state, Some(PidState::UninterruptibleSleep));
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
fn test_pid_mem() {
    let status = b"Name:	below
Umask:	0022
State:	S (sleeping)
Tgid:	93041
Ngid:	0
Pid:	93041
PPid:	1
TracerPid:	0
Uid:	0	0	0	0
Gid:	0	0	0	0
FDSize:	256
Groups:
NStgid:	93041
NSpid:	93041
NSpgid:	93041
NSsid:	93041
VmPeak:	 1381848 kB
VmSize:	 1381532 kB
VmLck:	       4 kB
VmPin:	    6240 kB
VmHWM:	  128092 kB
VmRSS:	  124404 kB
RssAnon:	   99284 kB
RssFile:	   25120 kB
RssShmem:	      12 kB
VmData:	 1236636 kB
VmStk:	     132 kB
VmExe:	   60652 kB
VmLib:	    9096 kB
VmPTE:	    1840 kB
VmSwap:	    8812 kB
HugetlbPages:	      13 kB
CoreDumping:	0
THP_enabled:	1
Threads:	147
SigQ:	5/228815
SigPnd:	0000000000000000
ShdPnd:	0000000000000000
SigBlk:	0000000000000000
SigIgn:	0000000000000000
SigCgt:	00000001840054ec
CapInh:	0000000000000000
CapPrm:	0000003fffffffff
CapEff:	0000003fffffffff
CapBnd:	0000003fffffffff
CapAmb:	0000000000000000
NoNewPrivs:	0
Seccomp:	0
Speculation_Store_Bypass:	vulnerable
Cpus_allowed:	ffffff
Cpus_allowed_list:	0-23
Mems_allowed:	01
Mems_allowed_list:	0
voluntary_ctxt_switches:	2144888
nonvoluntary_ctxt_switches:	37733";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(93041, "status", status);
    let reader = procfs.get_reader();
    let pidmem = reader
        .read_pid_mem(93041)
        .expect("Failed to read pid status file");

    assert_eq!(pidmem.vm_size, Some(1_381_532 * 1024));
    assert_eq!(pidmem.lock, Some(4 * 1024));
    assert_eq!(pidmem.pin, Some(6240 * 1024));
    assert_eq!(pidmem.anon, Some(99_284 * 1024));
    assert_eq!(pidmem.file, Some(25_120 * 1024));
    assert_eq!(pidmem.shmem, Some(12 * 1024));
    assert_eq!(pidmem.pte, Some(1840 * 1024));
    assert_eq!(pidmem.swap, Some(8812 * 1024));
    assert_eq!(pidmem.huge_tlb, Some(13 * 1024));
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
fn test_pid_cgroupv2() {
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
fn test_pid_cgroupv1() {
    let cgroup = b"11:pids:/init.scope
10:perf_event:/
9:hugetlb:/
8:cpu,cpuacct:/init.scope
7:blkio:/init.scope
6:freezer:/
5:cpuset:/
4:memory:/init.scope
3:devices:/init.scope
2:net_cls,net_prio:/
1:name=systemd:/init.scope";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(1024, "cgroup", cgroup);
    let reader = procfs.get_reader();
    let cgroup = reader
        .read_pid_cgroup(1024)
        .expect("Failed to read pid cgroup file");

    assert_eq!(cgroup, "/init.scope");
}

#[test]
fn test_pid_cmdline() {
    // procfs cmdline format is nul bytes to separate with a trailing nul byte
    let cmdline = b"one\0--long-flag\0-f\0";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(123, "cmdline", cmdline);
    let mut reader = procfs.get_reader();
    let cmdline = reader
        .read_pid_cmdline(123)
        .expect("Failed to read pid cmdline file");

    assert_eq!(
        cmdline.expect("missing cmdline"),
        vec!["one", "--long-flag", "-f"]
    );
}

#[test]
fn test_pid_cmdline_loop() {
    // procfs cmdline format is nul bytes to separate with a trailing nul byte
    let cmdline = b"one\0--long-flag\0-f\0";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(123, "cmdline", cmdline);
    let mut reader = procfs.get_reader();
    for _ in 0..10000 {
        assert_eq!(
            reader
                .read_pid_cmdline(123)
                .expect("Failed to read pid cmdline file")
                .expect("missing cmdline"),
            vec!["one", "--long-flag", "-f"]
        );
    }
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

    let status = b"Name:	below
Umask:	0022
State:	S (sleeping)
Tgid:	93041
Ngid:	0
Pid:	93041
PPid:	1
TracerPid:	0
Uid:	0	0	0	0
Gid:	0	0	0	0
FDSize:	256
Groups:
NStgid:	93041
NSpid:	93041
NSpgid:	93041
NSsid:	93041
VmPeak:	 1381848 kB
VmSize:	 1381532 kB
VmLck:	       4 kB
VmPin:	    6240 kB
VmHWM:	  128092 kB
VmRSS:	  124404 kB
RssAnon:	   99284 kB
RssFile:	   25120 kB
RssShmem:	      12 kB
VmData:	 1236636 kB
VmStk:	     132 kB
VmExe:	   60652 kB
VmLib:	    9096 kB
VmPTE:	    1840 kB
VmSwap:	    8812 kB
HugetlbPages:	      13 kB
CoreDumping:	0
THP_enabled:	1
Threads:	147
SigQ:	5/228815
SigPnd:	0000000000000000
ShdPnd:	0000000000000000
SigBlk:	0000000000000000
SigIgn:	0000000000000000
SigCgt:	00000001840054ec
CapInh:	0000000000000000
CapPrm:	0000003fffffffff
CapEff:	0000003fffffffff
CapBnd:	0000003fffffffff
CapAmb:	0000000000000000
NoNewPrivs:	0
Seccomp:	0
Speculation_Store_Bypass:	vulnerable
Cpus_allowed:	ffffff
Cpus_allowed_list:	0-23
Mems_allowed:	01
Mems_allowed_list:	0
voluntary_ctxt_switches:	2144888
nonvoluntary_ctxt_switches:	37733";

    // procfs cmdline format is nul bytes to separate with a trailing nul byte
    let cmdline = b"one\0two\0three\0";

    let procfs = TestProcfs::new();
    procfs.create_pid_file_with_content(1024, "stat", stat);
    procfs.create_pid_file_with_content(1024, "io", io);
    procfs.create_pid_file_with_content(1024, "status", status);
    procfs.create_pid_file_with_content(1024, "cgroup", cgroup);
    procfs.create_pid_file_with_content(1024, "cmdline", cmdline);
    procfs.create_pid_file_with_content(1025, "stat", stat);
    procfs.create_pid_file_with_content(1025, "status", status);
    procfs.create_pid_file_with_content(1025, "io", io);
    procfs.create_pid_file_with_content(1025, "cgroup", cgroup);
    procfs.create_pid_file_with_content(1025, "cmdline", cmdline);
    procfs.create_file_with_content("uptime", uptime);
    let mut reader = procfs.get_reader();

    let pidmap = reader.read_all_pids().expect("Failed to get all pids");

    assert_eq!(pidmap[&1024].stat.comm, Some("bash".to_string()));
    assert_eq!(
        pidmap[&1025].cgroup,
        "/user.slice/user-119756.slice/session-3.scope".to_string()
    );
    assert_eq!(
        pidmap[&1025]
            .cmdline_vec
            .as_ref()
            .expect("cmdline missing")
            .join(" "),
        "one two three"
    );
}

fn write_net_map(netsysfs: &TestProcfs) {
    for interface in &["enp1s0", "enp2s0"] {
        netsysfs.create_net_stat_file_with_content(interface, "collisions", 1);
        netsysfs.create_net_stat_file_with_content(interface, "multicast", 2);
        netsysfs.create_net_stat_file_with_content(interface, "rx_bytes", 2_087_593_014_826);
        netsysfs.create_net_stat_file_with_content(interface, "rx_compressed", 4);
        netsysfs.create_net_stat_file_with_content(interface, "rx_crc_errors", 5);
        netsysfs.create_net_stat_file_with_content(interface, "rx_dropped", 6);
        netsysfs.create_net_stat_file_with_content(interface, "rx_errors", 7);
        netsysfs.create_net_stat_file_with_content(interface, "rx_fifo_errors", 8);
        netsysfs.create_net_stat_file_with_content(interface, "rx_frame_errors", 9);
        netsysfs.create_net_stat_file_with_content(interface, "rx_length_errors", 10);
        netsysfs.create_net_stat_file_with_content(interface, "rx_missed_errors", 11);
        netsysfs.create_net_stat_file_with_content(interface, "rx_nohandler", 12);
        netsysfs.create_net_stat_file_with_content(interface, "rx_over_errors", 13);
        netsysfs.create_net_stat_file_with_content(interface, "rx_packets", 14);
        netsysfs.create_net_stat_file_with_content(interface, "tx_aborted_errors", 15);
        netsysfs.create_net_stat_file_with_content(interface, "tx_bytes", 1_401_221_862_430);
        netsysfs.create_net_stat_file_with_content(interface, "tx_carrier_errors", 17);
        netsysfs.create_net_stat_file_with_content(interface, "tx_compressed", 18);
        netsysfs.create_net_stat_file_with_content(interface, "tx_dropped", 19);
        netsysfs.create_net_stat_file_with_content(interface, "tx_errors", 20);
        netsysfs.create_net_stat_file_with_content(interface, "tx_fifo_errors", 21);
        netsysfs.create_net_stat_file_with_content(interface, "tx_heartbeat_errors", 22);
        netsysfs.create_net_stat_file_with_content(interface, "tx_packets", 23);
        netsysfs.create_net_stat_file_with_content(interface, "tx_window_errors", 24);
    }
}

fn write_net_snmp(netsysfs: &TestProcfs) {
    let snmp = b"Ip: Forwarding DefaultTTL InReceives InHdrErrors InAddrErrors ForwDatagrams InUnknownProtos InDiscards InDelivers OutRequests OutDiscards OutNoRoutes ReasmTimeout ReasmReqds ReasmOKs ReasmFails FragOKs FragFails FragCreates
Ip: 2 96 630036507 0 0 0 0 0 629963239 630016831 0 186411 0 0 0 0 0 0 0
Icmp: InMsgs InErrors InCsumErrors InDestUnreachs InTimeExcds InParmProbs InSrcQuenchs InRedirects InEchos InEchoReps InTimestamps InTimestampReps InAddrMasks InAddrMaskReps OutMsgs OutErrors OutDestUnreachs OutTimeExcds OutParmProbs OutSrcQuenchs OutRedirects OutEchos OutEchoReps OutTimestamps OutTimestampReps OutAddrMasks OutAddrMaskReps
Icmp: 31 31 0 31 0 0 0 0 0 0 0 0 0 0 31 0 31 0 0 0 0 0 0 0 0 0 0
IcmpMsg: InType3 OutType3
IcmpMsg: 31 31
Tcp: RtoAlgorithm RtoMin RtoMax MaxConn ActiveOpens PassiveOpens AttemptFails EstabResets CurrEstab InSegs OutSegs RetransSegs InErrs OutRsts InCsumErrors
Tcp: 1 200 120000 -1 54858563 40737307 4734320 5454512 820 2041813239 3258286962 2341081 955 16078320 39
Udp: InDatagrams NoPorts InErrors OutDatagrams RcvbufErrors SndbufErrors InCsumErrors IgnoredMulti
Udp: 51051 31 84 116484 84 0 0 19384
UdpLite: InDatagrams NoPorts InErrors OutDatagrams RcvbufErrors SndbufErrors InCsumErrors IgnoredMulti
UdpLite: 0 0 0 0 0 0 0 0";
    netsysfs.create_file_with_content("snmp", snmp);
}

fn write_net_snmp6(netsysfs: &TestProcfs) {
    let snmp6 = b"Ip6InReceives                   	1594971243
Ip6InHdrErrors                  	17032537
Ip6InTooBigErrors               	0
Ip6InNoRoutes                   	95
Ip6InAddrErrors                 	1333
Ip6InUnknownProtos              	0
Ip6InTruncatedPkts              	0
Ip6InDiscards                   	0
Ip6InDelivers                   	1500587362
Ip6OutForwDatagrams             	0
Ip6OutRequests                  	1495881793
Ip6OutDiscards                  	0
Ip6OutNoRoutes                  	626
Ip6ReasmTimeout                 	0
Ip6ReasmReqds                   	0
Ip6ReasmOKs                     	0
Ip6ReasmFails                   	0
Ip6FragOKs                      	0
Ip6FragFails                    	0
Ip6FragCreates                  	0
Ip6InMcastPkts                  	155122808
Ip6OutMcastPkts                 	1591270
Ip6InOctets                     	4493023649370
Ip6OutOctets                    	3622952718119
Ip6InMcastOctets                	19936651296
Ip6OutMcastOctets               	206033131
Ip6InBcastOctets                	0
Ip6OutBcastOctets               	0
Ip6InNoECTPkts                  	1594929854
Ip6InECT1Pkts                   	0
Ip6InECT0Pkts                   	207855
Ip6InCEPkts                     	0
Icmp6InMsgs                     	8121791
Icmp6InErrors                   	462
Icmp6OutMsgs                    	7763670
Icmp6OutErrors                  	0
Icmp6InCsumErrors               	0
Icmp6InDestUnreachs             	1251
Icmp6InPktTooBigs               	156
Icmp6InTimeExcds                	2
Icmp6InParmProblems             	0
Icmp6InEchos                    	24443
Icmp6InEchoReplies              	31691
Icmp6InGroupMembQueries         	0
Icmp6InGroupMembResponses       	0
Icmp6InGroupMembReductions      	0
Icmp6InRouterSolicits           	0
Icmp6InRouterAdvertisements     	549895
Icmp6InNeighborSolicits         	3742942
Icmp6InNeighborAdvertisements   	3771411
Icmp6InRedirects                	0
Icmp6InMLDv2Reports             	0
Icmp6OutDestUnreachs            	1266
Icmp6OutPktTooBigs              	0
Icmp6OutTimeExcds               	0
Icmp6OutParmProblems            	0
Icmp6OutEchos                   	31691
Icmp6OutEchoReplies             	24443
Icmp6OutGroupMembQueries        	0
Icmp6OutGroupMembResponses      	0
Icmp6OutGroupMembReductions     	0
Icmp6OutRouterSolicits          	1
Icmp6OutRouterAdvertisements    	0
Icmp6OutNeighborSolicits        	3963540
Icmp6OutNeighborAdvertisements  	3742708
Icmp6OutRedirects               	0
Icmp6OutMLDv2Reports            	21
Icmp6InType1                    	1251
Icmp6InType2                    	156
Icmp6InType3                    	2
Icmp6InType128                  	24443
Icmp6InType129                  	31691
Icmp6InType134                  	549895
Icmp6InType135                  	3742942
Icmp6InType136                  	3771411
Icmp6OutType1                   	1266
Icmp6OutType128                 	31691
Icmp6OutType129                 	24443
Icmp6OutType133                 	1
Icmp6OutType135                 	3963540
Icmp6OutType136                 	3742708
Icmp6OutType143                 	21
Udp6InDatagrams                 	159518170
Udp6NoPorts                     	47
Udp6InErrors                    	2163583
Udp6OutDatagrams                	3106145
Udp6RcvbufErrors                	2163583
Udp6SndbufErrors                	0
Udp6InCsumErrors                	0
Udp6IgnoredMulti                	0
UdpLite6InDatagrams             	0
UdpLite6NoPorts                 	0
UdpLite6InErrors                	0
UdpLite6OutDatagrams            	0
UdpLite6RcvbufErrors            	0
UdpLite6SndbufErrors            	0
UdpLite6InCsumErrors            	0";

    netsysfs.create_file_with_content("snmp6", snmp6);
}

fn write_net_netstat(netsysfs: &TestProcfs) {
    let netstat = b"TcpExt: SyncookiesSent SyncookiesRecv SyncookiesFailed EmbryonicRsts PruneCalled RcvPruned OfoPruned OutOfWindowIcmps LockDroppedIcmps ArpFilter TW TWRecycled TWKilled PAWSActive PAWSEstab DelayedACKs DelayedACKLocked DelayedACKLost ListenOverflows ListenDrops TCPHPHits TCPPureAcks TCPHPAcks TCPRenoRecovery TCPSackRecovery TCPSACKReneging TCPSACKReorder TCPRenoReorder TCPTSReorder TCPFullUndo TCPPartialUndo TCPDSACKUndo TCPLossUndo TCPLostRetransmit TCPRenoFailures TCPSackFailures TCPLossFailures TCPFastRetrans TCPSlowStartRetrans TCPTimeouts TCPLossProbes TCPLossProbeRecovery TCPRenoRecoveryFail TCPSackRecoveryFail TCPRcvCollapsed TCPBacklogCoalesce TCPDSACKOldSent TCPDSACKOfoSent TCPDSACKRecv TCPDSACKOfoRecv TCPAbortOnData TCPAbortOnClose TCPAbortOnMemory TCPAbortOnTimeout TCPAbortOnLinger TCPAbortFailed TCPMemoryPressures TCPMemoryPressuresChrono TCPSACKDiscard TCPDSACKIgnoredOld TCPDSACKIgnoredNoUndo TCPSpuriousRTOs TCPMD5NotFound TCPMD5Unexpected TCPMD5Failure TCPSackShifted TCPSackMerged TCPSackShiftFallback TCPBacklogDrop PFMemallocDrop TCPMinTTLDrop TCPDeferAcceptDrop IPReversePathFilter TCPTimeWaitOverflow TCPReqQFullDoCookies TCPReqQFullDrop TCPRetransFail TCPRcvCoalesce TCPOFOQueue TCPOFODrop TCPOFOMerge TCPChallengeACK TCPSYNChallenge TCPFastOpenActive TCPFastOpenActiveFail TCPFastOpenPassive TCPFastOpenPassiveFail TCPFastOpenListenOverflow TCPFastOpenCookieReqd TCPFastOpenBlackhole TCPSpuriousRtxHostQueues BusyPollRxPackets TCPAutoCorking TCPFromZeroWindowAdv TCPToZeroWindowAdv TCPWantZeroWindowAdv TCPSynRetrans TCPOrigDataSent TCPHystartTrainDetect TCPHystartTrainCwnd TCPHystartDelayDetect TCPHystartDelayCwnd TCPACKSkippedSynRecv TCPACKSkippedPAWS TCPACKSkippedSeq TCPACKSkippedFinWait2 TCPACKSkippedTimeWait TCPACKSkippedChallenge TCPWinProbe TCPKeepAlive TCPMTUPFail TCPMTUPSuccess TCPDelivered TCPDeliveredCE TCPAckCompressed TCPZeroWindowDrop TCPRcvQDrop TCPWqueueTooBig
TcpExt: 734 734 72186 207 32430 0 0 0 0 0 44799169 718071 0 0 477 13818919 85426 82837 36278 36278 648162608 229195403 644151467 5678 0 0 0 241 25 16 20 1 56568 56306 6714 0 9590 68973 1260322 424264 0 0 2979 0 0 10960020 0 0 0 0 4857068 1755828 0 129 0 0 0 0 0 0 0 1433 0 0 0 0 0 0 1 0 0 0 0 0 734 0 0 113000427 329930 0 49 2829 917 8328485 589326 0 0 0 0 965 956 0 26637 2478340 2478376 8121684 56007 2362573793 123903 4672843 604 50392 140 42 1981 0 36 53 17509 21875 0 136 2403045772 0 0 0 0 0
IpExt: InNoRoutes InTruncatedPkts InMcastPkts OutMcastPkts InBcastPkts OutBcastPkts InOctets OutOctets InMcastOctets OutMcastOctets InBcastOctets OutBcastOctets InCsumErrors InNoECTPkts InECT1Pkts InECT0Pkts InCEPkts ReasmOverlaps
IpExt: 0 0 72982 72982 26227 6841 3021953584043 3021942373821 11953543 11953543 12283455 1121095 0 630134902 0 0 0 0";

    netsysfs.create_file_with_content("netstat", netstat);
}

#[test]
fn test_read_net_stat() {
    let netsysfs = TestProcfs::new();
    write_net_snmp(&netsysfs);
    write_net_snmp6(&netsysfs);
    write_net_netstat(&netsysfs);
    write_net_map(&netsysfs);
    let netstat = netsysfs
        .get_net_reader()
        .read_netstat()
        .expect("Fail to get NetStat");
    verify_tcp(&netstat);
    verify_tcp_ext(&netstat);
    verify_ip(&netstat);
    verify_ip_ext(&netstat);
    verify_ip6(&netstat);
    verify_icmp(&netstat);
    verify_icmp6(&netstat);
    verify_udp(&netstat);
    verify_udp6(&netstat);
    verify_interfaces(&netstat);
}

fn verify_tcp(netstat: &NetStat) {
    let tcp = netstat.tcp.as_ref().expect("Fail to collect tcp stats");
    assert_eq!(tcp.active_opens, Some(54_858_563));
    assert_eq!(tcp.passive_opens, Some(40_737_307));
    assert_eq!(tcp.attempt_fails, Some(4_734_320));
    assert_eq!(tcp.estab_resets, Some(5_454_512));
    assert_eq!(tcp.curr_estab, Some(820));
    assert_eq!(tcp.in_segs, Some(2_041_813_239));
    assert_eq!(tcp.out_segs, Some(3_258_286_962));
    assert_eq!(tcp.retrans_segs, Some(2_341_081));
    assert_eq!(tcp.in_errs, Some(955));
    assert_eq!(tcp.out_rsts, Some(16_078_320));
    assert_eq!(tcp.in_csum_errors, Some(39));
}

fn verify_tcp_ext(netstat: &NetStat) {
    let tcp_ext = netstat
        .tcp_ext
        .as_ref()
        .expect("Fail to collect TcpExt stats");
    assert_eq!(tcp_ext.syncookies_sent, Some(734));
    assert_eq!(tcp_ext.syncookies_recv, Some(734));
    assert_eq!(tcp_ext.syncookies_failed, Some(72186));
    assert_eq!(tcp_ext.embryonic_rsts, Some(207));
    assert_eq!(tcp_ext.prune_called, Some(32430));
    assert_eq!(tcp_ext.tw, Some(44_799_169));
    assert_eq!(tcp_ext.paws_estab, Some(477));
    assert_eq!(tcp_ext.delayed_acks, Some(13_818_919));
    assert_eq!(tcp_ext.delayed_ack_locked, Some(85426));
    assert_eq!(tcp_ext.delayed_ack_lost, Some(82837));
    assert_eq!(tcp_ext.listen_overflows, Some(36278));
    assert_eq!(tcp_ext.listen_drops, Some(36278));
    assert_eq!(tcp_ext.tcp_hp_hits, Some(648_162_608));
    assert_eq!(tcp_ext.tcp_pure_acks, Some(229_195_403));
    assert_eq!(tcp_ext.tcp_hp_acks, Some(644_151_467));
    assert_eq!(tcp_ext.tcp_reno_recovery, Some(5678));
    assert_eq!(tcp_ext.tcp_reno_reorder, Some(241));
    assert_eq!(tcp_ext.tcp_ts_reorder, Some(25));
    assert_eq!(tcp_ext.tcp_full_undo, Some(16));
    assert_eq!(tcp_ext.tcp_partial_undo, Some(20));
    assert_eq!(tcp_ext.tcp_dsack_undo, Some(1));
    assert_eq!(tcp_ext.tcp_loss_undo, Some(56568));
    assert_eq!(tcp_ext.tcp_lost_retransmit, Some(56306));
    assert_eq!(tcp_ext.tcp_reno_failures, Some(6714));
    assert_eq!(tcp_ext.tcp_loss_failures, Some(9590));
    assert_eq!(tcp_ext.tcp_fast_retrans, Some(68973));
    assert_eq!(tcp_ext.tcp_slow_start_retrans, Some(1_260_322));
    assert_eq!(tcp_ext.tcp_timeouts, Some(424_264));
}

fn verify_ip(netstat: &NetStat) {
    let ip = netstat.ip.as_ref().expect("Fail to collect ip stats");
    assert_eq!(ip.forwarding, Some(2));
    assert_eq!(ip.in_receives, Some(630_036_507));
    assert_eq!(ip.forw_datagrams, Some(0));
    assert_eq!(ip.in_discards, Some(0));
    assert_eq!(ip.in_delivers, Some(629_963_239));
    assert_eq!(ip.out_requests, Some(630_016_831));
    assert_eq!(ip.out_discards, Some(0));
    assert_eq!(ip.out_no_routes, Some(186_411));
}

fn verify_ip_ext(netstat: &NetStat) {
    let ip_ext = netstat
        .ip_ext
        .as_ref()
        .expect("Fail to collect IpExt stats");
    assert_eq!(ip_ext.in_mcast_pkts, Some(72982));
    assert_eq!(ip_ext.out_mcast_pkts, Some(72982));
    assert_eq!(ip_ext.in_bcast_pkts, Some(26227));
    assert_eq!(ip_ext.out_bcast_pkts, Some(6841));
    assert_eq!(ip_ext.in_octets, Some(3_021_953_584_043));
    assert_eq!(ip_ext.out_octets, Some(3_021_942_373_821));
    assert_eq!(ip_ext.in_mcast_octets, Some(11_953_543));
    assert_eq!(ip_ext.out_mcast_octets, Some(11_953_543));
    assert_eq!(ip_ext.in_bcast_octets, Some(12_283_455));
    assert_eq!(ip_ext.out_bcast_octets, Some(1_121_095));
    assert_eq!(ip_ext.in_no_ect_pkts, Some(630_134_902));
}

fn verify_ip6(netstat: &NetStat) {
    let ip6 = netstat.ip6.as_ref().expect("Fail to collect ip6 stats");
    assert_eq!(ip6.in_receives, Some(1_594_971_243));
    assert_eq!(ip6.in_hdr_errors, Some(17_032_537));
    assert_eq!(ip6.in_no_routes, Some(95));
    assert_eq!(ip6.in_addr_errors, Some(1333));
    assert_eq!(ip6.in_discards, Some(0));
    assert_eq!(ip6.in_delivers, Some(1_500_587_362));
    assert_eq!(ip6.out_forw_datagrams, Some(0));
    assert_eq!(ip6.out_requests, Some(1_495_881_793));
    assert_eq!(ip6.out_no_routes, Some(626));
    assert_eq!(ip6.in_mcast_pkts, Some(155_122_808));
    assert_eq!(ip6.out_mcast_pkts, Some(1_591_270));
    assert_eq!(ip6.in_octets, Some(4_493_023_649_370));
    assert_eq!(ip6.out_octets, Some(3_622_952_718_119));
    assert_eq!(ip6.in_mcast_octets, Some(19_936_651_296));
    assert_eq!(ip6.out_mcast_octets, Some(206_033_131));
    assert_eq!(ip6.in_bcast_octets, Some(0));
    assert_eq!(ip6.out_bcast_octets, Some(0));
}

fn verify_icmp(netstat: &NetStat) {
    let icmp = netstat.icmp.as_ref().expect("Fail to collect icmp stats");
    assert_eq!(icmp.in_msgs, Some(31));
    assert_eq!(icmp.in_errors, Some(31));
    assert_eq!(icmp.in_dest_unreachs, Some(31));
    assert_eq!(icmp.out_msgs, Some(31));
    assert_eq!(icmp.out_errors, Some(0));
    assert_eq!(icmp.out_dest_unreachs, Some(31));
}

fn verify_icmp6(netstat: &NetStat) {
    let icmp6 = netstat.icmp6.as_ref().expect("Fail to collect icmp6 stats");
    assert_eq!(icmp6.in_msgs, Some(812_1791));
    assert_eq!(icmp6.in_errors, Some(462));
    assert_eq!(icmp6.out_msgs, Some(7_763_670));
    assert_eq!(icmp6.out_errors, Some(0));
    assert_eq!(icmp6.in_dest_unreachs, Some(1251));
    assert_eq!(icmp6.out_dest_unreachs, Some(1266));
}

fn verify_udp(netstat: &NetStat) {
    let udp = netstat.udp.as_ref().expect("Fail to collect udp stats");
    assert_eq!(udp.in_datagrams, Some(51051));
    assert_eq!(udp.no_ports, Some(31));
    assert_eq!(udp.in_errors, Some(84));
    assert_eq!(udp.out_datagrams, Some(116_484));
    assert_eq!(udp.rcvbuf_errors, Some(84));
    assert_eq!(udp.sndbuf_errors, Some(0));
    assert_eq!(udp.ignored_multi, Some(19384));
}

fn verify_udp6(netstat: &NetStat) {
    let udp6 = netstat.udp6.as_ref().expect("Fail to collect udp6 stats");
    assert_eq!(udp6.in_datagrams, Some(159_518_170));
    assert_eq!(udp6.no_ports, Some(47));
    assert_eq!(udp6.in_errors, Some(2_163_583));
    assert_eq!(udp6.out_datagrams, Some(3_106_145));
    assert_eq!(udp6.rcvbuf_errors, Some(2_163_583));
    assert_eq!(udp6.sndbuf_errors, Some(0));
    assert_eq!(udp6.in_csum_errors, Some(0));
    assert_eq!(udp6.ignored_multi, Some(0));
}

fn verify_interfaces(netstat: &NetStat) {
    let netmap = netstat
        .interfaces
        .as_ref()
        .expect("Fail to collect interfaces stats");
    for interface in &["enp1s0", "enp2s0"] {
        let netstat = netmap.get(*interface).expect("Fail to find interface");
        assert_eq!(netstat.collisions, Some(1));
        assert_eq!(netstat.multicast, Some(2));
        assert_eq!(netstat.rx_bytes, Some(2_087_593_014_826 as i64));
        assert_eq!(netstat.rx_compressed, Some(4));
        assert_eq!(netstat.rx_crc_errors, Some(5));
        assert_eq!(netstat.rx_dropped, Some(6));
        assert_eq!(netstat.rx_errors, Some(7));
        assert_eq!(netstat.rx_fifo_errors, Some(8));
        assert_eq!(netstat.rx_frame_errors, Some(9));
        assert_eq!(netstat.rx_length_errors, Some(10));
        assert_eq!(netstat.rx_missed_errors, Some(11));
        assert_eq!(netstat.rx_nohandler, Some(12));
        assert_eq!(netstat.rx_over_errors, Some(13));
        assert_eq!(netstat.rx_packets, Some(14));
        assert_eq!(netstat.tx_aborted_errors, Some(15));
        assert_eq!(netstat.tx_bytes, Some(1_401_221_862_430 as i64));
        assert_eq!(netstat.tx_carrier_errors, Some(17));
        assert_eq!(netstat.tx_compressed, Some(18));
        assert_eq!(netstat.tx_dropped, Some(19));
        assert_eq!(netstat.tx_errors, Some(20));
        assert_eq!(netstat.tx_fifo_errors, Some(21));
        assert_eq!(netstat.tx_heartbeat_errors, Some(22));
        assert_eq!(netstat.tx_packets, Some(23));
        assert_eq!(netstat.tx_window_errors, Some(24));
    }
}

#[test]
fn test_read_pid_exec() {
    let procfs = TestProcfs::new();
    let res = procfs.create_pid_file_with_link(1234, "exe_path", "exe");
    let reader = procfs.get_reader();
    let exe_path = reader
        .read_pid_exe_path(1234)
        .expect("Failed to read pid exe file");

    assert_eq!(exe_path, res);
}
