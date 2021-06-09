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

// TODO(T91718069): Remove Thrift structs completely

use crate::types::*;
use std::convert::TryInto;

impl From<procfs_thrift::CpuStat> for CpuStat {
    fn from(cpu_stat: procfs_thrift::CpuStat) -> Self {
        Self {
            user_usec: cpu_stat.user_usec.map(|x| x.try_into().unwrap()),
            nice_usec: cpu_stat.nice_usec.map(|x| x.try_into().unwrap()),
            system_usec: cpu_stat.system_usec.map(|x| x.try_into().unwrap()),
            idle_usec: cpu_stat.idle_usec.map(|x| x.try_into().unwrap()),
            iowait_usec: cpu_stat.iowait_usec.map(|x| x.try_into().unwrap()),
            irq_usec: cpu_stat.irq_usec.map(|x| x.try_into().unwrap()),
            softirq_usec: cpu_stat.softirq_usec.map(|x| x.try_into().unwrap()),
            stolen_usec: cpu_stat.stolen_usec.map(|x| x.try_into().unwrap()),
            guest_usec: cpu_stat.guest_usec.map(|x| x.try_into().unwrap()),
            guest_nice_usec: cpu_stat.guest_nice_usec.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::Stat> for Stat {
    fn from(stat: procfs_thrift::Stat) -> Self {
        Self {
            total_cpu: stat.total_cpu.map(From::from),
            cpus: stat.cpus.map(|v| v.into_iter().map(From::from).collect()),
            total_interrupt_count: stat.total_interrupt_count.map(|x| x.try_into().unwrap()),
            context_switches: stat.context_switches.map(|x| x.try_into().unwrap()),
            boot_time_epoch_secs: stat.boot_time_epoch_secs.map(|x| x.try_into().unwrap()),
            total_processes: stat.total_processes.map(|x| x.try_into().unwrap()),
            running_processes: stat.running_processes.map(|x| x.try_into().unwrap()),
            blocked_processes: stat.blocked_processes.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::MemInfo> for MemInfo {
    fn from(mem_info: procfs_thrift::MemInfo) -> Self {
        Self {
            total: mem_info.total.map(|x| x.try_into().unwrap()),
            free: mem_info.free.map(|x| x.try_into().unwrap()),
            available: mem_info.available.map(|x| x.try_into().unwrap()),
            buffers: mem_info.buffers.map(|x| x.try_into().unwrap()),
            cached: mem_info.cached.map(|x| x.try_into().unwrap()),
            swap_cached: mem_info.swap_cached.map(|x| x.try_into().unwrap()),
            active: mem_info.active.map(|x| x.try_into().unwrap()),
            inactive: mem_info.inactive.map(|x| x.try_into().unwrap()),
            active_anon: mem_info.active_anon.map(|x| x.try_into().unwrap()),
            inactive_anon: mem_info.inactive_anon.map(|x| x.try_into().unwrap()),
            active_file: mem_info.active_file.map(|x| x.try_into().unwrap()),
            inactive_file: mem_info.inactive_file.map(|x| x.try_into().unwrap()),
            unevictable: mem_info.unevictable.map(|x| x.try_into().unwrap()),
            mlocked: mem_info.mlocked.map(|x| x.try_into().unwrap()),
            swap_total: mem_info.swap_total.map(|x| x.try_into().unwrap()),
            swap_free: mem_info.swap_free.map(|x| x.try_into().unwrap()),
            dirty: mem_info.dirty.map(|x| x.try_into().unwrap()),
            writeback: mem_info.writeback.map(|x| x.try_into().unwrap()),
            anon_pages: mem_info.anon_pages.map(|x| x.try_into().unwrap()),
            mapped: mem_info.mapped.map(|x| x.try_into().unwrap()),
            shmem: mem_info.shmem.map(|x| x.try_into().unwrap()),
            kreclaimable: mem_info.kreclaimable.map(|x| x.try_into().unwrap()),
            slab: mem_info.slab.map(|x| x.try_into().unwrap()),
            slab_reclaimable: mem_info.slab_reclaimable.map(|x| x.try_into().unwrap()),
            slab_unreclaimable: mem_info.slab_unreclaimable.map(|x| x.try_into().unwrap()),
            kernel_stack: mem_info.kernel_stack.map(|x| x.try_into().unwrap()),
            page_tables: mem_info.page_tables.map(|x| x.try_into().unwrap()),
            anon_huge_pages: mem_info.anon_huge_pages.map(|x| x.try_into().unwrap()),
            shmem_huge_pages: mem_info.shmem_huge_pages.map(|x| x.try_into().unwrap()),
            file_huge_pages: mem_info.file_huge_pages.map(|x| x.try_into().unwrap()),
            total_huge_pages: mem_info.total_huge_pages.map(|x| x.try_into().unwrap()),
            free_huge_pages: mem_info.free_huge_pages.map(|x| x.try_into().unwrap()),
            huge_page_size: mem_info.huge_page_size.map(|x| x.try_into().unwrap()),
            cma_total: mem_info.cma_total.map(|x| x.try_into().unwrap()),
            cma_free: mem_info.cma_free.map(|x| x.try_into().unwrap()),
            vmalloc_total: mem_info.vmalloc_total.map(|x| x.try_into().unwrap()),
            vmalloc_used: mem_info.vmalloc_used.map(|x| x.try_into().unwrap()),
            vmalloc_chunk: mem_info.vmalloc_chunk.map(|x| x.try_into().unwrap()),
            direct_map_4k: mem_info.direct_map_4k.map(|x| x.try_into().unwrap()),
            direct_map_2m: mem_info.direct_map_2m.map(|x| x.try_into().unwrap()),
            direct_map_1g: mem_info.direct_map_1g.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::InterfaceStat> for InterfaceStat {
    fn from(interface_stat: procfs_thrift::InterfaceStat) -> Self {
        Self {
            collisions: interface_stat.collisions.map(|x| x.try_into().unwrap()),
            multicast: interface_stat.multicast.map(|x| x.try_into().unwrap()),
            rx_bytes: interface_stat.rx_bytes.map(|x| x.try_into().unwrap()),
            rx_compressed: interface_stat.rx_compressed.map(|x| x.try_into().unwrap()),
            rx_crc_errors: interface_stat.rx_crc_errors.map(|x| x.try_into().unwrap()),
            rx_dropped: interface_stat.rx_dropped.map(|x| x.try_into().unwrap()),
            rx_errors: interface_stat.rx_errors.map(|x| x.try_into().unwrap()),
            rx_fifo_errors: interface_stat.rx_fifo_errors.map(|x| x.try_into().unwrap()),
            rx_frame_errors: interface_stat
                .rx_frame_errors
                .map(|x| x.try_into().unwrap()),
            rx_length_errors: interface_stat
                .rx_length_errors
                .map(|x| x.try_into().unwrap()),
            rx_missed_errors: interface_stat
                .rx_missed_errors
                .map(|x| x.try_into().unwrap()),
            rx_nohandler: interface_stat.rx_nohandler.map(|x| x.try_into().unwrap()),
            rx_over_errors: interface_stat.rx_over_errors.map(|x| x.try_into().unwrap()),
            rx_packets: interface_stat.rx_packets.map(|x| x.try_into().unwrap()),
            tx_aborted_errors: interface_stat
                .tx_aborted_errors
                .map(|x| x.try_into().unwrap()),
            tx_bytes: interface_stat.tx_bytes.map(|x| x.try_into().unwrap()),
            tx_carrier_errors: interface_stat
                .tx_carrier_errors
                .map(|x| x.try_into().unwrap()),
            tx_compressed: interface_stat.tx_compressed.map(|x| x.try_into().unwrap()),
            tx_dropped: interface_stat.tx_dropped.map(|x| x.try_into().unwrap()),
            tx_errors: interface_stat.tx_errors.map(|x| x.try_into().unwrap()),
            tx_fifo_errors: interface_stat.tx_fifo_errors.map(|x| x.try_into().unwrap()),
            tx_heartbeat_errors: interface_stat
                .tx_heartbeat_errors
                .map(|x| x.try_into().unwrap()),
            tx_packets: interface_stat.tx_packets.map(|x| x.try_into().unwrap()),
            tx_window_errors: interface_stat
                .tx_window_errors
                .map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::TcpStat> for TcpStat {
    fn from(tcp_stat: procfs_thrift::TcpStat) -> Self {
        Self {
            active_opens: tcp_stat.active_opens.map(|x| x.try_into().unwrap()),
            passive_opens: tcp_stat.passive_opens.map(|x| x.try_into().unwrap()),
            attempt_fails: tcp_stat.attempt_fails.map(|x| x.try_into().unwrap()),
            estab_resets: tcp_stat.estab_resets.map(|x| x.try_into().unwrap()),
            curr_estab: tcp_stat.curr_estab.map(|x| x.try_into().unwrap()),
            in_segs: tcp_stat.in_segs.map(|x| x.try_into().unwrap()),
            out_segs: tcp_stat.out_segs.map(|x| x.try_into().unwrap()),
            retrans_segs: tcp_stat.retrans_segs.map(|x| x.try_into().unwrap()),
            in_errs: tcp_stat.in_errs.map(|x| x.try_into().unwrap()),
            out_rsts: tcp_stat.out_rsts.map(|x| x.try_into().unwrap()),
            in_csum_errors: tcp_stat.in_csum_errors.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::TcpExtStat> for TcpExtStat {
    fn from(tcp_ext_stat: procfs_thrift::TcpExtStat) -> Self {
        Self {
            syncookies_sent: tcp_ext_stat.syncookies_sent.map(|x| x.try_into().unwrap()),
            syncookies_recv: tcp_ext_stat.syncookies_recv.map(|x| x.try_into().unwrap()),
            syncookies_failed: tcp_ext_stat
                .syncookies_failed
                .map(|x| x.try_into().unwrap()),
            embryonic_rsts: tcp_ext_stat.embryonic_rsts.map(|x| x.try_into().unwrap()),
            prune_called: tcp_ext_stat.prune_called.map(|x| x.try_into().unwrap()),
            tw: tcp_ext_stat.tw.map(|x| x.try_into().unwrap()),
            paws_estab: tcp_ext_stat.paws_estab.map(|x| x.try_into().unwrap()),
            delayed_acks: tcp_ext_stat.delayed_acks.map(|x| x.try_into().unwrap()),
            delayed_ack_locked: tcp_ext_stat
                .delayed_ack_locked
                .map(|x| x.try_into().unwrap()),
            delayed_ack_lost: tcp_ext_stat.delayed_ack_lost.map(|x| x.try_into().unwrap()),
            listen_overflows: tcp_ext_stat.listen_overflows.map(|x| x.try_into().unwrap()),
            listen_drops: tcp_ext_stat.listen_drops.map(|x| x.try_into().unwrap()),
            tcp_hp_hits: tcp_ext_stat.tcp_hp_hits.map(|x| x.try_into().unwrap()),
            tcp_pure_acks: tcp_ext_stat.tcp_pure_acks.map(|x| x.try_into().unwrap()),
            tcp_hp_acks: tcp_ext_stat.tcp_hp_acks.map(|x| x.try_into().unwrap()),
            tcp_reno_recovery: tcp_ext_stat
                .tcp_reno_recovery
                .map(|x| x.try_into().unwrap()),
            tcp_reno_reorder: tcp_ext_stat.tcp_reno_reorder.map(|x| x.try_into().unwrap()),
            tcp_ts_reorder: tcp_ext_stat.tcp_ts_reorder.map(|x| x.try_into().unwrap()),
            tcp_full_undo: tcp_ext_stat.tcp_full_undo.map(|x| x.try_into().unwrap()),
            tcp_partial_undo: tcp_ext_stat.tcp_partial_undo.map(|x| x.try_into().unwrap()),
            tcp_dsack_undo: tcp_ext_stat.tcp_dsack_undo.map(|x| x.try_into().unwrap()),
            tcp_loss_undo: tcp_ext_stat.tcp_loss_undo.map(|x| x.try_into().unwrap()),
            tcp_lost_retransmit: tcp_ext_stat
                .tcp_lost_retransmit
                .map(|x| x.try_into().unwrap()),
            tcp_reno_failures: tcp_ext_stat
                .tcp_reno_failures
                .map(|x| x.try_into().unwrap()),
            tcp_loss_failures: tcp_ext_stat
                .tcp_loss_failures
                .map(|x| x.try_into().unwrap()),
            tcp_fast_retrans: tcp_ext_stat.tcp_fast_retrans.map(|x| x.try_into().unwrap()),
            tcp_slow_start_retrans: tcp_ext_stat
                .tcp_slow_start_retrans
                .map(|x| x.try_into().unwrap()),
            tcp_timeouts: tcp_ext_stat.tcp_timeouts.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::IpExtStat> for IpExtStat {
    fn from(ip_ext_stat: procfs_thrift::IpExtStat) -> Self {
        Self {
            in_mcast_pkts: ip_ext_stat.in_mcast_pkts.map(|x| x.try_into().unwrap()),
            out_mcast_pkts: ip_ext_stat.out_mcast_pkts.map(|x| x.try_into().unwrap()),
            in_bcast_pkts: ip_ext_stat.in_bcast_pkts.map(|x| x.try_into().unwrap()),
            out_bcast_pkts: ip_ext_stat.out_bcast_pkts.map(|x| x.try_into().unwrap()),
            in_octets: ip_ext_stat.in_octets.map(|x| x.try_into().unwrap()),
            out_octets: ip_ext_stat.out_octets.map(|x| x.try_into().unwrap()),
            in_mcast_octets: ip_ext_stat.in_mcast_octets.map(|x| x.try_into().unwrap()),
            out_mcast_octets: ip_ext_stat.out_mcast_octets.map(|x| x.try_into().unwrap()),
            in_bcast_octets: ip_ext_stat.in_bcast_octets.map(|x| x.try_into().unwrap()),
            out_bcast_octets: ip_ext_stat.out_bcast_octets.map(|x| x.try_into().unwrap()),
            in_no_ect_pkts: ip_ext_stat.in_no_ect_pkts.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::IpStat> for IpStat {
    fn from(ip_stat: procfs_thrift::IpStat) -> Self {
        Self {
            forwarding: ip_stat.forwarding.map(|x| x.try_into().unwrap()),
            in_receives: ip_stat.in_receives.map(|x| x.try_into().unwrap()),
            forw_datagrams: ip_stat.forw_datagrams.map(|x| x.try_into().unwrap()),
            in_discards: ip_stat.in_discards.map(|x| x.try_into().unwrap()),
            in_delivers: ip_stat.in_delivers.map(|x| x.try_into().unwrap()),
            out_requests: ip_stat.out_requests.map(|x| x.try_into().unwrap()),
            out_discards: ip_stat.out_discards.map(|x| x.try_into().unwrap()),
            out_no_routes: ip_stat.out_no_routes.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::Ip6Stat> for Ip6Stat {
    fn from(ip6_stat: procfs_thrift::Ip6Stat) -> Self {
        Self {
            in_receives: ip6_stat.in_receives.map(|x| x.try_into().unwrap()),
            in_hdr_errors: ip6_stat.in_hdr_errors.map(|x| x.try_into().unwrap()),
            in_no_routes: ip6_stat.in_no_routes.map(|x| x.try_into().unwrap()),
            in_addr_errors: ip6_stat.in_addr_errors.map(|x| x.try_into().unwrap()),
            in_discards: ip6_stat.in_discards.map(|x| x.try_into().unwrap()),
            in_delivers: ip6_stat.in_delivers.map(|x| x.try_into().unwrap()),
            out_forw_datagrams: ip6_stat.out_forw_datagrams.map(|x| x.try_into().unwrap()),
            out_requests: ip6_stat.out_requests.map(|x| x.try_into().unwrap()),
            out_no_routes: ip6_stat.out_no_routes.map(|x| x.try_into().unwrap()),
            in_mcast_pkts: ip6_stat.in_mcast_pkts.map(|x| x.try_into().unwrap()),
            out_mcast_pkts: ip6_stat.out_mcast_pkts.map(|x| x.try_into().unwrap()),
            in_octets: ip6_stat.in_octets.map(|x| x.try_into().unwrap()),
            out_octets: ip6_stat.out_octets.map(|x| x.try_into().unwrap()),
            in_mcast_octets: ip6_stat.in_mcast_octets.map(|x| x.try_into().unwrap()),
            out_mcast_octets: ip6_stat.out_mcast_octets.map(|x| x.try_into().unwrap()),
            in_bcast_octets: ip6_stat.in_bcast_octets.map(|x| x.try_into().unwrap()),
            out_bcast_octets: ip6_stat.out_bcast_octets.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::IcmpStat> for IcmpStat {
    fn from(icmp_stat: procfs_thrift::IcmpStat) -> Self {
        Self {
            in_msgs: icmp_stat.in_msgs.map(|x| x.try_into().unwrap()),
            in_errors: icmp_stat.in_errors.map(|x| x.try_into().unwrap()),
            in_dest_unreachs: icmp_stat.in_dest_unreachs.map(|x| x.try_into().unwrap()),
            out_msgs: icmp_stat.out_msgs.map(|x| x.try_into().unwrap()),
            out_errors: icmp_stat.out_errors.map(|x| x.try_into().unwrap()),
            out_dest_unreachs: icmp_stat.out_dest_unreachs.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::Icmp6Stat> for Icmp6Stat {
    fn from(icmp6_stat: procfs_thrift::Icmp6Stat) -> Self {
        Self {
            in_msgs: icmp6_stat.in_msgs.map(|x| x.try_into().unwrap()),
            in_errors: icmp6_stat.in_errors.map(|x| x.try_into().unwrap()),
            out_msgs: icmp6_stat.out_msgs.map(|x| x.try_into().unwrap()),
            out_errors: icmp6_stat.out_errors.map(|x| x.try_into().unwrap()),
            in_dest_unreachs: icmp6_stat.in_dest_unreachs.map(|x| x.try_into().unwrap()),
            out_dest_unreachs: icmp6_stat.out_dest_unreachs.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::UdpStat> for UdpStat {
    fn from(udp_stat: procfs_thrift::UdpStat) -> Self {
        Self {
            in_datagrams: udp_stat.in_datagrams.map(|x| x.try_into().unwrap()),
            no_ports: udp_stat.no_ports.map(|x| x.try_into().unwrap()),
            in_errors: udp_stat.in_errors.map(|x| x.try_into().unwrap()),
            out_datagrams: udp_stat.out_datagrams.map(|x| x.try_into().unwrap()),
            rcvbuf_errors: udp_stat.rcvbuf_errors.map(|x| x.try_into().unwrap()),
            sndbuf_errors: udp_stat.sndbuf_errors.map(|x| x.try_into().unwrap()),
            ignored_multi: udp_stat.ignored_multi.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::Udp6Stat> for Udp6Stat {
    fn from(udp6_stat: procfs_thrift::Udp6Stat) -> Self {
        Self {
            in_datagrams: udp6_stat.in_datagrams.map(|x| x.try_into().unwrap()),
            no_ports: udp6_stat.no_ports.map(|x| x.try_into().unwrap()),
            in_errors: udp6_stat.in_errors.map(|x| x.try_into().unwrap()),
            out_datagrams: udp6_stat.out_datagrams.map(|x| x.try_into().unwrap()),
            rcvbuf_errors: udp6_stat.rcvbuf_errors.map(|x| x.try_into().unwrap()),
            sndbuf_errors: udp6_stat.sndbuf_errors.map(|x| x.try_into().unwrap()),
            in_csum_errors: udp6_stat.in_csum_errors.map(|x| x.try_into().unwrap()),
            ignored_multi: udp6_stat.ignored_multi.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::VmStat> for VmStat {
    fn from(vm_stat: procfs_thrift::VmStat) -> Self {
        Self {
            pgpgin: vm_stat.pgpgin.map(|x| x.try_into().unwrap()),
            pgpgout: vm_stat.pgpgout.map(|x| x.try_into().unwrap()),
            pswpin: vm_stat.pswpin.map(|x| x.try_into().unwrap()),
            pswpout: vm_stat.pswpout.map(|x| x.try_into().unwrap()),
            pgsteal_kswapd: vm_stat.pgsteal_kswapd.map(|x| x.try_into().unwrap()),
            pgsteal_direct: vm_stat.pgsteal_direct.map(|x| x.try_into().unwrap()),
            pgscan_kswapd: vm_stat.pgscan_kswapd.map(|x| x.try_into().unwrap()),
            pgscan_direct: vm_stat.pgscan_direct.map(|x| x.try_into().unwrap()),
            oom_kill: vm_stat.oom_kill.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::DiskStat> for DiskStat {
    fn from(disk_stat: procfs_thrift::DiskStat) -> Self {
        Self {
            major: disk_stat.major.map(|x| x.try_into().unwrap()),
            minor: disk_stat.minor.map(|x| x.try_into().unwrap()),
            name: disk_stat.name,
            read_completed: disk_stat.read_completed.map(|x| x.try_into().unwrap()),
            read_merged: disk_stat.read_merged.map(|x| x.try_into().unwrap()),
            read_sectors: disk_stat.read_sectors.map(|x| x.try_into().unwrap()),
            time_spend_read_ms: disk_stat.time_spend_read_ms.map(|x| x.try_into().unwrap()),
            write_completed: disk_stat.write_completed.map(|x| x.try_into().unwrap()),
            write_merged: disk_stat.write_merged.map(|x| x.try_into().unwrap()),
            write_sectors: disk_stat.write_sectors.map(|x| x.try_into().unwrap()),
            time_spend_write_ms: disk_stat.time_spend_write_ms.map(|x| x.try_into().unwrap()),
            discard_completed: disk_stat.discard_completed.map(|x| x.try_into().unwrap()),
            discard_merged: disk_stat.discard_merged.map(|x| x.try_into().unwrap()),
            discard_sectors: disk_stat.discard_sectors.map(|x| x.try_into().unwrap()),
            time_spend_discard_ms: disk_stat
                .time_spend_discard_ms
                .map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::PidState> for PidState {
    fn from(pid_state: procfs_thrift::PidState) -> Self {
        match pid_state {
            procfs_thrift::PidState::RUNNING => Self::Running,
            procfs_thrift::PidState::SLEEPING => Self::Sleeping,
            procfs_thrift::PidState::UNINTERRUPTIBLE_SLEEP => Self::UninterruptibleSleep,
            procfs_thrift::PidState::STOPPED => Self::Stopped,
            procfs_thrift::PidState::TRACING_STOPPED => Self::TracingStopped,
            procfs_thrift::PidState::ZOMBIE => Self::Zombie,
            procfs_thrift::PidState::DEAD => Self::Dead,
            procfs_thrift::PidState::IDLE => Self::Idle,
            procfs_thrift::PidState::PARKED => Self::Parked,
            _ => panic!("Invalid PidState"),
        }
    }
}

impl From<procfs_thrift::PidStat> for PidStat {
    fn from(pid_stat: procfs_thrift::PidStat) -> Self {
        Self {
            pid: pid_stat.pid,
            comm: pid_stat.comm,
            state: pid_stat.state.map(From::from),
            ppid: pid_stat.ppid,
            pgrp: pid_stat.pgrp,
            session: pid_stat.session,
            minflt: pid_stat.minflt.map(|x| x.try_into().unwrap()),
            majflt: pid_stat.majflt.map(|x| x.try_into().unwrap()),
            user_usecs: pid_stat.user_usecs.map(|x| x.try_into().unwrap()),
            system_usecs: pid_stat.system_usecs.map(|x| x.try_into().unwrap()),
            num_threads: pid_stat.num_threads.map(|x| x.try_into().unwrap()),
            running_secs: pid_stat.running_secs.map(|x| x.try_into().unwrap()),
            rss_bytes: pid_stat.rss_bytes.map(|x| x.try_into().unwrap()),
            processor: pid_stat.processor,
        }
    }
}

impl From<procfs_thrift::PidMem> for PidMem {
    fn from(pid_mem: procfs_thrift::PidMem) -> Self {
        Self {
            vm_size: pid_mem.vm_size.map(|x| x.try_into().unwrap()),
            lock: pid_mem.lock.map(|x| x.try_into().unwrap()),
            pin: pid_mem.pin.map(|x| x.try_into().unwrap()),
            anon: pid_mem.anon.map(|x| x.try_into().unwrap()),
            file: pid_mem.file.map(|x| x.try_into().unwrap()),
            shmem: pid_mem.shmem.map(|x| x.try_into().unwrap()),
            pte: pid_mem.pte.map(|x| x.try_into().unwrap()),
            swap: pid_mem.swap.map(|x| x.try_into().unwrap()),
            huge_tlb: pid_mem.huge_tlb.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::PidIo> for PidIo {
    fn from(pid_io: procfs_thrift::PidIo) -> Self {
        Self {
            rbytes: pid_io.rbytes.map(|x| x.try_into().unwrap()),
            wbytes: pid_io.wbytes.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<procfs_thrift::PidInfo> for PidInfo {
    fn from(pid_info: procfs_thrift::PidInfo) -> Self {
        Self {
            stat: pid_info.stat.into(),
            io: pid_info.io.into(),
            cgroup: pid_info.cgroup,
            cmdline_vec: pid_info.cmdline_vec,
            exe_path: pid_info.exe_path,
            mem: pid_info.mem.into(),
        }
    }
}

impl From<procfs_thrift::NetStat> for NetStat {
    fn from(net_stat: procfs_thrift::NetStat) -> Self {
        Self {
            interfaces: net_stat
                .interfaces
                .map(|m| m.into_iter().map(|(k, v)| (k, v.into())).collect()),
            tcp: net_stat.tcp.map(From::from),
            tcp_ext: net_stat.tcp_ext.map(From::from),
            ip: net_stat.ip.map(From::from),
            ip_ext: net_stat.ip_ext.map(From::from),
            ip6: net_stat.ip6.map(From::from),
            icmp: net_stat.icmp.map(From::from),
            icmp6: net_stat.icmp6.map(From::from),
            udp: net_stat.udp.map(From::from),
            udp6: net_stat.udp6.map(From::from),
        }
    }
}

impl From<CpuStat> for procfs_thrift::CpuStat {
    fn from(cpu_stat: CpuStat) -> Self {
        Self {
            user_usec: cpu_stat.user_usec.map(|x| x.try_into().unwrap()),
            nice_usec: cpu_stat.nice_usec.map(|x| x.try_into().unwrap()),
            system_usec: cpu_stat.system_usec.map(|x| x.try_into().unwrap()),
            idle_usec: cpu_stat.idle_usec.map(|x| x.try_into().unwrap()),
            iowait_usec: cpu_stat.iowait_usec.map(|x| x.try_into().unwrap()),
            irq_usec: cpu_stat.irq_usec.map(|x| x.try_into().unwrap()),
            softirq_usec: cpu_stat.softirq_usec.map(|x| x.try_into().unwrap()),
            stolen_usec: cpu_stat.stolen_usec.map(|x| x.try_into().unwrap()),
            guest_usec: cpu_stat.guest_usec.map(|x| x.try_into().unwrap()),
            guest_nice_usec: cpu_stat.guest_nice_usec.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<Stat> for procfs_thrift::Stat {
    fn from(stat: Stat) -> Self {
        Self {
            total_cpu: stat.total_cpu.map(From::from),
            cpus: stat.cpus.map(|v| v.into_iter().map(From::from).collect()),
            total_interrupt_count: stat.total_interrupt_count.map(|x| x.try_into().unwrap()),
            context_switches: stat.context_switches.map(|x| x.try_into().unwrap()),
            boot_time_epoch_secs: stat.boot_time_epoch_secs.map(|x| x.try_into().unwrap()),
            total_processes: stat.total_processes.map(|x| x.try_into().unwrap()),
            running_processes: stat.running_processes.map(|x| x.try_into().unwrap()),
            blocked_processes: stat.blocked_processes.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<MemInfo> for procfs_thrift::MemInfo {
    fn from(mem_info: MemInfo) -> Self {
        Self {
            total: mem_info.total.map(|x| x.try_into().unwrap()),
            free: mem_info.free.map(|x| x.try_into().unwrap()),
            available: mem_info.available.map(|x| x.try_into().unwrap()),
            buffers: mem_info.buffers.map(|x| x.try_into().unwrap()),
            cached: mem_info.cached.map(|x| x.try_into().unwrap()),
            swap_cached: mem_info.swap_cached.map(|x| x.try_into().unwrap()),
            active: mem_info.active.map(|x| x.try_into().unwrap()),
            inactive: mem_info.inactive.map(|x| x.try_into().unwrap()),
            active_anon: mem_info.active_anon.map(|x| x.try_into().unwrap()),
            inactive_anon: mem_info.inactive_anon.map(|x| x.try_into().unwrap()),
            active_file: mem_info.active_file.map(|x| x.try_into().unwrap()),
            inactive_file: mem_info.inactive_file.map(|x| x.try_into().unwrap()),
            unevictable: mem_info.unevictable.map(|x| x.try_into().unwrap()),
            mlocked: mem_info.mlocked.map(|x| x.try_into().unwrap()),
            swap_total: mem_info.swap_total.map(|x| x.try_into().unwrap()),
            swap_free: mem_info.swap_free.map(|x| x.try_into().unwrap()),
            dirty: mem_info.dirty.map(|x| x.try_into().unwrap()),
            writeback: mem_info.writeback.map(|x| x.try_into().unwrap()),
            anon_pages: mem_info.anon_pages.map(|x| x.try_into().unwrap()),
            mapped: mem_info.mapped.map(|x| x.try_into().unwrap()),
            shmem: mem_info.shmem.map(|x| x.try_into().unwrap()),
            kreclaimable: mem_info.kreclaimable.map(|x| x.try_into().unwrap()),
            slab: mem_info.slab.map(|x| x.try_into().unwrap()),
            slab_reclaimable: mem_info.slab_reclaimable.map(|x| x.try_into().unwrap()),
            slab_unreclaimable: mem_info.slab_unreclaimable.map(|x| x.try_into().unwrap()),
            kernel_stack: mem_info.kernel_stack.map(|x| x.try_into().unwrap()),
            page_tables: mem_info.page_tables.map(|x| x.try_into().unwrap()),
            anon_huge_pages: mem_info.anon_huge_pages.map(|x| x.try_into().unwrap()),
            shmem_huge_pages: mem_info.shmem_huge_pages.map(|x| x.try_into().unwrap()),
            file_huge_pages: mem_info.file_huge_pages.map(|x| x.try_into().unwrap()),
            total_huge_pages: mem_info.total_huge_pages.map(|x| x.try_into().unwrap()),
            free_huge_pages: mem_info.free_huge_pages.map(|x| x.try_into().unwrap()),
            huge_page_size: mem_info.huge_page_size.map(|x| x.try_into().unwrap()),
            cma_total: mem_info.cma_total.map(|x| x.try_into().unwrap()),
            cma_free: mem_info.cma_free.map(|x| x.try_into().unwrap()),
            vmalloc_total: mem_info.vmalloc_total.map(|x| x.try_into().unwrap()),
            vmalloc_used: mem_info.vmalloc_used.map(|x| x.try_into().unwrap()),
            vmalloc_chunk: mem_info.vmalloc_chunk.map(|x| x.try_into().unwrap()),
            direct_map_4k: mem_info.direct_map_4k.map(|x| x.try_into().unwrap()),
            direct_map_2m: mem_info.direct_map_2m.map(|x| x.try_into().unwrap()),
            direct_map_1g: mem_info.direct_map_1g.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<InterfaceStat> for procfs_thrift::InterfaceStat {
    fn from(interface_stat: InterfaceStat) -> Self {
        Self {
            collisions: interface_stat.collisions.map(|x| x.try_into().unwrap()),
            multicast: interface_stat.multicast.map(|x| x.try_into().unwrap()),
            rx_bytes: interface_stat.rx_bytes.map(|x| x.try_into().unwrap()),
            rx_compressed: interface_stat.rx_compressed.map(|x| x.try_into().unwrap()),
            rx_crc_errors: interface_stat.rx_crc_errors.map(|x| x.try_into().unwrap()),
            rx_dropped: interface_stat.rx_dropped.map(|x| x.try_into().unwrap()),
            rx_errors: interface_stat.rx_errors.map(|x| x.try_into().unwrap()),
            rx_fifo_errors: interface_stat.rx_fifo_errors.map(|x| x.try_into().unwrap()),
            rx_frame_errors: interface_stat
                .rx_frame_errors
                .map(|x| x.try_into().unwrap()),
            rx_length_errors: interface_stat
                .rx_length_errors
                .map(|x| x.try_into().unwrap()),
            rx_missed_errors: interface_stat
                .rx_missed_errors
                .map(|x| x.try_into().unwrap()),
            rx_nohandler: interface_stat.rx_nohandler.map(|x| x.try_into().unwrap()),
            rx_over_errors: interface_stat.rx_over_errors.map(|x| x.try_into().unwrap()),
            rx_packets: interface_stat.rx_packets.map(|x| x.try_into().unwrap()),
            tx_aborted_errors: interface_stat
                .tx_aborted_errors
                .map(|x| x.try_into().unwrap()),
            tx_bytes: interface_stat.tx_bytes.map(|x| x.try_into().unwrap()),
            tx_carrier_errors: interface_stat
                .tx_carrier_errors
                .map(|x| x.try_into().unwrap()),
            tx_compressed: interface_stat.tx_compressed.map(|x| x.try_into().unwrap()),
            tx_dropped: interface_stat.tx_dropped.map(|x| x.try_into().unwrap()),
            tx_errors: interface_stat.tx_errors.map(|x| x.try_into().unwrap()),
            tx_fifo_errors: interface_stat.tx_fifo_errors.map(|x| x.try_into().unwrap()),
            tx_heartbeat_errors: interface_stat
                .tx_heartbeat_errors
                .map(|x| x.try_into().unwrap()),
            tx_packets: interface_stat.tx_packets.map(|x| x.try_into().unwrap()),
            tx_window_errors: interface_stat
                .tx_window_errors
                .map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<TcpStat> for procfs_thrift::TcpStat {
    fn from(tcp_stat: TcpStat) -> Self {
        Self {
            active_opens: tcp_stat.active_opens.map(|x| x.try_into().unwrap()),
            passive_opens: tcp_stat.passive_opens.map(|x| x.try_into().unwrap()),
            attempt_fails: tcp_stat.attempt_fails.map(|x| x.try_into().unwrap()),
            estab_resets: tcp_stat.estab_resets.map(|x| x.try_into().unwrap()),
            curr_estab: tcp_stat.curr_estab.map(|x| x.try_into().unwrap()),
            in_segs: tcp_stat.in_segs.map(|x| x.try_into().unwrap()),
            out_segs: tcp_stat.out_segs.map(|x| x.try_into().unwrap()),
            retrans_segs: tcp_stat.retrans_segs.map(|x| x.try_into().unwrap()),
            in_errs: tcp_stat.in_errs.map(|x| x.try_into().unwrap()),
            out_rsts: tcp_stat.out_rsts.map(|x| x.try_into().unwrap()),
            in_csum_errors: tcp_stat.in_csum_errors.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<TcpExtStat> for procfs_thrift::TcpExtStat {
    fn from(tcp_ext_stat: TcpExtStat) -> Self {
        Self {
            syncookies_sent: tcp_ext_stat.syncookies_sent.map(|x| x.try_into().unwrap()),
            syncookies_recv: tcp_ext_stat.syncookies_recv.map(|x| x.try_into().unwrap()),
            syncookies_failed: tcp_ext_stat
                .syncookies_failed
                .map(|x| x.try_into().unwrap()),
            embryonic_rsts: tcp_ext_stat.embryonic_rsts.map(|x| x.try_into().unwrap()),
            prune_called: tcp_ext_stat.prune_called.map(|x| x.try_into().unwrap()),
            tw: tcp_ext_stat.tw.map(|x| x.try_into().unwrap()),
            paws_estab: tcp_ext_stat.paws_estab.map(|x| x.try_into().unwrap()),
            delayed_acks: tcp_ext_stat.delayed_acks.map(|x| x.try_into().unwrap()),
            delayed_ack_locked: tcp_ext_stat
                .delayed_ack_locked
                .map(|x| x.try_into().unwrap()),
            delayed_ack_lost: tcp_ext_stat.delayed_ack_lost.map(|x| x.try_into().unwrap()),
            listen_overflows: tcp_ext_stat.listen_overflows.map(|x| x.try_into().unwrap()),
            listen_drops: tcp_ext_stat.listen_drops.map(|x| x.try_into().unwrap()),
            tcp_hp_hits: tcp_ext_stat.tcp_hp_hits.map(|x| x.try_into().unwrap()),
            tcp_pure_acks: tcp_ext_stat.tcp_pure_acks.map(|x| x.try_into().unwrap()),
            tcp_hp_acks: tcp_ext_stat.tcp_hp_acks.map(|x| x.try_into().unwrap()),
            tcp_reno_recovery: tcp_ext_stat
                .tcp_reno_recovery
                .map(|x| x.try_into().unwrap()),
            tcp_reno_reorder: tcp_ext_stat.tcp_reno_reorder.map(|x| x.try_into().unwrap()),
            tcp_ts_reorder: tcp_ext_stat.tcp_ts_reorder.map(|x| x.try_into().unwrap()),
            tcp_full_undo: tcp_ext_stat.tcp_full_undo.map(|x| x.try_into().unwrap()),
            tcp_partial_undo: tcp_ext_stat.tcp_partial_undo.map(|x| x.try_into().unwrap()),
            tcp_dsack_undo: tcp_ext_stat.tcp_dsack_undo.map(|x| x.try_into().unwrap()),
            tcp_loss_undo: tcp_ext_stat.tcp_loss_undo.map(|x| x.try_into().unwrap()),
            tcp_lost_retransmit: tcp_ext_stat
                .tcp_lost_retransmit
                .map(|x| x.try_into().unwrap()),
            tcp_reno_failures: tcp_ext_stat
                .tcp_reno_failures
                .map(|x| x.try_into().unwrap()),
            tcp_loss_failures: tcp_ext_stat
                .tcp_loss_failures
                .map(|x| x.try_into().unwrap()),
            tcp_fast_retrans: tcp_ext_stat.tcp_fast_retrans.map(|x| x.try_into().unwrap()),
            tcp_slow_start_retrans: tcp_ext_stat
                .tcp_slow_start_retrans
                .map(|x| x.try_into().unwrap()),
            tcp_timeouts: tcp_ext_stat.tcp_timeouts.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<IpExtStat> for procfs_thrift::IpExtStat {
    fn from(ip_ext_stat: IpExtStat) -> Self {
        Self {
            in_mcast_pkts: ip_ext_stat.in_mcast_pkts.map(|x| x.try_into().unwrap()),
            out_mcast_pkts: ip_ext_stat.out_mcast_pkts.map(|x| x.try_into().unwrap()),
            in_bcast_pkts: ip_ext_stat.in_bcast_pkts.map(|x| x.try_into().unwrap()),
            out_bcast_pkts: ip_ext_stat.out_bcast_pkts.map(|x| x.try_into().unwrap()),
            in_octets: ip_ext_stat.in_octets.map(|x| x.try_into().unwrap()),
            out_octets: ip_ext_stat.out_octets.map(|x| x.try_into().unwrap()),
            in_mcast_octets: ip_ext_stat.in_mcast_octets.map(|x| x.try_into().unwrap()),
            out_mcast_octets: ip_ext_stat.out_mcast_octets.map(|x| x.try_into().unwrap()),
            in_bcast_octets: ip_ext_stat.in_bcast_octets.map(|x| x.try_into().unwrap()),
            out_bcast_octets: ip_ext_stat.out_bcast_octets.map(|x| x.try_into().unwrap()),
            in_no_ect_pkts: ip_ext_stat.in_no_ect_pkts.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<IpStat> for procfs_thrift::IpStat {
    fn from(ip_stat: IpStat) -> Self {
        Self {
            forwarding: ip_stat.forwarding.map(|x| x.try_into().unwrap()),
            in_receives: ip_stat.in_receives.map(|x| x.try_into().unwrap()),
            forw_datagrams: ip_stat.forw_datagrams.map(|x| x.try_into().unwrap()),
            in_discards: ip_stat.in_discards.map(|x| x.try_into().unwrap()),
            in_delivers: ip_stat.in_delivers.map(|x| x.try_into().unwrap()),
            out_requests: ip_stat.out_requests.map(|x| x.try_into().unwrap()),
            out_discards: ip_stat.out_discards.map(|x| x.try_into().unwrap()),
            out_no_routes: ip_stat.out_no_routes.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<Ip6Stat> for procfs_thrift::Ip6Stat {
    fn from(ip6_stat: Ip6Stat) -> Self {
        Self {
            in_receives: ip6_stat.in_receives.map(|x| x.try_into().unwrap()),
            in_hdr_errors: ip6_stat.in_hdr_errors.map(|x| x.try_into().unwrap()),
            in_no_routes: ip6_stat.in_no_routes.map(|x| x.try_into().unwrap()),
            in_addr_errors: ip6_stat.in_addr_errors.map(|x| x.try_into().unwrap()),
            in_discards: ip6_stat.in_discards.map(|x| x.try_into().unwrap()),
            in_delivers: ip6_stat.in_delivers.map(|x| x.try_into().unwrap()),
            out_forw_datagrams: ip6_stat.out_forw_datagrams.map(|x| x.try_into().unwrap()),
            out_requests: ip6_stat.out_requests.map(|x| x.try_into().unwrap()),
            out_no_routes: ip6_stat.out_no_routes.map(|x| x.try_into().unwrap()),
            in_mcast_pkts: ip6_stat.in_mcast_pkts.map(|x| x.try_into().unwrap()),
            out_mcast_pkts: ip6_stat.out_mcast_pkts.map(|x| x.try_into().unwrap()),
            in_octets: ip6_stat.in_octets.map(|x| x.try_into().unwrap()),
            out_octets: ip6_stat.out_octets.map(|x| x.try_into().unwrap()),
            in_mcast_octets: ip6_stat.in_mcast_octets.map(|x| x.try_into().unwrap()),
            out_mcast_octets: ip6_stat.out_mcast_octets.map(|x| x.try_into().unwrap()),
            in_bcast_octets: ip6_stat.in_bcast_octets.map(|x| x.try_into().unwrap()),
            out_bcast_octets: ip6_stat.out_bcast_octets.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<IcmpStat> for procfs_thrift::IcmpStat {
    fn from(icmp_stat: IcmpStat) -> Self {
        Self {
            in_msgs: icmp_stat.in_msgs.map(|x| x.try_into().unwrap()),
            in_errors: icmp_stat.in_errors.map(|x| x.try_into().unwrap()),
            in_dest_unreachs: icmp_stat.in_dest_unreachs.map(|x| x.try_into().unwrap()),
            out_msgs: icmp_stat.out_msgs.map(|x| x.try_into().unwrap()),
            out_errors: icmp_stat.out_errors.map(|x| x.try_into().unwrap()),
            out_dest_unreachs: icmp_stat.out_dest_unreachs.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<Icmp6Stat> for procfs_thrift::Icmp6Stat {
    fn from(icmp6_stat: Icmp6Stat) -> Self {
        Self {
            in_msgs: icmp6_stat.in_msgs.map(|x| x.try_into().unwrap()),
            in_errors: icmp6_stat.in_errors.map(|x| x.try_into().unwrap()),
            out_msgs: icmp6_stat.out_msgs.map(|x| x.try_into().unwrap()),
            out_errors: icmp6_stat.out_errors.map(|x| x.try_into().unwrap()),
            in_dest_unreachs: icmp6_stat.in_dest_unreachs.map(|x| x.try_into().unwrap()),
            out_dest_unreachs: icmp6_stat.out_dest_unreachs.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<UdpStat> for procfs_thrift::UdpStat {
    fn from(udp_stat: UdpStat) -> Self {
        Self {
            in_datagrams: udp_stat.in_datagrams.map(|x| x.try_into().unwrap()),
            no_ports: udp_stat.no_ports.map(|x| x.try_into().unwrap()),
            in_errors: udp_stat.in_errors.map(|x| x.try_into().unwrap()),
            out_datagrams: udp_stat.out_datagrams.map(|x| x.try_into().unwrap()),
            rcvbuf_errors: udp_stat.rcvbuf_errors.map(|x| x.try_into().unwrap()),
            sndbuf_errors: udp_stat.sndbuf_errors.map(|x| x.try_into().unwrap()),
            ignored_multi: udp_stat.ignored_multi.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<Udp6Stat> for procfs_thrift::Udp6Stat {
    fn from(udp6_stat: Udp6Stat) -> Self {
        Self {
            in_datagrams: udp6_stat.in_datagrams.map(|x| x.try_into().unwrap()),
            no_ports: udp6_stat.no_ports.map(|x| x.try_into().unwrap()),
            in_errors: udp6_stat.in_errors.map(|x| x.try_into().unwrap()),
            out_datagrams: udp6_stat.out_datagrams.map(|x| x.try_into().unwrap()),
            rcvbuf_errors: udp6_stat.rcvbuf_errors.map(|x| x.try_into().unwrap()),
            sndbuf_errors: udp6_stat.sndbuf_errors.map(|x| x.try_into().unwrap()),
            in_csum_errors: udp6_stat.in_csum_errors.map(|x| x.try_into().unwrap()),
            ignored_multi: udp6_stat.ignored_multi.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<VmStat> for procfs_thrift::VmStat {
    fn from(vm_stat: VmStat) -> Self {
        Self {
            pgpgin: vm_stat.pgpgin.map(|x| x.try_into().unwrap()),
            pgpgout: vm_stat.pgpgout.map(|x| x.try_into().unwrap()),
            pswpin: vm_stat.pswpin.map(|x| x.try_into().unwrap()),
            pswpout: vm_stat.pswpout.map(|x| x.try_into().unwrap()),
            pgsteal_kswapd: vm_stat.pgsteal_kswapd.map(|x| x.try_into().unwrap()),
            pgsteal_direct: vm_stat.pgsteal_direct.map(|x| x.try_into().unwrap()),
            pgscan_kswapd: vm_stat.pgscan_kswapd.map(|x| x.try_into().unwrap()),
            pgscan_direct: vm_stat.pgscan_direct.map(|x| x.try_into().unwrap()),
            oom_kill: vm_stat.oom_kill.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<DiskStat> for procfs_thrift::DiskStat {
    fn from(disk_stat: DiskStat) -> Self {
        Self {
            major: disk_stat.major.map(|x| x.try_into().unwrap()),
            minor: disk_stat.minor.map(|x| x.try_into().unwrap()),
            name: disk_stat.name,
            read_completed: disk_stat.read_completed.map(|x| x.try_into().unwrap()),
            read_merged: disk_stat.read_merged.map(|x| x.try_into().unwrap()),
            read_sectors: disk_stat.read_sectors.map(|x| x.try_into().unwrap()),
            time_spend_read_ms: disk_stat.time_spend_read_ms.map(|x| x.try_into().unwrap()),
            write_completed: disk_stat.write_completed.map(|x| x.try_into().unwrap()),
            write_merged: disk_stat.write_merged.map(|x| x.try_into().unwrap()),
            write_sectors: disk_stat.write_sectors.map(|x| x.try_into().unwrap()),
            time_spend_write_ms: disk_stat.time_spend_write_ms.map(|x| x.try_into().unwrap()),
            discard_completed: disk_stat.discard_completed.map(|x| x.try_into().unwrap()),
            discard_merged: disk_stat.discard_merged.map(|x| x.try_into().unwrap()),
            discard_sectors: disk_stat.discard_sectors.map(|x| x.try_into().unwrap()),
            time_spend_discard_ms: disk_stat
                .time_spend_discard_ms
                .map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<PidState> for procfs_thrift::PidState {
    fn from(pid_state: PidState) -> Self {
        match pid_state {
            PidState::Running => Self::RUNNING,
            PidState::Sleeping => Self::SLEEPING,
            PidState::UninterruptibleSleep => Self::UNINTERRUPTIBLE_SLEEP,
            PidState::Stopped => Self::STOPPED,
            PidState::TracingStopped => Self::TRACING_STOPPED,
            PidState::Zombie => Self::ZOMBIE,
            PidState::Dead => Self::DEAD,
            PidState::Idle => Self::IDLE,
            PidState::Parked => Self::PARKED,
        }
    }
}

impl From<PidStat> for procfs_thrift::PidStat {
    fn from(pid_stat: PidStat) -> Self {
        Self {
            pid: pid_stat.pid,
            comm: pid_stat.comm,
            state: pid_stat.state.map(From::from),
            ppid: pid_stat.ppid,
            pgrp: pid_stat.pgrp,
            session: pid_stat.session,
            minflt: pid_stat.minflt.map(|x| x.try_into().unwrap()),
            majflt: pid_stat.majflt.map(|x| x.try_into().unwrap()),
            user_usecs: pid_stat.user_usecs.map(|x| x.try_into().unwrap()),
            system_usecs: pid_stat.system_usecs.map(|x| x.try_into().unwrap()),
            num_threads: pid_stat.num_threads.map(|x| x.try_into().unwrap()),
            running_secs: pid_stat.running_secs.map(|x| x.try_into().unwrap()),
            rss_bytes: pid_stat.rss_bytes.map(|x| x.try_into().unwrap()),
            processor: pid_stat.processor,
        }
    }
}

impl From<PidMem> for procfs_thrift::PidMem {
    fn from(pid_mem: PidMem) -> Self {
        Self {
            vm_size: pid_mem.vm_size.map(|x| x.try_into().unwrap()),
            lock: pid_mem.lock.map(|x| x.try_into().unwrap()),
            pin: pid_mem.pin.map(|x| x.try_into().unwrap()),
            anon: pid_mem.anon.map(|x| x.try_into().unwrap()),
            file: pid_mem.file.map(|x| x.try_into().unwrap()),
            shmem: pid_mem.shmem.map(|x| x.try_into().unwrap()),
            pte: pid_mem.pte.map(|x| x.try_into().unwrap()),
            swap: pid_mem.swap.map(|x| x.try_into().unwrap()),
            huge_tlb: pid_mem.huge_tlb.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<PidIo> for procfs_thrift::PidIo {
    fn from(pid_io: PidIo) -> Self {
        Self {
            rbytes: pid_io.rbytes.map(|x| x.try_into().unwrap()),
            wbytes: pid_io.wbytes.map(|x| x.try_into().unwrap()),
        }
    }
}

impl From<PidInfo> for procfs_thrift::PidInfo {
    fn from(pid_info: PidInfo) -> Self {
        Self {
            stat: pid_info.stat.into(),
            io: pid_info.io.into(),
            cgroup: pid_info.cgroup,
            cmdline_vec: pid_info.cmdline_vec,
            exe_path: pid_info.exe_path,
            mem: pid_info.mem.into(),
        }
    }
}

impl From<NetStat> for procfs_thrift::NetStat {
    fn from(net_stat: NetStat) -> Self {
        Self {
            interfaces: net_stat
                .interfaces
                .map(|m| m.into_iter().map(|(k, v)| (k, v.into())).collect()),
            tcp: net_stat.tcp.map(From::from),
            tcp_ext: net_stat.tcp_ext.map(From::from),
            ip: net_stat.ip.map(From::from),
            ip_ext: net_stat.ip_ext.map(From::from),
            ip6: net_stat.ip6.map(From::from),
            icmp: net_stat.icmp.map(From::from),
            icmp6: net_stat.icmp6.map(From::from),
            udp: net_stat.udp.map(From::from),
            udp6: net_stat.udp6.map(From::from),
        }
    }
}
