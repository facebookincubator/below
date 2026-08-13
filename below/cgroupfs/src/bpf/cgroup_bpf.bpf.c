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

// A cgroup BPF iterator (requires Linux >= 6.1). When read from user space it
// is invoked once per cgroup in the subtree rooted at the target cgroup fd
// (attached with BPF_CGROUP_ITER_DESCENDANTS_PRE), and stores one
// `struct cgroup_bpf_record` per cgroup into the `results` hash map keyed by
// cgroup id. User space (below/src/cgroup_bpf.rs) drives the iterator to EOF
// (which runs this program for every cgroup) and then drains the map.
//
// We deliberately do NOT emit to the iterator's seq stream: the kernel's
// per-read seq buffer is only ~32 KiB, and cgroup_iter cannot resume a walk
// across seq-buffer refills (the continuation read fails with EOPNOTSUPP), so a
// large tree would be truncated. Writing to a map has no such limit.
//
// CPU basic stats come from the cgroup's rstat `bstat`. The program brings both
// the cpu and the memcg stats up to date once, on the first cgroup
// (flush_rstat); the flush is subtree-wide, so every descendant read afterward
// is current. memory.current is a live page_counter value. memory.stat is read
// via the memcg BPF kfuncs (bpf_get_mem_cgroup /
// bpf_mem_cgroup_page_state / bpf_mem_cgroup_vm_events), which resolve the
// (possibly compacted) vmstats slot and apply the output unit inside the kernel
// -- so the values match memory.stat exactly, with no fragile user-space slot
// remapping. Every kfunc is optional (declared __weak __ksym and guarded by
// bpf_ksym_exists); when the kernel exports none of the memcg kfuncs the
// MEM_STAT valid bit stays clear and user space reads memory.stat from the file
// instead.

#ifdef FBCODE_BUILD
#include <bpf/vmlinux/vmlinux.h>
#else
// Shared vendored BTF from the below-vmlinux crate; found on the include path
// (buck: bpf_header_library + -I; cargo: DEP_BELOW_VMLINUX_INCLUDE via
// build.rs).
#include <vmlinux.h>
#endif // FBCODE_BUILD

#include <bpf/bpf_core_read.h>
#include <bpf/bpf_helpers.h>

// The wire record this program emits, one per cgroup. It lives in a standalone
// header so bindgen can generate the matching Rust struct from the same
// definition -- see cgroup_bpf_record.h and the Rust side in
// cgroup_bpf/types.rs.
#include "cgroup_bpf_record.h"

// Flush kfuncs bring a cgroup's stats up to date before we read them; all are
// __weak, so the program loads with whichever the running kernel exports.
// bpf_cgroup_rstat_flush / cgroup_rstat_flush flush the cpu stats and every
// subsystem, memcg included, in one call (the kfunc is named cgroup_rstat_flush
// upstream and bpf_cgroup_rstat_flush on some fleet kernels, so we try both).
// css_rstat_flush(css) is per-subsystem: passing &cgrp->self updates only the
// cpu stats, so memcg is flushed on its own css (see flush_rstat). If the
// kernel exports none of them, user space does not run this program and reads
// the files instead (see cgroup_bpf.rs).
extern void bpf_cgroup_rstat_flush(struct cgroup* cgrp) __weak __ksym;
extern void cgroup_rstat_flush(struct cgroup* cgrp) __weak __ksym;
extern void css_rstat_flush(struct cgroup_subsys_state* css) __weak __ksym;

