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
  34: optional i64 cma_total,
  35: optional i64 cma_free,
  36: optional i64 vmalloc_total,
  37: optional i64 vmalloc_used,
  38: optional i64 vmalloc_chunk,
  39: optional i64 direct_map_4k,
  40: optional i64 direct_map_2m,
  41: optional i64 direct_map_1g,
}

struct InterfaceStat {
  1: optional i64 collisions,
  2: optional i64 multicast,
  3: optional i64 rx_bytes,
  4: optional i64 rx_compressed,
  5: optional i64 rx_crc_errors,
  6: optional i64 rx_dropped,
  7: optional i64 rx_errors,
  8: optional i64 rx_fifo_errors,
  9: optional i64 rx_frame_errors,
  10: optional i64 rx_length_errors,
  11: optional i64 rx_missed_errors,
  12: optional i64 rx_nohandler,
  13: optional i64 rx_over_errors,
  14: optional i64 rx_packets,
  15: optional i64 tx_aborted_errors,
  16: optional i64 tx_bytes,
  17: optional i64 tx_carrier_errors,
  18: optional i64 tx_compressed,
  19: optional i64 tx_dropped,
  20: optional i64 tx_errors,
  21: optional i64 tx_fifo_errors,
  22: optional i64 tx_heartbeat_errors,
  23: optional i64 tx_packets,
  24: optional i64 tx_window_errors,
}

struct TcpStat {
  1: optional i64 active_opens,
  2: optional i64 passive_opens,
  3: optional i64 attempt_fails,
  4: optional i64 estab_resets,
  5: optional i64 curr_estab,
  6: optional i64 in_segs,
  7: optional i64 out_segs,
  8: optional i64 retrans_segs,
  9: optional i64 in_errs,
  10: optional i64 out_rsts,
  11: optional i64 in_csum_errors,
}

struct TcpExtStat {
  1: optional i64 syncookies_sent,
  2: optional i64 syncookies_recv,
  3: optional i64 syncookies_failed,
  4: optional i64 embryonic_rsts,
  5: optional i64 prune_called,
  6: optional i64 tw,
  7: optional i64 paws_estab,
  8: optional i64 delayed_acks,
  9: optional i64 delayed_ack_locked,
  10: optional i64 delayed_ack_lost,
  11: optional i64 listen_overflows,
  12: optional i64 listen_drops,
  13: optional i64 tcp_hp_hits,
  14: optional i64 tcp_pure_acks,
  15: optional i64 tcp_hp_acks,
  16: optional i64 tcp_reno_recovery,
  17: optional i64 tcp_reno_reorder,
  18: optional i64 tcp_ts_reorder,
  19: optional i64 tcp_full_undo,
  20: optional i64 tcp_partial_undo,
  21: optional i64 tcp_dsack_undo,
  22: optional i64 tcp_loss_undo,
  23: optional i64 tcp_lost_retransmit,
  24: optional i64 tcp_reno_failures,
  25: optional i64 tcp_loss_failures,
  26: optional i64 tcp_fast_retrans,
  27: optional i64 tcp_slow_start_retrans,
  28: optional i64 tcp_timeouts,
}

struct IpExtStat {
  1: optional i64 in_mcast_pkts,
  2: optional i64 out_mcast_pkts,
  3: optional i64 in_bcast_pkts,
  4: optional i64 out_bcast_pkts,
  5: optional i64 in_octets,
  6: optional i64 out_octets,
  7: optional i64 in_mcast_octets,
  8: optional i64 out_mcast_octets,
  9: optional i64 in_bcast_octets,
  10: optional i64 out_bcast_octets,
  11: optional i64 in_no_ect_pkts,
}

struct IpStat {
  1: optional i64 forwarding,
  2: optional i64 in_receives,
  3: optional i64 forw_datagrams,
  4: optional i64 in_discards,
  5: optional i64 in_delivers,
  6: optional i64 out_requests,
  7: optional i64 out_discards,
  8: optional i64 out_no_routes,
}

struct Ip6Stat {
  1: optional i64 in_receives,
  2: optional i64 in_hdr_errors,
  3: optional i64 in_no_routes,
  4: optional i64 in_addr_errors,
  5: optional i64 in_discards,
  6: optional i64 in_delivers,
  7: optional i64 out_forw_datagrams,
  8: optional i64 out_requests,
  9: optional i64 out_no_routes,
  10: optional i64 in_mcast_pkts,
  11: optional i64 out_mcast_pkts,
  12: optional i64 in_octets,
  13: optional i64 out_octets,
  14: optional i64 in_mcast_octets,
  15: optional i64 out_mcast_octets,
  16: optional i64 in_bcast_octets,
  17: optional i64 out_bcast_octets,
}

struct IcmpStat {
  1: optional i64 in_msgs,
  2: optional i64 in_errors,
  3: optional i64 in_dest_unreachs,
  4: optional i64 out_msgs,
  5: optional i64 out_errors,
  6: optional i64 out_dest_unreachs,
}

struct Icmp6Stat {
  1: optional i64 in_msgs,
  2: optional i64 in_errors,
  3: optional i64 out_msgs,
  4: optional i64 out_errors,
  5: optional i64 in_dest_unreachs,
  6: optional i64 out_dest_unreachs,
}

struct UdpStat {
  1: optional i64 in_datagrams,
  2: optional i64 no_ports,
  3: optional i64 in_errors,
  4: optional i64 out_datagrams,
  5: optional i64 rcvbuf_errors,
  6: optional i64 sndbuf_errors,
  7: optional i64 ignored_multi,
}

struct Udp6Stat {
  1: optional i64 in_datagrams,
  2: optional i64 no_ports,
  3: optional i64 in_errors,
  4: optional i64 out_datagrams,
  5: optional i64 rcvbuf_errors,
  6: optional i64 sndbuf_errors,
  7: optional i64 in_csum_errors,
  8: optional i64 ignored_multi,
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

struct DiskStat {
  1: optional i64 major,
  2: optional i64 minor,
  3: optional string name,
  4: optional i64 read_completed,
  5: optional i64 read_merged,
  6: optional i64 read_sectors,
  7: optional i64 time_spend_read_ms,
  8: optional i64 write_completed,
  9: optional i64 write_merged,
  10: optional i64 write_sectors,
  11: optional i64 time_spend_write_ms,
  12: optional i64 discard_completed,
  13: optional i64 discard_merged,
  14: optional i64 discard_sectors,
  15: optional i64 time_spend_discard_ms,
}

enum PidState {
  RUNNING = 0,
  SLEEPING = 1,
  UNINTERRUPTIBLE_SLEEP = 2,
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
typedef map<string, InterfaceStat> NetMap
typedef map<string, DiskStat> DiskMap

struct NetStat {
  1: optional NetMap interfaces,
  2: optional TcpStat tcp,
  3: optional TcpExtStat tcp_ext,
  4: optional IpStat ip,
  5: optional IpExtStat ip_ext,
  6: optional Ip6Stat ip6,
  7: optional IcmpStat icmp,
  8: optional Icmp6Stat icmp6,
  9: optional UdpStat udp,
  10: optional Udp6Stat udp6,
}
