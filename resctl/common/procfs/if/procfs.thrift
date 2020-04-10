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

struct CpuStat {
  1: optional i64 user_usec,
  2: optional i64 nice_usec,
  3: optional i64 system_usec,
  4: optional i64 idle_usec,
  5: optional i64 iowait_usec,
  6: optional i64 irq_usec,
  7: optional i64 softirq_usec,
  8: optional i64 stolen_usec,
  9: optional i64 guest_usec,
  10: optional i64 guest_nice_usec,
}

struct Stat {
  1: optional CpuStat total_cpu,
  2: optional list<CpuStat> cpus,
  3: optional i64 total_interrupt_count,
  4: optional i64 context_switches,
  5: optional i64 boot_time_epoch_secs,
  6: optional i64 total_processes,
  7: optional i32 running_processes,
  8: optional i32 blocked_processes,
}

// In kilobytes unless specified otherwise
struct MemInfo {
  1: optional i64 total,
  2: optional i64 free,
  3: optional i64 available,
  4: optional i64 buffers,
  5: optional i64 cached,
  6: optional i64 swap_cached,
  7: optional i64 active,
  8: optional i64 inactive,
  9: optional i64 active_anon,
  10: optional i64 inactive_anon,
  11: optional i64 active_file,
  12: optional i64 inactive_file,
  13: optional i64 unevictable,
  14: optional i64 mlocked,
  15: optional i64 swap_total,
  16: optional i64 swap_free,
  17: optional i64 dirty,
  18: optional i64 writeback,
  19: optional i64 anon_pages,
  20: optional i64 mapped,
  21: optional i64 shmem,
  22: optional i64 kreclaimable,
  23: optional i64 slab,
  24: optional i64 slab_reclaimable,
  25: optional i64 slab_unreclaimable,
  26: optional i64 kernel_stack,
  27: optional i64 page_tables,
  28: optional i64 anon_huge_pages,
  29: optional i64 shmem_huge_pages,
  30: optional i64 file_huge_pages,
  // This is in number of pages, not kilobytes
  31: optional i64 total_huge_pages,
  // This is in number of pages, not kilobytes
  32: optional i64 free_huge_pages,
  33: optional i64 huge_page_size,
}

struct VmStat {
  1: optional i64 pgpgin,
  2: optional i64 pgpgout,
  3: optional i64 pswpin,
  4: optional i64 pswpout,
  5: optional i64 pgsteal_kswapd,
  6: optional i64 pgsteal_direct,
  7: optional i64 pgscan_kswapd,
  8: optional i64 pgscan_direct,
  9: optional i64 oom_kill,
}

enum PidState {
  RUNNING = 0,
  SLEEPING = 1,
  DISK_SLEEP = 2,
  STOPPED = 3,
  TRACING_STOPPED = 4,
  ZOMBIE = 5,
  DEAD = 6,
  IDLE = 7,
  PARKED = 8,
}

struct PidStat {
  1: optional i32 pid,
  2: optional string comm,
  3: optional PidState state,
  4: optional i32 ppid,
  5: optional i32 pgrp,
  6: optional i32 session,
  7: optional i64 minflt,
  8: optional i64 majflt,
  9: optional i64 user_usecs,
  10: optional i64 system_usecs,
  11: optional i64 num_threads,
  12: optional i64 running_secs,
  13: optional i64 rss_bytes,
  14: optional i32 processor,
}

struct PidIo {
  1: optional i64 rbytes,
  2: optional i64 wbytes,
}

struct PidInfo {
  1: PidStat stat,
  2: PidIo io,
  3: string cgroup,
}

typedef map<i32, PidInfo> PidMap