// Memcg stat kfuncs, upstream in Linux 7.0 and backported to some earlier fleet
// kernels -- which is why every use below tests for the kfunc itself rather
// than for a kernel version. bpf_get_mem_cgroup gets the cgroup's memcg and
// bpf_put_mem_cgroup releases it. bpf_mem_cgroup_page_state and
// bpf_mem_cgroup_vm_events each read one memory.stat counter, in the same unit
// the file prints, and return -1 when the kernel does not track that counter.
// bpf_mem_cgroup_flush_stats brings the memcg stats up to date, the way the
// file does before it prints memory.stat. All are __weak, so the program loads
// on kernels that lack them. Each call is guarded by bpf_ksym_exists, and when
// they are missing user space reads memory.stat from the file.
extern struct mem_cgroup* bpf_get_mem_cgroup(
    struct cgroup_subsys_state* css) __weak __ksym;
extern void bpf_put_mem_cgroup(struct mem_cgroup* memcg) __weak __ksym;
extern void bpf_mem_cgroup_flush_stats(struct mem_cgroup* memcg) __weak __ksym;
extern unsigned long bpf_mem_cgroup_page_state(
    struct mem_cgroup* memcg,
    int idx) __weak __ksym;
extern unsigned long bpf_mem_cgroup_vm_events(
    struct mem_cgroup* memcg,
    enum vm_event_item event) __weak __ksym;

// The bit in `struct cgroup_bpf_record::valid` for one cgroup_files group. A
// set bit means BPF supplied that group; a clear bit tells user space to read
// the file. The bit index per group is enum cgroup_files (in
// cgroup_bpf_record.h), shared with the Rust side via bindgen.
#define CGROUP_FILE_BIT(f) (1u << (f))

// BPF_F_NO_PREALLOC: allocate entries on demand rather than preallocating all
// max_entries at load. The record is large (~600 B), so a full prealloc would
// pin tens of MB of kernel memory regardless of the live cgroup count;
// on-demand alloc is safe here because the map is written only from the
// sleepable iterator (process context). max_entries stays an upper bound.
struct {
  __uint(type, BPF_MAP_TYPE_HASH);
  __uint(max_entries, 65536);
  __uint(map_flags, BPF_F_NO_PREALLOC);
  __type(key, u64);
  __type(value, struct cgroup_bpf_record);
} results SEC(".maps");

// Per-CPU scratch for building one record. The record is too large (> the 512B
// BPF stack limit) to live on the stack, so we fill it here and then copy it
// into `results`. Safe because below drives the iterator from a single thread,
// one read at a time (see cgroup_bpf.rs) -- the program is never attached and
// read concurrently, so nothing else touches this CPU's slot between fill and
// store.
struct {
  __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
  __uint(max_entries, 1);
  __type(key, u32);
  __type(value, struct cgroup_bpf_record);
} scratch SEC(".maps");

// Groups to skip reading, so we don't build a struct user space would discard
// and read from the file anyway. A set bit (one CGROUP_FILES_* index) means
// skip that group. User space sets it once at startup from what it found out
// about the kernel (see cgroup_bpf/coverage.rs); default 0 = read all.
struct {
  __uint(type, BPF_MAP_TYPE_ARRAY);
  __uint(max_entries, 1);
  __type(key, u32);
  __type(value, u32);
} skip_groups SEC(".maps");

// Extra reads user space has turned on, as CGROUP_BPF_FEATURE_* bits. A bit is
// set only when user space checked that the cgroupfs file shows that field, so
// a clear bit leaves the -1 sentinel and the field comes from the file.
struct {
  __uint(type, BPF_MAP_TYPE_ARRAY);
  __uint(max_entries, 1);
  __type(key, u32);
  __type(value, u32);
} features SEC(".maps");

// How to read each counter the program cannot name, one entry per
// enum cgroup_bpf_item slot. User space fills it from the running kernel's BTF
// (see MEM_STAT_FIELDS_READ_BY_INDEX in cgroup_bpf_record.h).
struct {
  __uint(type, BPF_MAP_TYPE_ARRAY);
  __uint(max_entries, CGROUP_BPF_ITEM_NUM);
  __type(key, u32);
  __type(value, struct cgroup_bpf_item_index);
} item_index SEC(".maps");

