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

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CpuStat {
    pub user_usec: Option<u64>,
    pub nice_usec: Option<u64>,
    pub system_usec: Option<u64>,
    pub idle_usec: Option<u64>,
    pub iowait_usec: Option<u64>,
    pub irq_usec: Option<u64>,
    pub softirq_usec: Option<u64>,
    pub stolen_usec: Option<u64>,
    pub guest_usec: Option<u64>,
    pub guest_nice_usec: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stat {
    pub total_cpu: Option<CpuStat>,
    pub cpus_map: Option<BTreeMap<u32, CpuStat>>,
    pub total_interrupt_count: Option<u64>,
    pub context_switches: Option<u64>,
    pub boot_time_epoch_secs: Option<u64>,
    pub total_processes: Option<u64>,
    pub running_processes: Option<u32>,
    pub blocked_processes: Option<u32>,
}

// In kilobytes unless specified otherwise
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MemInfo {
    pub total: Option<u64>,
    pub free: Option<u64>,
    pub available: Option<u64>,
    pub buffers: Option<u64>,
    pub cached: Option<u64>,
    pub swap_cached: Option<u64>,
    pub active: Option<u64>,
    pub inactive: Option<u64>,
    pub active_anon: Option<u64>,
    pub inactive_anon: Option<u64>,
    pub active_file: Option<u64>,
    pub inactive_file: Option<u64>,
    pub unevictable: Option<u64>,
    pub mlocked: Option<u64>,
    pub swap_total: Option<u64>,
    pub swap_free: Option<u64>,
    pub dirty: Option<u64>,
    pub writeback: Option<u64>,
    pub anon_pages: Option<u64>,
    pub mapped: Option<u64>,
    pub shmem: Option<u64>,
    pub kreclaimable: Option<u64>,
    pub slab: Option<u64>,
    pub slab_reclaimable: Option<u64>,
    pub slab_unreclaimable: Option<u64>,
    pub kernel_stack: Option<u64>,
    pub page_tables: Option<u64>,
    pub anon_huge_pages: Option<u64>,
    pub shmem_huge_pages: Option<u64>,
    pub file_huge_pages: Option<u64>,
    // This is in number of pages: not kilobytes
    pub total_huge_pages: Option<u64>,
    // This is in number of pages: not kilobytes
    pub free_huge_pages: Option<u64>,
    pub huge_page_size: Option<u64>,
    pub cma_total: Option<u64>,
    pub cma_free: Option<u64>,
    pub vmalloc_total: Option<u64>,
    pub vmalloc_used: Option<u64>,
    pub vmalloc_chunk: Option<u64>,
    pub direct_map_4k: Option<u64>,
    pub direct_map_2m: Option<u64>,
    pub direct_map_1g: Option<u64>,
    pub hugetlb: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct InterfaceStat {
    pub collisions: Option<u64>,
    pub multicast: Option<u64>,
    pub rx_bytes: Option<u64>,
    pub rx_compressed: Option<u64>,
    pub rx_crc_errors: Option<u64>,
    pub rx_dropped: Option<u64>,
    pub rx_errors: Option<u64>,
    pub rx_fifo_errors: Option<u64>,
    pub rx_frame_errors: Option<u64>,
    pub rx_length_errors: Option<u64>,
    pub rx_missed_errors: Option<u64>,
    pub rx_nohandler: Option<u64>,
    pub rx_over_errors: Option<u64>,
    pub rx_packets: Option<u64>,
    pub tx_aborted_errors: Option<u64>,
    pub tx_bytes: Option<u64>,
    pub tx_carrier_errors: Option<u64>,
    pub tx_compressed: Option<u64>,
    pub tx_dropped: Option<u64>,
    pub tx_errors: Option<u64>,
    pub tx_fifo_errors: Option<u64>,
    pub tx_heartbeat_errors: Option<u64>,
    pub tx_packets: Option<u64>,
    pub tx_window_errors: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TcpStat {
    pub active_opens: Option<u64>,
    pub passive_opens: Option<u64>,
    pub attempt_fails: Option<u64>,
    pub estab_resets: Option<u64>,
    pub curr_estab: Option<u64>,
    pub in_segs: Option<u64>,
    pub out_segs: Option<u64>,
    pub retrans_segs: Option<u64>,
    pub in_errs: Option<u64>,
    pub out_rsts: Option<u64>,
    pub in_csum_errors: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TcpExtStat {
    pub syncookies_sent: Option<u64>,
    pub syncookies_recv: Option<u64>,
    pub syncookies_failed: Option<u64>,
    pub embryonic_rsts: Option<u64>,
    pub prune_called: Option<u64>,
    pub tw: Option<u64>,
    pub paws_estab: Option<u64>,
    pub delayed_acks: Option<u64>,
    pub delayed_ack_locked: Option<u64>,
    pub delayed_ack_lost: Option<u64>,
    pub listen_overflows: Option<u64>,
    pub listen_drops: Option<u64>,
    pub tcp_hp_hits: Option<u64>,
    pub tcp_pure_acks: Option<u64>,
    pub tcp_hp_acks: Option<u64>,
    pub tcp_reno_recovery: Option<u64>,
    pub tcp_reno_reorder: Option<u64>,
    pub tcp_ts_reorder: Option<u64>,
    pub tcp_full_undo: Option<u64>,
    pub tcp_partial_undo: Option<u64>,
    pub tcp_dsack_undo: Option<u64>,
    pub tcp_loss_undo: Option<u64>,
    pub tcp_lost_retransmit: Option<u64>,
    pub tcp_reno_failures: Option<u64>,
    pub tcp_loss_failures: Option<u64>,
    pub tcp_fast_retrans: Option<u64>,
    pub tcp_slow_start_retrans: Option<u64>,
    pub tcp_timeouts: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IpExtStat {
    pub in_mcast_pkts: Option<u64>,
    pub out_mcast_pkts: Option<u64>,
    pub in_bcast_pkts: Option<u64>,
    pub out_bcast_pkts: Option<u64>,
    pub in_octets: Option<u64>,
    pub out_octets: Option<u64>,
    pub in_mcast_octets: Option<u64>,
    pub out_mcast_octets: Option<u64>,
    pub in_bcast_octets: Option<u64>,
    pub out_bcast_octets: Option<u64>,
    pub in_no_ect_pkts: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IpStat {
    pub forwarding: Option<u64>,
    pub in_receives: Option<u64>,
    pub forw_datagrams: Option<u64>,
    pub in_discards: Option<u64>,
    pub in_delivers: Option<u64>,
    pub out_requests: Option<u64>,
    pub out_discards: Option<u64>,
    pub out_no_routes: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Ip6Stat {
    pub in_receives: Option<u64>,
    pub in_hdr_errors: Option<u64>,
    pub in_no_routes: Option<u64>,
    pub in_addr_errors: Option<u64>,
    pub in_discards: Option<u64>,
    pub in_delivers: Option<u64>,
    pub out_forw_datagrams: Option<u64>,
    pub out_requests: Option<u64>,
    pub out_no_routes: Option<u64>,
    pub in_mcast_pkts: Option<u64>,
    pub out_mcast_pkts: Option<u64>,
    pub in_octets: Option<u64>,
    pub out_octets: Option<u64>,
    pub in_mcast_octets: Option<u64>,
    pub out_mcast_octets: Option<u64>,
    pub in_bcast_octets: Option<u64>,
    pub out_bcast_octets: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IcmpStat {
    pub in_msgs: Option<u64>,
    pub in_errors: Option<u64>,
    pub in_dest_unreachs: Option<u64>,
    pub out_msgs: Option<u64>,
    pub out_errors: Option<u64>,
    pub out_dest_unreachs: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Icmp6Stat {
    pub in_msgs: Option<u64>,
    pub in_errors: Option<u64>,
    pub out_msgs: Option<u64>,
    pub out_errors: Option<u64>,
    pub in_dest_unreachs: Option<u64>,
    pub out_dest_unreachs: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct UdpStat {
    pub in_datagrams: Option<u64>,
    pub no_ports: Option<u64>,
    pub in_errors: Option<u64>,
    pub out_datagrams: Option<u64>,
    pub rcvbuf_errors: Option<u64>,
    pub sndbuf_errors: Option<u64>,
    pub ignored_multi: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Udp6Stat {
    pub in_datagrams: Option<u64>,
    pub no_ports: Option<u64>,
    pub in_errors: Option<u64>,
    pub out_datagrams: Option<u64>,
    pub rcvbuf_errors: Option<u64>,
    pub sndbuf_errors: Option<u64>,
    pub in_csum_errors: Option<u64>,
    pub ignored_multi: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VmStat {
    pub pgpgin: Option<u64>,
    pub pgpgout: Option<u64>,
    pub pswpin: Option<u64>,
    pub pswpout: Option<u64>,
    pub pgsteal_kswapd: Option<u64>,
    pub pgsteal_direct: Option<u64>,
    pub pgscan_kswapd: Option<u64>,
    pub pgscan_direct: Option<u64>,
    pub oom_kill: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SlabInfo {
    pub name: Option<String>,
    pub active_objs: Option<u64>,
    pub num_objs: Option<u64>,
    pub obj_size: Option<u64>,
    pub obj_per_slab: Option<u64>,
    pub pages_per_slab: Option<u64>,
    pub active_slabs: Option<u64>,
    pub num_slabs: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Ksm {
    pub advisor_max_cpu: Option<u64>,
    pub advisor_max_pages_to_scan: Option<u64>,
    pub advisor_min_pages_to_scan: Option<u64>,
    pub advisor_mode: Option<String>,
    pub advisor_target_scan_time: Option<u64>,
    pub full_scans: Option<u64>,
    pub general_profit: Option<i64>,
    pub ksm_zero_pages: Option<i64>,
    pub max_page_sharing: Option<u64>,
    pub merge_across_nodes: Option<u64>,
    pub pages_scanned: Option<u64>,
    pub pages_shared: Option<u64>,
    pub pages_sharing: Option<u64>,
    pub pages_skipped: Option<u64>,
    pub pages_to_scan: Option<u64>,
    pub pages_unshared: Option<u64>,
    pub pages_volatile: Option<u64>,
    pub run: Option<u64>,
    pub sleep_millisecs: Option<u64>,
    pub smart_scan: Option<u64>,
    pub stable_node_chains: Option<u64>,
    pub stable_node_chains_prune_millisecs: Option<u64>,
    pub stable_node_dups: Option<u64>,
    pub use_zero_pages: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MountInfo {
    pub mnt_id: Option<i32>,
    pub parent_mnt_id: Option<i32>,
    pub majmin: Option<String>,
    pub root: Option<String>,
    pub mount_point: Option<String>,
    pub mount_options: Option<String>,
    pub fs_type: Option<String>,
    pub mount_source: Option<String>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DiskStat {
    pub major: Option<u64>,
    pub minor: Option<u64>,
    pub name: Option<String>,
    pub is_partition: Option<bool>,
    pub read_completed: Option<u64>,
    pub read_merged: Option<u64>,
    pub read_sectors: Option<u64>,
    pub time_spend_read_ms: Option<u64>,
    pub write_completed: Option<u64>,
    pub write_merged: Option<u64>,
    pub write_sectors: Option<u64>,
    pub time_spend_write_ms: Option<u64>,
    pub discard_completed: Option<u64>,
    pub discard_merged: Option<u64>,
    pub discard_sectors: Option<u64>,
    pub time_spend_discard_ms: Option<u64>,
    pub disk_usage: Option<f32>,
    pub partition_size: Option<u64>,
    pub filesystem_type: Option<String>,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize, Deserialize, Eq, Hash)]
pub enum PidState {
    Running,
    Sleeping,
    UninterruptibleSleep,
    Stopped,
    TracingStopped,
    Zombie,
    Dead,
    Idle,
    Parked,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidStat {
    pub pid: Option<i32>,
    pub comm: Option<String>,
    pub state: Option<PidState>,
    pub ppid: Option<i32>,
    pub pgrp: Option<i32>,
    pub session: Option<i32>,
    pub minflt: Option<u64>,
    pub majflt: Option<u64>,
    pub user_usecs: Option<u64>,
    pub system_usecs: Option<u64>,
    pub num_threads: Option<u64>,
    pub running_secs: Option<u64>,
    pub rss_bytes: Option<u64>,
    pub processor: Option<i32>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidStatus {
    pub ns_tgid: Option<Vec<u32>>,
    pub vm_size: Option<u64>,
    pub lock: Option<u64>,
    pub pin: Option<u64>,
    pub anon: Option<u64>,
    pub file: Option<u64>,
    pub shmem: Option<u64>,
    pub pte: Option<u64>,
    pub swap: Option<u64>,
    pub huge_tlb: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidIo {
    pub rbytes: Option<u64>,
    pub wbytes: Option<u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidInfo {
    pub stat: PidStat,
    pub io: PidIo,
    pub cgroup: String,
    // Optional b/c cmdline may be sanitized or redacted based on security policy
    pub cmdline_vec: Option<Vec<String>>,
    pub exe_path: Option<String>,
    // TODO: Remove alias
    // This field was previously called "mem"
    #[serde(alias = "mem")]
    pub status: PidStatus,
}

pub type PidMap = BTreeMap<i32, PidInfo>;
pub type NetMap = BTreeMap<String, InterfaceStat>;
pub type DiskMap = BTreeMap<String, DiskStat>;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NetStat {
    pub interfaces: Option<NetMap>,
    pub tcp: Option<TcpStat>,
    pub tcp_ext: Option<TcpExtStat>,
    pub ip: Option<IpStat>,
    pub ip_ext: Option<IpExtStat>,
    pub ip6: Option<Ip6Stat>,
    pub icmp: Option<IcmpStat>,
    pub icmp6: Option<Icmp6Stat>,
    pub udp: Option<UdpStat>,
    pub udp6: Option<Udp6Stat>,
}

impl fmt::Display for PidState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PidState::Running => write!(f, "RUNNING"),
            PidState::Sleeping => write!(f, "SLEEPING"),
            PidState::UninterruptibleSleep => write!(f, "UNINTERRUPTIBLE_SLEEP"),
            PidState::Stopped => write!(f, "STOPPED"),
            PidState::TracingStopped => write!(f, "TRACING_STOPPED"),
            PidState::Zombie => write!(f, "ZOMBIE"),
            PidState::Dead => write!(f, "DEAD"),
            PidState::Idle => write!(f, "IDLE"),
            PidState::Parked => write!(f, "PARKED"),
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Sysctl {
    pub kernel_hung_task_detect_count: Option<u64>,
}
