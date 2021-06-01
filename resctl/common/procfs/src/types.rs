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

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CpuStat {
    pub user_usec: Option<i64>,
    pub nice_usec: Option<i64>,
    pub system_usec: Option<i64>,
    pub idle_usec: Option<i64>,
    pub iowait_usec: Option<i64>,
    pub irq_usec: Option<i64>,
    pub softirq_usec: Option<i64>,
    pub stolen_usec: Option<i64>,
    pub guest_usec: Option<i64>,
    pub guest_nice_usec: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stat {
    pub total_cpu: Option<CpuStat>,
    pub cpus: Option<Vec<CpuStat>>,
    pub total_interrupt_count: Option<i64>,
    pub context_switches: Option<i64>,
    pub boot_time_epoch_secs: Option<i64>,
    pub total_processes: Option<i64>,
    pub running_processes: Option<i32>,
    pub blocked_processes: Option<i32>,
}

// In kilobytes unless specified otherwise
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MemInfo {
    pub total: Option<i64>,
    pub free: Option<i64>,
    pub available: Option<i64>,
    pub buffers: Option<i64>,
    pub cached: Option<i64>,
    pub swap_cached: Option<i64>,
    pub active: Option<i64>,
    pub inactive: Option<i64>,
    pub active_anon: Option<i64>,
    pub inactive_anon: Option<i64>,
    pub active_file: Option<i64>,
    pub inactive_file: Option<i64>,
    pub unevictable: Option<i64>,
    pub mlocked: Option<i64>,
    pub swap_total: Option<i64>,
    pub swap_free: Option<i64>,
    pub dirty: Option<i64>,
    pub writeback: Option<i64>,
    pub anon_pages: Option<i64>,
    pub mapped: Option<i64>,
    pub shmem: Option<i64>,
    pub kreclaimable: Option<i64>,
    pub slab: Option<i64>,
    pub slab_reclaimable: Option<i64>,
    pub slab_unreclaimable: Option<i64>,
    pub kernel_stack: Option<i64>,
    pub page_tables: Option<i64>,
    pub anon_huge_pages: Option<i64>,
    pub shmem_huge_pages: Option<i64>,
    pub file_huge_pages: Option<i64>,
    // This is in number of pages: not kilobytes
    pub total_huge_pages: Option<i64>,
    // This is in number of pages: not kilobytes
    pub free_huge_pages: Option<i64>,
    pub huge_page_size: Option<i64>,
    pub cma_total: Option<i64>,
    pub cma_free: Option<i64>,
    pub vmalloc_total: Option<i64>,
    pub vmalloc_used: Option<i64>,
    pub vmalloc_chunk: Option<i64>,
    pub direct_map_4k: Option<i64>,
    pub direct_map_2m: Option<i64>,
    pub direct_map_1g: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct InterfaceStat {
    pub collisions: Option<i64>,
    pub multicast: Option<i64>,
    pub rx_bytes: Option<i64>,
    pub rx_compressed: Option<i64>,
    pub rx_crc_errors: Option<i64>,
    pub rx_dropped: Option<i64>,
    pub rx_errors: Option<i64>,
    pub rx_fifo_errors: Option<i64>,
    pub rx_frame_errors: Option<i64>,
    pub rx_length_errors: Option<i64>,
    pub rx_missed_errors: Option<i64>,
    pub rx_nohandler: Option<i64>,
    pub rx_over_errors: Option<i64>,
    pub rx_packets: Option<i64>,
    pub tx_aborted_errors: Option<i64>,
    pub tx_bytes: Option<i64>,
    pub tx_carrier_errors: Option<i64>,
    pub tx_compressed: Option<i64>,
    pub tx_dropped: Option<i64>,
    pub tx_errors: Option<i64>,
    pub tx_fifo_errors: Option<i64>,
    pub tx_heartbeat_errors: Option<i64>,
    pub tx_packets: Option<i64>,
    pub tx_window_errors: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TcpStat {
    pub active_opens: Option<i64>,
    pub passive_opens: Option<i64>,
    pub attempt_fails: Option<i64>,
    pub estab_resets: Option<i64>,
    pub curr_estab: Option<i64>,
    pub in_segs: Option<i64>,
    pub out_segs: Option<i64>,
    pub retrans_segs: Option<i64>,
    pub in_errs: Option<i64>,
    pub out_rsts: Option<i64>,
    pub in_csum_errors: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TcpExtStat {
    pub syncookies_sent: Option<i64>,
    pub syncookies_recv: Option<i64>,
    pub syncookies_failed: Option<i64>,
    pub embryonic_rsts: Option<i64>,
    pub prune_called: Option<i64>,
    pub tw: Option<i64>,
    pub paws_estab: Option<i64>,
    pub delayed_acks: Option<i64>,
    pub delayed_ack_locked: Option<i64>,
    pub delayed_ack_lost: Option<i64>,
    pub listen_overflows: Option<i64>,
    pub listen_drops: Option<i64>,
    pub tcp_hp_hits: Option<i64>,
    pub tcp_pure_acks: Option<i64>,
    pub tcp_hp_acks: Option<i64>,
    pub tcp_reno_recovery: Option<i64>,
    pub tcp_reno_reorder: Option<i64>,
    pub tcp_ts_reorder: Option<i64>,
    pub tcp_full_undo: Option<i64>,
    pub tcp_partial_undo: Option<i64>,
    pub tcp_dsack_undo: Option<i64>,
    pub tcp_loss_undo: Option<i64>,
    pub tcp_lost_retransmit: Option<i64>,
    pub tcp_reno_failures: Option<i64>,
    pub tcp_loss_failures: Option<i64>,
    pub tcp_fast_retrans: Option<i64>,
    pub tcp_slow_start_retrans: Option<i64>,
    pub tcp_timeouts: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IpExtStat {
    pub in_mcast_pkts: Option<i64>,
    pub out_mcast_pkts: Option<i64>,
    pub in_bcast_pkts: Option<i64>,
    pub out_bcast_pkts: Option<i64>,
    pub in_octets: Option<i64>,
    pub out_octets: Option<i64>,
    pub in_mcast_octets: Option<i64>,
    pub out_mcast_octets: Option<i64>,
    pub in_bcast_octets: Option<i64>,
    pub out_bcast_octets: Option<i64>,
    pub in_no_ect_pkts: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IpStat {
    pub forwarding: Option<i64>,
    pub in_receives: Option<i64>,
    pub forw_datagrams: Option<i64>,
    pub in_discards: Option<i64>,
    pub in_delivers: Option<i64>,
    pub out_requests: Option<i64>,
    pub out_discards: Option<i64>,
    pub out_no_routes: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Ip6Stat {
    pub in_receives: Option<i64>,
    pub in_hdr_errors: Option<i64>,
    pub in_no_routes: Option<i64>,
    pub in_addr_errors: Option<i64>,
    pub in_discards: Option<i64>,
    pub in_delivers: Option<i64>,
    pub out_forw_datagrams: Option<i64>,
    pub out_requests: Option<i64>,
    pub out_no_routes: Option<i64>,
    pub in_mcast_pkts: Option<i64>,
    pub out_mcast_pkts: Option<i64>,
    pub in_octets: Option<i64>,
    pub out_octets: Option<i64>,
    pub in_mcast_octets: Option<i64>,
    pub out_mcast_octets: Option<i64>,
    pub in_bcast_octets: Option<i64>,
    pub out_bcast_octets: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct IcmpStat {
    pub in_msgs: Option<i64>,
    pub in_errors: Option<i64>,
    pub in_dest_unreachs: Option<i64>,
    pub out_msgs: Option<i64>,
    pub out_errors: Option<i64>,
    pub out_dest_unreachs: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Icmp6Stat {
    pub in_msgs: Option<i64>,
    pub in_errors: Option<i64>,
    pub out_msgs: Option<i64>,
    pub out_errors: Option<i64>,
    pub in_dest_unreachs: Option<i64>,
    pub out_dest_unreachs: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct UdpStat {
    pub in_datagrams: Option<i64>,
    pub no_ports: Option<i64>,
    pub in_errors: Option<i64>,
    pub out_datagrams: Option<i64>,
    pub rcvbuf_errors: Option<i64>,
    pub sndbuf_errors: Option<i64>,
    pub ignored_multi: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Udp6Stat {
    pub in_datagrams: Option<i64>,
    pub no_ports: Option<i64>,
    pub in_errors: Option<i64>,
    pub out_datagrams: Option<i64>,
    pub rcvbuf_errors: Option<i64>,
    pub sndbuf_errors: Option<i64>,
    pub in_csum_errors: Option<i64>,
    pub ignored_multi: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VmStat {
    pub pgpgin: Option<i64>,
    pub pgpgout: Option<i64>,
    pub pswpin: Option<i64>,
    pub pswpout: Option<i64>,
    pub pgsteal_kswapd: Option<i64>,
    pub pgsteal_direct: Option<i64>,
    pub pgscan_kswapd: Option<i64>,
    pub pgscan_direct: Option<i64>,
    pub oom_kill: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DiskStat {
    pub major: Option<i64>,
    pub minor: Option<i64>,
    pub name: Option<String>,
    pub read_completed: Option<i64>,
    pub read_merged: Option<i64>,
    pub read_sectors: Option<i64>,
    pub time_spend_read_ms: Option<i64>,
    pub write_completed: Option<i64>,
    pub write_merged: Option<i64>,
    pub write_sectors: Option<i64>,
    pub time_spend_write_ms: Option<i64>,
    pub discard_completed: Option<i64>,
    pub discard_merged: Option<i64>,
    pub discard_sectors: Option<i64>,
    pub time_spend_discard_ms: Option<i64>,
}

#[derive(Clone, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
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
    pub minflt: Option<i64>,
    pub majflt: Option<i64>,
    pub user_usecs: Option<i64>,
    pub system_usecs: Option<i64>,
    pub num_threads: Option<i64>,
    pub running_secs: Option<i64>,
    pub rss_bytes: Option<i64>,
    pub processor: Option<i32>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidMem {
    pub vm_size: Option<i64>,
    pub lock: Option<i64>,
    pub pin: Option<i64>,
    pub anon: Option<i64>,
    pub file: Option<i64>,
    pub shmem: Option<i64>,
    pub pte: Option<i64>,
    pub swap: Option<i64>,
    pub huge_tlb: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidIo {
    pub rbytes: Option<i64>,
    pub wbytes: Option<i64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PidInfo {
    pub stat: PidStat,
    pub io: PidIo,
    pub cgroup: String,
    // Optional b/c cmdline may be sanitized or redacted based on security policy
    pub cmdline_vec: Option<Vec<String>>,
    pub exe_path: Option<String>,
    pub mem: PidMem,
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