// Read one memcg "page state" counter (a node_stat_item / memcg_stat_item) into
// FIELD, already in memory.stat's output unit. Leaves the -1 "unavailable"
// sentinel when the running kernel does not track the counter -- either the
// enumerator is absent from its BTF, or the kfunc itself returns -1. User space
// maps -1 to None, matching memory.stat omitting that line.
#define KF_STATE(FIELD, ETYPE, ITEM)                     \
  do {                                                   \
    unsigned long _v = (unsigned long)-1;                \
    if (bpf_core_enum_value_exists(enum ETYPE, ITEM))    \
      _v = bpf_mem_cgroup_page_state(                    \
          memcg, bpf_core_enum_value(enum ETYPE, ITEM)); \
    rec->FIELD = _v;                                     \
  } while (0)

// Read one memcg event counter (a vm_event_item) into FIELD, or leave the -1
// "unavailable" sentinel (see KF_STATE).
#define KF_EVENT(FIELD, ITEM)                                    \
  do {                                                           \
    unsigned long _v = (unsigned long)-1;                        \
    if (bpf_core_enum_value_exists(enum vm_event_item, ITEM))    \
      _v = bpf_mem_cgroup_vm_events(                             \
          memcg, bpf_core_enum_value(enum vm_event_item, ITEM)); \
    rec->FIELD = _v;                                             \
  } while (0)

// Read one counter by the index user space found for it, or return the -1
// sentinel when this kernel has no such counter.
static __always_inline unsigned long read_by_index(
    struct mem_cgroup* memcg,
    u32 slot) {
  struct cgroup_bpf_item_index* item = bpf_map_lookup_elem(&item_index, &slot);
  if (!item)
    return (unsigned long)-1;
  if (item->source == CGROUP_BPF_ITEM_PAGE_STATE)
    return bpf_mem_cgroup_page_state(memcg, item->index);
  if (item->source == CGROUP_BPF_ITEM_VM_EVENT)
    return bpf_mem_cgroup_vm_events(memcg, (enum vm_event_item)item->index);
  return (unsigned long)-1;
}

// Read one pgscan/pgsteal component into FIELD and, when this kernel has it,
// add it to the running aggregate SUM (setting ANY, so the caller leaves the
// aggregate at -1 only when no component is present). memory.stat prints both
// the components and their sum, so we record both.
#define RECLAIM_COMPONENT(FIELD, SLOT, SUM, ANY)   \
  do {                                             \
    unsigned long _v = read_by_index(memcg, SLOT); \
    rec->FIELD = _v;                               \
    if (_v != (unsigned long)-1) {                 \
      (SUM) += _v;                                 \
      (ANY) = 1;                                   \
    }                                              \
  } while (0)

// Return the cgroup's css for one subsystem, or NULL when the subsystem is not
// enabled on the cgroup.
//
// @ssid must come from bpf_core_enum_value(enum cgroup_subsys_id, ...). The ids
// are numbered over the subsystems the kernel was built with, so a kernel built
// without, say, cpuacct shifts every later id down by one, and a compile-time
// value would select another subsystem's css for us to cast to the wrong type.
// A CO-RE array index must be a literal, so BPF_CORE_READ(cgrp, subsys[ssid])
// is not an option: relocate the array's offset and index it here.
static __always_inline struct cgroup_subsys_state* cgroup_css(
    struct cgroup* cgrp,
    u32 ssid) {
  struct cgroup_subsys_state* css = NULL;
  if (ssid >= bpf_core_enum_value(enum cgroup_subsys_id, CGROUP_SUBSYS_COUNT))
    return NULL;
  unsigned long off =
      bpf_core_field_offset(struct cgroup, subsys) + ssid * sizeof(css);
  if (bpf_probe_read_kernel(&css, sizeof(css), (void*)cgrp + off))
    return NULL;
  return css;
}

