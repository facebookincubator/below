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

// The wire record the BPF program emits, one per cgroup. This is the single
// source of truth: the BPF program (cgroup_bpf.bpf.c) includes this header, and
// bindgen turns the same struct into the Rust type the reader decodes -- so the
// two never drift. (buck generates it with rust_bindgen_library; the
// open-source build generates it at compile time with bindgen in build.rs.)
//
// Fields use plain `unsigned long long` / `unsigned int` rather than the kernel
// u64/u32 typedefs so the header parses on its own: the BPF program includes it
// after vmlinux.h, bindgen parses it with no extra includes, and both see the
// same 64/32-bit layout.
//
// The ms_* fields are FINAL memory.stat values (bytes for size counters, raw
// counts for the pg*/workingset counters) as produced by the memcg kfuncs, i.e.
// exactly what the memory.stat file prints -- user space copies them verbatim.
// A field set to -1 (all bits set) marks a counter the running kernel does not
// track; user space maps that to None, matching memory.stat omitting the line.
// (This per-field sentinel replaces an earlier present-bitmask, which capped
// the count at 64 fields.)

#pragma once

// Bit index into cgroup_bpf_record.valid for each cgroup stat file (group) the
// BPF program tries to read. A set bit means BPF supplied that group for the
// cgroup; a clear bit tells user space to read the file instead. This enum is
// the single source of truth for the bit positions -- bindgen mirrors it to
// Rust so the two sides never drift -- and CGROUP_FILES_NUM is the number of
// groups. (A scalar `valid` + this enum is used instead of a kernel
// DECLARE_BITMAP: the bitmap macro is not in the BPF include set and the groups
// fit a u32 with room.)
enum cgroup_files {
  CGROUP_FILES_CPU_USAGE = 0,
  CGROUP_FILES_CPU_THROTTLE,
  CGROUP_FILES_MEM_CURRENT,
  CGROUP_FILES_MEM_STAT,
  CGROUP_FILES_CGROUP_STAT,
  CGROUP_FILES_MEM_EVENTS,
  CGROUP_FILES_MEM_LIMITS,
  CGROUP_FILES_NUM,
};

// Bit index into the `features` map value for each extra read the BPF program
// may do. Some memory.stat lines are printed only under a kernel build option
// or a cgroup2 mount option, even though the kernel keeps tracking their
// counters, so reading them always would report a line the file does not have.
// BPF reads such a counter only when user space sets its bit, having checked
// the option itself (see cgroup_bpf/coverage.rs). A clear bit leaves the -1
// sentinel and the field comes from the file.
enum cgroup_bpf_features {
  // memory.stat prints "hugetlb" only when the cgroup2 mount carries
  // memory_hugetlb_accounting.
  CGROUP_BPF_FEATURE_HUGETLB = 0,
  // memory.stat prints "zswap" and "zswapped" only with CONFIG_ZSWAP.
  CGROUP_BPF_FEATURE_ZSWAP,
  CGROUP_BPF_FEATURE_NUM,
};

// The memory.stat counters the program reads by index instead of by name,
// listed once so the program, the record and user space agree on the slots.
//
// The program cannot name these. The reclaim counters were vm_event_item until
// Linux 7.1 moved them to node_stat_item, and C allows one enumerator of a
// given name per file, so a header can spell them one way or the other but
// never both; NR_HUGETLB is simply newer than the pinned dump. Instead user
// space finds each one in the running kernel's BTF and puts its index in the
// `item_index` map, which works whichever enum holds it and needs no
// kernel-version test.
#define MEM_STAT_FIELDS_READ_BY_INDEX(X)       \
  X(NR_HUGETLB, ms_hugetlb)                    \
  X(PGREFILL, ms_pgrefill)                     \
  X(PGSCAN_KSWAPD, ms_pgscan_kswapd)           \
  X(PGSCAN_DIRECT, ms_pgscan_direct)           \
  X(PGSCAN_KHUGEPAGED, ms_pgscan_khugepaged)   \
  X(PGSCAN_PROACTIVE, ms_pgscan_proactive)     \
  X(PGSTEAL_KSWAPD, ms_pgsteal_kswapd)         \
  X(PGSTEAL_DIRECT, ms_pgsteal_direct)         \
  X(PGSTEAL_KHUGEPAGED, ms_pgsteal_khugepaged) \
  X(PGSTEAL_PROACTIVE, ms_pgsteal_proactive)

