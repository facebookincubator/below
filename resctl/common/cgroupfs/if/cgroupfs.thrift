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

namespace cpp2 resctl.common.cgroupfs
namespace py3 resctl.common.cgroupfs

struct CpuStat {
  1: optional i64 usage_usec,
  2: optional i64 user_usec,
  3: optional i64 system_usec,
  4: optional i64 nr_periods,
  5: optional i64 nr_throttled,
  6: optional i64 throttled_usec,
}

struct IoStat {
  1: optional i64 rbytes,
  2: optional i64 wbytes,
  3: optional i64 rios,
  4: optional i64 wios,
  5: optional i64 dbytes,
  6: optional i64 dios,
}

struct MemoryStat {
  1: optional i64 anon,
  2: optional i64 file,
  3: optional i64 kernel_stack,
  4: optional i64 slab,
  5: optional i64 sock,
  6: optional i64 shmem,
  7: optional i64 file_mapped,
  8: optional i64 file_dirty,
  9: optional i64 file_writeback,
  10: optional i64 anon_thp,
  11: optional i64 inactive_anon,
  12: optional i64 active_anon,
  13: optional i64 inactive_file,
  14: optional i64 active_file,
  15: optional i64 unevictable,
  16: optional i64 slab_reclaimable,
  17: optional i64 slab_unreclaimable,
  18: optional i64 pgfault,
  19: optional i64 pgmajfault,
  20: optional i64 workingset_refault,
  21: optional i64 workingset_activate,
  22: optional i64 workingset_nodereclaim,
  23: optional i64 pgrefill,
  24: optional i64 pgscan,
  25: optional i64 pgsteal,
  26: optional i64 pgactivate,
  27: optional i64 pgdeactivate,
  28: optional i64 pglazyfree,
  29: optional i64 pglazyfreed,
  30: optional i64 thp_fault_alloc,
  31: optional i64 thp_collapse_alloc,
}

struct PressureMetrics {
  1: optional double avg10,
  2: optional double avg60,
  3: optional double avg300,
  4: optional i64 total,
}

struct CpuPressure {
  1: PressureMetrics some,
}

struct IoPressure {
  1: PressureMetrics some,
  2: PressureMetrics full,
}

struct MemoryPressure {
  1: PressureMetrics some,
  2: PressureMetrics full,
}

struct Pressure {
  1: CpuPressure cpu,
  2: IoPressure io,
  3: MemoryPressure memory,
}