// Flush this cgroup's rstat subtree so bstat (cpu) and the memcg stats become
// current, using whichever flush kfunc the kernel exports. Called once on the
// first (root) element; the flush is subtree-wide, so one call covers every
// descendant read afterward.
//
// bpf_cgroup_rstat_flush / cgroup_rstat_flush flush the cpu base stats and
// every subsystem, memcg included, in one call. The newer css_rstat_flush is
// per-subsystem: css_rstat_flush(&cgrp->self) flushes only the cpu (self) css
// and leaves memcg untouched, so on that path we flush memcg on its own css --
// via bpf_mem_cgroup_flush_stats, or css_rstat_flush(&memcg->css) when that
// kfunc is absent.
static __always_inline void flush_rstat(struct cgroup* cgrp) {
  if (bpf_ksym_exists(bpf_cgroup_rstat_flush)) {
    bpf_cgroup_rstat_flush(cgrp);
    return;
  }
  if (bpf_ksym_exists(cgroup_rstat_flush)) {
    cgroup_rstat_flush(cgrp);
    return;
  }
  if (!bpf_ksym_exists(css_rstat_flush))
    return;
  css_rstat_flush(&cgrp->self); // cpu/base only

  // memcg lives on its own css. barrier_var keeps the compiler from folding the
  // weak-symbol checks into a bitwise op on their addresses, which the verifier
  // rejects.
  bool has_get = bpf_ksym_exists(bpf_get_mem_cgroup);
  bool has_put = bpf_ksym_exists(bpf_put_mem_cgroup);
  barrier_var(has_get);
  barrier_var(has_put);
  if (!has_get || !has_put)
    return;
  struct mem_cgroup* memcg = bpf_get_mem_cgroup(&cgrp->self);
  if (memcg) {
    if (bpf_ksym_exists(bpf_mem_cgroup_flush_stats))
      bpf_mem_cgroup_flush_stats(memcg);
    else
      css_rstat_flush(&memcg->css);
    bpf_put_mem_cgroup(memcg);
  }
}

// Whether flush_rstat's flush covered memcg this pass, so memory.stat (which is
// rstat-backed) is fresh. Mirrors the kfunc availability flush_rstat keys off;
// ksym availability is constant across the walk, so every cgroup sees what the
// root saw. barrier_var: see flush_rstat.
static __always_inline bool memcg_flush_applied(void) {
  bool has_bpf_cgroup_flush = bpf_ksym_exists(bpf_cgroup_rstat_flush);
  bool has_cgroup_flush = bpf_ksym_exists(cgroup_rstat_flush);
  bool has_css_flush = bpf_ksym_exists(css_rstat_flush);
  bool has_get = bpf_ksym_exists(bpf_get_mem_cgroup);
  bool has_put = bpf_ksym_exists(bpf_put_mem_cgroup);
  barrier_var(has_bpf_cgroup_flush);
  barrier_var(has_cgroup_flush);
  barrier_var(has_css_flush);
  barrier_var(has_get);
  barrier_var(has_put);
  // A whole-cgroup flush covers every subsystem, memcg included.
  if (has_bpf_cgroup_flush || has_cgroup_flush)
    return true;
  // Per-css path: memcg is flushed via its own css, reached with the
  // acquire/release kfuncs.
  return has_css_flush && has_get && has_put;
}