// Slot of each of those counters in the `item_index` map.
enum cgroup_bpf_item {
#define X(name, field) CGROUP_BPF_ITEM_##name,
  MEM_STAT_FIELDS_READ_BY_INDEX(X)
#undef X
      CGROUP_BPF_ITEM_NUM,
};

// Which kfunc reads one of them on this kernel. User space decides from the
// enum it found the counter in; ABSENT means it found neither, so the program
// leaves the -1 sentinel.
enum cgroup_bpf_item_source {
  CGROUP_BPF_ITEM_ABSENT = 0,
  CGROUP_BPF_ITEM_PAGE_STATE,
  CGROUP_BPF_ITEM_VM_EVENT,
};

// One entry of the `item_index` map: how to read that counter, and its index in
// the running kernel's enum.
struct cgroup_bpf_item_index {
  unsigned int source; // enum cgroup_bpf_item_source
  int index;
};

// X-macro lists of the memory.stat counters read the same way, so each field's
// struct slot (below) and its collection call (collect_memory_stat in
// cgroup_bpf.bpf.c) are written once and cannot drift. Each entry maps a record
// field to the kernel counter it reads:
//   PAGE_STATE -- X(field, enum_type, enum_item), read via
//                 bpf_mem_cgroup_page_state (node_stat_item / memcg_stat_item).
//   EVENT      -- X(field, enum_item), read via bpf_mem_cgroup_vm_events
//                 (vm_event_item).
// The enum type/item tokens are only used where the list is expanded into
// collection calls; the struct expansion ignores them, so this header still
// parses (for bindgen) without vmlinux.h. Counters that need special handling
// (pgscan/pgsteal aggregates + components, hugetlb) are declared and collected
// explicitly, not via these lists.
#define MEM_STAT_PAGE_STATE_FIELDS(X)                                      \
  X(ms_anon, node_stat_item, NR_ANON_MAPPED)                               \
  X(ms_file, node_stat_item, NR_FILE_PAGES)                                \
  X(ms_kernel, memcg_stat_item, MEMCG_KMEM)                                \
  X(ms_kernel_stack, node_stat_item, NR_KERNEL_STACK_KB)                   \
  X(ms_sock, memcg_stat_item, MEMCG_SOCK)                                  \
  X(ms_shmem, node_stat_item, NR_SHMEM)                                    \
  X(ms_zswap, memcg_stat_item, MEMCG_ZSWAP_B)                              \
  X(ms_zswapped, memcg_stat_item, MEMCG_ZSWAPPED)                          \
  X(ms_file_mapped, node_stat_item, NR_FILE_MAPPED)                        \
  X(ms_file_dirty, node_stat_item, NR_FILE_DIRTY)                          \
  X(ms_file_writeback, node_stat_item, NR_WRITEBACK)                       \
  X(ms_swapcached, node_stat_item, NR_SWAPCACHE)                           \
  X(ms_file_thp, node_stat_item, NR_FILE_THPS)                             \
  X(ms_anon_thp, node_stat_item, NR_ANON_THPS)                             \
  X(ms_shmem_thp, node_stat_item, NR_SHMEM_THPS)                           \
  X(ms_inactive_anon, node_stat_item, NR_INACTIVE_ANON)                    \
  X(ms_active_anon, node_stat_item, NR_ACTIVE_ANON)                        \
  X(ms_inactive_file, node_stat_item, NR_INACTIVE_FILE)                    \
  X(ms_active_file, node_stat_item, NR_ACTIVE_FILE)                        \
  X(ms_unevictable, node_stat_item, NR_UNEVICTABLE)                        \
  X(ms_slab_reclaimable, node_stat_item, NR_SLAB_RECLAIMABLE_B)            \
  X(ms_slab_unreclaimable, node_stat_item, NR_SLAB_UNRECLAIMABLE_B)        \
  X(ms_workingset_refault_anon, node_stat_item, WORKINGSET_REFAULT_ANON)   \
  X(ms_workingset_refault_file, node_stat_item, WORKINGSET_REFAULT_FILE)   \
  X(ms_workingset_activate_anon, node_stat_item, WORKINGSET_ACTIVATE_ANON) \
  X(ms_workingset_activate_file, node_stat_item, WORKINGSET_ACTIVATE_FILE) \
  X(ms_workingset_restore_anon, node_stat_item, WORKINGSET_RESTORE_ANON)   \
  X(ms_workingset_restore_file, node_stat_item, WORKINGSET_RESTORE_FILE)   \
  X(ms_workingset_nodereclaim, node_stat_item, WORKINGSET_NODERECLAIM)