// cpu.stat: raw bstat cputime (usage), plus cfs_bandwidth throttle when the cpu
// controller is enabled.
static __always_inline void collect_cpu(
    struct cgroup* cgrp,
    struct cgroup_bpf_record* rec) {
  // The root cgroup's cpu.stat is special: the kernel computes it from global
  // per-CPU kcpustat (root_cgroup_cputime), not from cgrp->bstat, which is
  // 0/undefined for the root. Reading bstat here would report 0 for the root
  // and mismatch the file, so skip cpu for the root (level 0) and let user
  // space read the file.
  if (BPF_CORE_READ(cgrp, level) == 0)
    return;
  rec->cpu_sum_exec_runtime_ns =
      BPF_CORE_READ(cgrp, bstat.cputime.sum_exec_runtime);
  rec->cpu_utime_ns = BPF_CORE_READ(cgrp, bstat.cputime.utime);
  rec->cpu_stime_ns = BPF_CORE_READ(cgrp, bstat.cputime.stime);
  rec->valid |=
      CGROUP_FILE_BIT(CGROUP_FILES_CPU_USAGE); // setup bitmask for userspace

  // The throttle lines need the cpu controller's cfs_bandwidth, which exists
  // only with CONFIG_CFS_BANDWIDTH. Reading it unguarded would fail to relocate
  // and the whole program would not load.
  if (!bpf_core_enum_value_exists(enum cgroup_subsys_id, cpu_cgrp_id) ||
      !bpf_core_field_exists(struct task_group, cfs_bandwidth))
    return;
  struct cgroup_subsys_state* cpu_css =
      cgroup_css(cgrp, bpf_core_enum_value(enum cgroup_subsys_id, cpu_cgrp_id));
  if (cpu_css) {
    struct task_group* tg = (struct task_group*)cpu_css;
    rec->cpu_nr_periods = BPF_CORE_READ(tg, cfs_bandwidth.nr_periods);
    rec->cpu_nr_throttled = BPF_CORE_READ(tg, cfs_bandwidth.nr_throttled);
    rec->cpu_throttled_time_ns =
        BPF_CORE_READ(tg, cfs_bandwidth.throttled_time);
    rec->valid |= CGROUP_FILE_BIT(
        CGROUP_FILES_CPU_THROTTLE); // setup bitmask for userspace
  }
}

// cgroup.stat: nr_descendants / nr_dying_descendants are plain lock-maintained
// ints in struct cgroup (not rstat), read directly off the iterator's cgrp.
static __always_inline void collect_cgroup_stat(
    struct cgroup* cgrp,
    struct cgroup_bpf_record* rec) {
  if (!bpf_core_field_exists(cgrp->nr_descendants) ||
      !bpf_core_field_exists(cgrp->nr_dying_descendants))
    return;
  int nd = BPF_CORE_READ(cgrp, nr_descendants);
  int ndd = BPF_CORE_READ(cgrp, nr_dying_descendants);
  rec->cgroup_nr_descendants = nd > 0 ? (u64)nd : 0;
  rec->cgroup_nr_dying_descendants = ndd > 0 ? (u64)ndd : 0;
  rec->valid |=
      CGROUP_FILE_BIT(CGROUP_FILES_CGROUP_STAT); // setup bitmask for userspace
}

// memory.current, memory.events(.local), and the memory.{min,low,high,max} +
// memory.oom.group limits -- all read directly off the memcg. memory.events is
// gated by `skip`, since a newer kernel prints a line here that we do not read
// (see coverage.rs).
static __always_inline void
collect_memory(struct cgroup* cgrp, struct cgroup_bpf_record* rec, u32 skip) {
  // The root cgroup has no memory.current / memory.events(.local) /
  // memory.{min,low,high,max} / memory.oom.group interface files
  // (root_mem_cgroup is not a real charge target), so reading them off the
  // memcg would report values the file omits. Skip the root (level 0) and let
  // user space read the absent file -> None, matching the file path (same
  // reasoning as collect_cpu).
  if (BPF_CORE_READ(cgrp, level) == 0)
    return;
  if (!bpf_core_enum_value_exists(enum cgroup_subsys_id, memory_cgrp_id))
    return;
  struct cgroup_subsys_state* mem_css = cgroup_css(
      cgrp, bpf_core_enum_value(enum cgroup_subsys_id, memory_cgrp_id));
  if (!mem_css)
    return;
  struct mem_cgroup* memcg = (struct mem_cgroup*)mem_css;

  rec->memory_current_pages = BPF_CORE_READ(memcg, memory.usage.counter);
  rec->valid |=
      CGROUP_FILE_BIT(CGROUP_FILES_MEM_CURRENT); // setup bitmask for userspace

  // memory.events / memory.events.local: fixed atomic arrays indexed directly
  // by enum memcg_memory_event (low..oom_kill are stable indices 0..4).
  if (!(skip & CGROUP_FILE_BIT(CGROUP_FILES_MEM_EVENTS))) {
    rec->me_low = BPF_CORE_READ(memcg, memory_events[MEMCG_LOW].counter);
    rec->me_high = BPF_CORE_READ(memcg, memory_events[MEMCG_HIGH].counter);
    rec->me_max = BPF_CORE_READ(memcg, memory_events[MEMCG_MAX].counter);
    rec->me_oom = BPF_CORE_READ(memcg, memory_events[MEMCG_OOM].counter);
    rec->me_oom_kill =
        BPF_CORE_READ(memcg, memory_events[MEMCG_OOM_KILL].counter);
    rec->mel_low = BPF_CORE_READ(memcg, memory_events_local[MEMCG_LOW].counter);
    rec->mel_high =
        BPF_CORE_READ(memcg, memory_events_local[MEMCG_HIGH].counter);
    rec->mel_max = BPF_CORE_READ(memcg, memory_events_local[MEMCG_MAX].counter);
    rec->mel_oom = BPF_CORE_READ(memcg, memory_events_local[MEMCG_OOM].counter);
    rec->mel_oom_kill =
        BPF_CORE_READ(memcg, memory_events_local[MEMCG_OOM_KILL].counter);
    rec->valid |=
        CGROUP_FILE_BIT(CGROUP_FILES_MEM_EVENTS); // setup bitmask for userspace
  }

  // memory.{min,low,high,max} limits (page_counter, in pages) +
  // memory.oom.group.
  rec->mem_min = BPF_CORE_READ(memcg, memory.min);
  rec->mem_low = BPF_CORE_READ(memcg, memory.low);
  rec->mem_high = BPF_CORE_READ(memcg, memory.high);
  rec->mem_max = BPF_CORE_READ(memcg, memory.max);
  rec->mem_oom_group = BPF_CORE_READ(memcg, oom_group) ? 1 : 0;
  rec->valid |=
      CGROUP_FILE_BIT(CGROUP_FILES_MEM_LIMITS); // setup bitmask for userspace
}