#define MEM_STAT_EVENT_FIELDS(X)         \
  X(ms_pgfault, PGFAULT)                 \
  X(ms_pgmajfault, PGMAJFAULT)           \
  X(ms_pgactivate, PGACTIVATE)           \
  X(ms_pgdeactivate, PGDEACTIVATE)       \
  X(ms_pglazyfree, PGLAZYFREE)           \
  X(ms_pglazyfreed, PGLAZYFREED)         \
  X(ms_thp_fault_alloc, THP_FAULT_ALLOC) \
  X(ms_thp_collapse_alloc, THP_COLLAPSE_ALLOC)

struct cgroup_bpf_record {
  unsigned long long cgroup_id;
  unsigned long long parent_id; // parent cgroup id; 0 for the root cgroup
  unsigned int valid;
  unsigned int pad;
  unsigned long long cpu_sum_exec_runtime_ns;
  unsigned long long cpu_utime_ns;
  unsigned long long cpu_stime_ns;
  unsigned long long cpu_nr_periods;
  unsigned long long cpu_nr_throttled;
  unsigned long long cpu_throttled_time_ns;
  unsigned long long memory_current_pages;
  // memory.stat "page state" counters, declared from the X-list.
#define X(field, etype, item) unsigned long long field;
  MEM_STAT_PAGE_STATE_FIELDS(X)
#undef X
  // pgscan/pgsteal are the file's aggregate, the sum of their four components.
  // below reads only the aggregates today; the components are recorded anyway
  // so the record mirrors memory.stat's full breakdown.
  unsigned long long ms_pgscan;
  unsigned long long ms_pgsteal;
  // The counters read by index (hugetlb included), declared from their X-list.
#define X(name, field) unsigned long long field;
  MEM_STAT_FIELDS_READ_BY_INDEX(X)
#undef X
  // memory.stat event counters, declared from the X-list.
#define X(field, item) unsigned long long field;
  MEM_STAT_EVENT_FIELDS(X)
#undef X
  // cgroup.stat (lock-maintained ints in struct cgroup; clamped >= 0).
  unsigned long long cgroup_nr_descendants;
  unsigned long long cgroup_nr_dying_descendants;
  // memory.events / memory.events.local (raw event counts; low..oom_kill only,
  // sock_throttled is left to the file -- see cgroup_bpf.rs).
  unsigned long long me_low;
  unsigned long long me_high;
  unsigned long long me_max;
  unsigned long long me_oom;
  unsigned long long me_oom_kill;
  unsigned long long mel_low;
  unsigned long long mel_high;
  unsigned long long mel_max;
  unsigned long long mel_oom;
  unsigned long long mel_oom_kill;
  // memory.{min,low,high,max} limits: raw page_counter values (pages); user
  // space applies the PAGE_COUNTER_MAX -> "max"(-1) sentinel and pages->bytes.
  unsigned long long mem_min;
  unsigned long long mem_low;
  unsigned long long mem_high;
  unsigned long long mem_max;
  unsigned long long mem_oom_group; // memory.oom.group flag (0/1)
};