// memory.stat via the memcg kfuncs. Read only when flush_rstat covered memcg
// (memcg_flush_applied) and the read kfuncs exist; otherwise the MEM_STAT valid
// bit stays clear and user space reads memory.stat from the file. The memcg
// reference is acquired and released here so the pair stays in one place -- no
// flush here, flush_rstat already flushed this memcg's whole subtree once on
// the first cgroup.
static __always_inline void collect_memory_stat(
    struct cgroup* cgrp,
    struct cgroup_bpf_record* rec,
    u32 feats) {
  // Skip the root (level 0): its memcg is special (like collect_cpu /
  // collect_memory), so fall back to the file for the root's memory.stat.
  if (BPF_CORE_READ(cgrp, level) == 0)
    return;
  // barrier_var keeps the compiler from folding the weak-symbol checks into a
  // bitwise op on their addresses, which the verifier rejects (see
  // flush_rstat).
  bool has_get = bpf_ksym_exists(bpf_get_mem_cgroup);
  bool has_put = bpf_ksym_exists(bpf_put_mem_cgroup);
  bool has_page_state = bpf_ksym_exists(bpf_mem_cgroup_page_state);
  bool has_vm_events = bpf_ksym_exists(bpf_mem_cgroup_vm_events);
  barrier_var(has_get);
  barrier_var(has_put);
  barrier_var(has_page_state);
  barrier_var(has_vm_events);
  if (!memcg_flush_applied() || !has_get || !has_put || !has_page_state ||
      !has_vm_events)
    return;
  struct mem_cgroup* memcg = bpf_get_mem_cgroup(&cgrp->self);
  if (!memcg)
    return;

  // "page state" counters (node_stat_item / memcg_stat_item), read from the
  // shared X-list so each field's struct slot and this read stay in lockstep.
#define X(field, etype, item) KF_STATE(field, etype, item);
  MEM_STAT_PAGE_STATE_FIELDS(X)
#undef X

  // The next three groups are lines memory.stat prints only under an option,
  // though the kernel tracks their counters either way. The kfunc returns a
  // real 0 there, so each is read only when its option is on.

  // anon_thp/file_thp/shmem_thp need CONFIG_TRANSPARENT_HUGEPAGE, which also
  // drops the THP_FAULT_ALLOC event. They are read above with the X-list, which
  // keeps that list uniform, and cleared here.
  if (!bpf_core_enum_value_exists(enum vm_event_item, THP_FAULT_ALLOC)) {
    rec->ms_anon_thp = (u64)-1;
    rec->ms_file_thp = (u64)-1;
    rec->ms_shmem_thp = (u64)-1;
  }
  // zswap/zswapped need CONFIG_ZSWAP. Nothing the program can name shows that,
  // so user space checks it and sets the feature bit.
  if (!(feats & (1u << CGROUP_BPF_FEATURE_ZSWAP))) {
    rec->ms_zswap = (u64)-1;
    rec->ms_zswapped = (u64)-1;
  }
  // hugetlb needs the cgroup2 mount to account hugetlb to memcg, which BPF
  // cannot see; user space reads the mount option and sets the feature bit.
  rec->ms_hugetlb = (u64)-1;
  if (feats & (1u << CGROUP_BPF_FEATURE_HUGETLB))
    rec->ms_hugetlb = read_by_index(memcg, CGROUP_BPF_ITEM_NR_HUGETLB);

  // The reclaim counters, read by index rather than by name (see
  // MEM_STAT_RECLAIM_FIELDS). pgscan/pgsteal record each component and the
  // file's aggregate, which is their sum.
  rec->ms_pgrefill = read_by_index(memcg, CGROUP_BPF_ITEM_PGREFILL);
  u64 pgscan = 0, pgsteal = 0;
  int pgscan_any = 0, pgsteal_any = 0;
  RECLAIM_COMPONENT(
      ms_pgscan_kswapd, CGROUP_BPF_ITEM_PGSCAN_KSWAPD, pgscan, pgscan_any);
  RECLAIM_COMPONENT(
      ms_pgscan_direct, CGROUP_BPF_ITEM_PGSCAN_DIRECT, pgscan, pgscan_any);
  RECLAIM_COMPONENT(
      ms_pgscan_khugepaged,
      CGROUP_BPF_ITEM_PGSCAN_KHUGEPAGED,
      pgscan,
      pgscan_any);
  RECLAIM_COMPONENT(
      ms_pgscan_proactive,
      CGROUP_BPF_ITEM_PGSCAN_PROACTIVE,
      pgscan,
      pgscan_any);
  rec->ms_pgscan = pgscan_any ? pgscan : (u64)-1;
  RECLAIM_COMPONENT(
      ms_pgsteal_kswapd, CGROUP_BPF_ITEM_PGSTEAL_KSWAPD, pgsteal, pgsteal_any);
  RECLAIM_COMPONENT(
      ms_pgsteal_direct, CGROUP_BPF_ITEM_PGSTEAL_DIRECT, pgsteal, pgsteal_any);
  RECLAIM_COMPONENT(
      ms_pgsteal_khugepaged,
      CGROUP_BPF_ITEM_PGSTEAL_KHUGEPAGED,
      pgsteal,
      pgsteal_any);
  RECLAIM_COMPONENT(
      ms_pgsteal_proactive,
      CGROUP_BPF_ITEM_PGSTEAL_PROACTIVE,
      pgsteal,
      pgsteal_any);
  rec->ms_pgsteal = pgsteal_any ? pgsteal : (u64)-1;

  // event counters (vm_event_item), read from the shared X-list.
#define X(field, item) KF_EVENT(field, item);
  MEM_STAT_EVENT_FIELDS(X)
#undef X

  rec->valid |=
      CGROUP_FILE_BIT(CGROUP_FILES_MEM_STAT); // setup bitmask for userspace
  bpf_put_mem_cgroup(memcg);
}

SEC("iter.s/cgroup")
int cgroup_bpf_read(struct bpf_iter__cgroup* ctx) {
  // cgrp: the one cgroup this invocation handles -- the iterator calls us once
  // per cgroup. The final call passes NULL (the iterator's end marker); skip
  // it.
  struct cgroup* cgrp = ctx->cgroup;
  if (cgrp == 0)
    return 0;

  if (ctx->meta->seq_num == 0)
    flush_rstat(cgrp);

  u32 zero = 0;

  // @rec: this cgroup's record, built in the per-CPU `scratch` slot rather than
  // on the stack. struct cgroup_bpf_record exceeds the 512-byte BPF stack
  // limit, so a stack copy will not even compile, e.g., it fails on: struct
  // cgroup_bpf_record rec = {};
  struct cgroup_bpf_record* rec = bpf_map_lookup_elem(&scratch, &zero);
  if (rec == 0)
    return 0;
  // The slot is reused for every cgroup on this CPU, so clear the previous
  // cgroup's data first; otherwise its fields and `valid` bits would leak in.
  __builtin_memset(rec, 0, sizeof(*rec));

  rec->cgroup_id = BPF_CORE_READ(cgrp, kn, id);

  // Parent cgroup id (0 for the root, whose self.parent is NULL). Reached via
  // the css parent rather than kn->parent: kernfs_node.parent was renamed to
  // __parent in Linux 6.15, while cgroup->self.parent and css->cgroup are
  // stable, so this stays CO-RE-clean across kernels. The parent's kn->id
  // equals its cgroup_id, so user space can rebuild the tree by joining
  // parent_id to cgroup_id.
  struct cgroup_subsys_state* parent_css = BPF_CORE_READ(cgrp, self.parent);
  rec->parent_id = BPF_CORE_READ(parent_css, cgroup, kn, id);

  // skip: the stat files BPF must not supply on this kernel, so we do not spend
  // the reads on a struct the collector would ignore in favour of the file.
  u32* skip_mask_ptr = bpf_map_lookup_elem(&skip_groups, &zero);
  u32 skip_mask = skip_mask_ptr ? *skip_mask_ptr : 0;

  // feats: the extra reads user space turned on (CGROUP_BPF_FEATURE_*).
  u32* feats_ptr = bpf_map_lookup_elem(&features, &zero);
  u32 feats = feats_ptr ? *feats_ptr : 0;

  if (!(skip_mask & CGROUP_FILE_BIT(CGROUP_FILES_CPU_USAGE)))
    collect_cpu(cgrp, rec); // for cpu.stat
  if (!(skip_mask & CGROUP_FILE_BIT(CGROUP_FILES_CGROUP_STAT)))
    collect_cgroup_stat(cgrp, rec); // for cgroup.stat
  if (!(skip_mask & CGROUP_FILE_BIT(CGROUP_FILES_MEM_STAT)))
    collect_memory_stat(cgrp, rec, feats); // for memory.stat

  collect_memory(cgrp, rec, skip_mask); // for memory.{min, max ... }
  // Publish the finished record into `results`, keyed by cgroup id; user space
  // drains the map once the iterator reaches EOF.
  u64 key = rec->cgroup_id;
  bpf_map_update_elem(&results, &key, rec, BPF_ANY);
  return 0;
}

char _license[] SEC("license") = "GPL";
