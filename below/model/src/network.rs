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

use super::*;

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct NetworkModel {
    #[queriable(subquery)]
    pub interfaces: BTreeMap<String, SingleNetModel>,
    #[queriable(subquery)]
    pub tcp: TcpModel,
    #[queriable(subquery)]
    pub ip: IpModel,
    #[queriable(subquery)]
    pub ip6: Ip6Model,
    #[queriable(subquery)]
    pub icmp: IcmpModel,
    #[queriable(subquery)]
    pub icmp6: Icmp6Model,
    #[queriable(subquery)]
    pub udp: UdpModel,
    #[queriable(subquery)]
    pub udp6: Udp6Model,
}

impl NetworkModel {
    pub fn new(sample: &NetworkStats, last: Option<(&NetworkStats, Duration)>) -> Self {
        let mut interfaces: BTreeMap<String, SingleNetModel> = BTreeMap::new();

        let net_stats = sample.net;
        let ethtool_stats = sample.ethtool;

        let mut iface_names = BTreeSet::new();
        if let Some(ifaces) = net_stats.interfaces.as_ref() {
            for (interface, _) in ifaces.iter() {
                iface_names.insert(interface.to_string());
            }
        }
        for key in ethtool_stats.nic.keys() {
            iface_names.insert(key.to_string());
        }

        for interface in iface_names {
            let iface_stat = net_stats
                .interfaces
                .as_ref()
                .and_then(|ifaces| ifaces.get(&interface));
            let ethtool_stat = ethtool_stats.nic.get(&interface);

            let s_iface = SingleNetworkStat {
                iface: iface_stat,
                nic: ethtool_stat
            };

            let mut l_network_stat = SingleNetworkStat {
                iface: None,
                nic: None
            };
            let l_iface = last.map(|(l, d)| {
                let l_iface_stat = l.net.interfaces.as_ref()
                    .and_then(|ifaces| {
                        ifaces.get(&interface)
                    });
                let l_ethtool_stat = l.ethtool.nic.get(&interface);
                l_network_stat = SingleNetworkStat {
                    iface: l_iface_stat,
                    nic: l_ethtool_stat
                };
                (&l_network_stat, d)
            });

            let net_model = SingleNetModel::new(
                &interface,
                &s_iface,
                l_iface,
            );
            interfaces.insert(interface, net_model);
        }

        NetworkModel {
            interfaces,
            tcp: TcpModel::new(
                sample.net.tcp.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.tcp.as_ref().map(|n| (n, d))
                }),
            ),
            ip: IpModel::new(
                sample.net.ip.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.ip.as_ref().map(|n| (n, d))
                }),
                sample.net.ip_ext.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.ip_ext.as_ref().map(|n| (n, d))
                }),
            ),
            ip6: Ip6Model::new(
                sample.net.ip6.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.ip6.as_ref().map(|n| (n, d))
                }),
            ),
            icmp: IcmpModel::new(
                sample.net.icmp.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.icmp.as_ref().map(|n| (n, d))
                }),
            ),
            icmp6: Icmp6Model::new(
                sample.net.icmp6.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.icmp6.as_ref().map(|n| (n, d))
                }),
            ),
            udp: UdpModel::new(
                sample.net.udp.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.udp.as_ref().map(|n| (n, d))
                }),
            ),
            udp6: Udp6Model::new(
                sample.net.udp6.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(l, d)| {
                    let n = l.net;
                    n.udp6.as_ref().map(|n| (n, d))
                }),
            ),
        }
    }
}

impl Nameable for NetworkModel {
    fn name() -> &'static str {
        "network"
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct TcpModel {
    pub active_opens_per_sec: Option<u64>,
    pub passive_opens_per_sec: Option<u64>,
    pub attempt_fails_per_sec: Option<u64>,
    pub estab_resets_per_sec: Option<u64>,
    pub curr_estab_conn: Option<u64>,
    pub in_segs_per_sec: Option<u64>,
    pub out_segs_per_sec: Option<u64>,
    pub retrans_segs_per_sec: Option<u64>,
    pub retrans_segs: Option<u64>,
    pub in_errs: Option<u64>,
    pub out_rsts_per_sec: Option<u64>,
    pub in_csum_errors: Option<u64>,
    // Collected TcpExt stats, but not going to display. If we got feedback that user do need
    // those stats, we can add those here.
}

impl TcpModel {
    pub fn new(sample: &procfs::TcpStat, last: Option<(&procfs::TcpStat, Duration)>) -> TcpModel {
        TcpModel {
            active_opens_per_sec: get_option_rate!(active_opens, sample, last),
            passive_opens_per_sec: get_option_rate!(passive_opens, sample, last),
            attempt_fails_per_sec: get_option_rate!(attempt_fails, sample, last),
            estab_resets_per_sec: get_option_rate!(estab_resets, sample, last),
            curr_estab_conn: sample.curr_estab.map(|s| s as u64),
            in_segs_per_sec: get_option_rate!(in_segs, sample, last),
            out_segs_per_sec: get_option_rate!(out_segs, sample, last),
            retrans_segs_per_sec: get_option_rate!(retrans_segs, sample, last),
            retrans_segs: sample.retrans_segs.map(|s| s as u64),
            in_errs: sample.in_errs.map(|s| s as u64),
            out_rsts_per_sec: get_option_rate!(out_rsts, sample, last),
            in_csum_errors: sample.in_csum_errors.map(|s| s as u64),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct IpModel {
    pub forwarding_pkts_per_sec: Option<u64>,
    pub in_receives_pkts_per_sec: Option<u64>,
    pub forw_datagrams_per_sec: Option<u64>,
    pub in_discards_pkts_per_sec: Option<u64>,
    pub in_delivers_pkts_per_sec: Option<u64>,
    pub out_requests_per_sec: Option<u64>,
    pub out_discards_pkts_per_sec: Option<u64>,
    pub out_no_routes_pkts_per_sec: Option<u64>,
    // IpExt stats below
    pub in_mcast_pkts_per_sec: Option<u64>,
    pub out_mcast_pkts_per_sec: Option<u64>,
    pub in_bcast_pkts_per_sec: Option<u64>,
    pub out_bcast_pkts_per_sec: Option<u64>,
    pub in_octets_per_sec: Option<u64>,
    pub out_octets_per_sec: Option<u64>,
    pub in_mcast_octets_per_sec: Option<u64>,
    pub out_mcast_octets_per_sec: Option<u64>,
    pub in_bcast_octets_per_sec: Option<u64>,
    pub out_bcast_octets_per_sec: Option<u64>,
    pub in_no_ect_pkts_per_sec: Option<u64>,
}

impl IpModel {
    pub fn new(
        sample: &procfs::IpStat,
        last: Option<(&procfs::IpStat, Duration)>,
        sample_ext: &procfs::IpExtStat,
        last_ext: Option<(&procfs::IpExtStat, Duration)>,
    ) -> IpModel {
        IpModel {
            forwarding_pkts_per_sec: get_option_rate!(forwarding, sample, last),
            in_receives_pkts_per_sec: get_option_rate!(in_receives, sample, last),
            forw_datagrams_per_sec: get_option_rate!(forw_datagrams, sample, last),
            in_discards_pkts_per_sec: get_option_rate!(in_discards, sample, last),
            in_delivers_pkts_per_sec: get_option_rate!(in_delivers, sample, last),
            out_requests_per_sec: get_option_rate!(out_requests, sample, last),
            out_discards_pkts_per_sec: get_option_rate!(out_discards, sample, last),
            out_no_routes_pkts_per_sec: get_option_rate!(out_no_routes, sample, last),
            // IpExt
            in_mcast_pkts_per_sec: get_option_rate!(in_mcast_pkts, sample_ext, last_ext),
            out_mcast_pkts_per_sec: get_option_rate!(out_mcast_pkts, sample_ext, last_ext),
            in_bcast_pkts_per_sec: get_option_rate!(in_bcast_pkts, sample_ext, last_ext),
            out_bcast_pkts_per_sec: get_option_rate!(out_bcast_pkts, sample_ext, last_ext),
            in_octets_per_sec: get_option_rate!(in_octets, sample_ext, last_ext),
            out_octets_per_sec: get_option_rate!(out_octets, sample_ext, last_ext),
            in_mcast_octets_per_sec: get_option_rate!(in_mcast_octets, sample_ext, last_ext),
            out_mcast_octets_per_sec: get_option_rate!(out_mcast_octets, sample_ext, last_ext),
            in_bcast_octets_per_sec: get_option_rate!(in_bcast_octets, sample_ext, last_ext),
            out_bcast_octets_per_sec: get_option_rate!(out_bcast_octets, sample_ext, last_ext),
            in_no_ect_pkts_per_sec: get_option_rate!(in_no_ect_pkts, sample_ext, last_ext),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct Ip6Model {
    pub in_receives_pkts_per_sec: Option<u64>,
    pub in_hdr_errors: Option<u64>,
    pub in_no_routes_pkts_per_sec: Option<u64>,
    pub in_addr_errors: Option<u64>,
    pub in_discards_pkts_per_sec: Option<u64>,
    pub in_delivers_pkts_per_sec: Option<u64>,
    pub out_forw_datagrams_per_sec: Option<u64>,
    pub out_requests_per_sec: Option<u64>,
    pub out_no_routes_pkts_per_sec: Option<u64>,
    pub in_mcast_pkts_per_sec: Option<u64>,
    pub out_mcast_pkts_per_sec: Option<u64>,
    pub in_octets_per_sec: Option<u64>,
    pub out_octets_per_sec: Option<u64>,
    pub in_mcast_octets_per_sec: Option<u64>,
    pub out_mcast_octets_per_sec: Option<u64>,
    pub in_bcast_octets_per_sec: Option<u64>,
    pub out_bcast_octets_per_sec: Option<u64>,
}

impl Ip6Model {
    pub fn new(sample: &procfs::Ip6Stat, last: Option<(&procfs::Ip6Stat, Duration)>) -> Ip6Model {
        Ip6Model {
            in_receives_pkts_per_sec: get_option_rate!(in_receives, sample, last),
            in_hdr_errors: sample.in_hdr_errors.map(|s| s as u64),
            in_no_routes_pkts_per_sec: get_option_rate!(in_no_routes, sample, last),
            in_addr_errors: sample.in_addr_errors.map(|s| s as u64),
            in_discards_pkts_per_sec: get_option_rate!(in_discards, sample, last),
            in_delivers_pkts_per_sec: get_option_rate!(in_delivers, sample, last),
            out_forw_datagrams_per_sec: get_option_rate!(out_forw_datagrams, sample, last),
            out_requests_per_sec: get_option_rate!(out_requests, sample, last),
            out_no_routes_pkts_per_sec: get_option_rate!(out_no_routes, sample, last),
            in_mcast_pkts_per_sec: get_option_rate!(in_mcast_pkts, sample, last),
            out_mcast_pkts_per_sec: get_option_rate!(out_mcast_pkts, sample, last),
            in_octets_per_sec: get_option_rate!(in_octets, sample, last),
            out_octets_per_sec: get_option_rate!(out_octets, sample, last),
            in_mcast_octets_per_sec: get_option_rate!(in_mcast_octets, sample, last),
            out_mcast_octets_per_sec: get_option_rate!(out_mcast_octets, sample, last),
            in_bcast_octets_per_sec: get_option_rate!(in_bcast_octets, sample, last),
            out_bcast_octets_per_sec: get_option_rate!(out_bcast_octets, sample, last),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct IcmpModel {
    pub in_msgs_per_sec: Option<u64>,
    pub in_errors: Option<u64>,
    pub in_dest_unreachs: Option<u64>,
    pub out_msgs_per_sec: Option<u64>,
    pub out_errors: Option<u64>,
    pub out_dest_unreachs: Option<u64>,
}

impl IcmpModel {
    pub fn new(
        sample: &procfs::IcmpStat,
        last: Option<(&procfs::IcmpStat, Duration)>,
    ) -> IcmpModel {
        IcmpModel {
            in_msgs_per_sec: get_option_rate!(in_msgs, sample, last),
            in_errors: sample.in_errors.map(|s| s as u64),
            in_dest_unreachs: sample.in_dest_unreachs.map(|s| s as u64),
            out_msgs_per_sec: get_option_rate!(out_msgs, sample, last),
            out_errors: sample.out_errors.map(|s| s as u64),
            out_dest_unreachs: sample.out_dest_unreachs.map(|s| s as u64),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct Icmp6Model {
    pub in_msgs_per_sec: Option<u64>,
    pub in_errors: Option<u64>,
    pub in_dest_unreachs: Option<u64>,
    pub out_msgs_per_sec: Option<u64>,
    pub out_errors: Option<u64>,
    pub out_dest_unreachs: Option<u64>,
}

impl Icmp6Model {
    pub fn new(
        sample: &procfs::Icmp6Stat,
        last: Option<(&procfs::Icmp6Stat, Duration)>,
    ) -> Icmp6Model {
        Icmp6Model {
            in_msgs_per_sec: get_option_rate!(in_msgs, sample, last),
            in_errors: sample.in_errors.map(|s| s as u64),
            in_dest_unreachs: sample.in_dest_unreachs.map(|s| s as u64),
            out_msgs_per_sec: get_option_rate!(out_msgs, sample, last),
            out_errors: sample.out_errors.map(|s| s as u64),
            out_dest_unreachs: sample.out_dest_unreachs.map(|s| s as u64),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct UdpModel {
    pub in_datagrams_pkts_per_sec: Option<u64>,
    pub no_ports: Option<u64>,
    pub in_errors: Option<u64>,
    pub out_datagrams_pkts_per_sec: Option<u64>,
    pub rcvbuf_errors: Option<u64>,
    pub sndbuf_errors: Option<u64>,
    pub ignored_multi: Option<u64>,
}

impl UdpModel {
    pub fn new(sample: &procfs::UdpStat, last: Option<(&procfs::UdpStat, Duration)>) -> UdpModel {
        UdpModel {
            in_datagrams_pkts_per_sec: get_option_rate!(in_datagrams, sample, last),
            no_ports: sample.no_ports.map(|s| s as u64),
            in_errors: sample.in_errors.map(|s| s as u64),
            out_datagrams_pkts_per_sec: get_option_rate!(out_datagrams, sample, last),
            rcvbuf_errors: sample.rcvbuf_errors.map(|s| s as u64),
            sndbuf_errors: sample.sndbuf_errors.map(|s| s as u64),
            ignored_multi: sample.ignored_multi.map(|s| s as u64),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct Udp6Model {
    pub in_datagrams_pkts_per_sec: Option<u64>,
    pub no_ports: Option<u64>,
    pub in_errors: Option<u64>,
    pub out_datagrams_pkts_per_sec: Option<u64>,
    pub rcvbuf_errors: Option<u64>,
    pub sndbuf_errors: Option<u64>,
    pub in_csum_errors: Option<u64>,
    pub ignored_multi: Option<u64>,
}

impl Udp6Model {
    pub fn new(
        sample: &procfs::Udp6Stat,
        last: Option<(&procfs::Udp6Stat, Duration)>,
    ) -> Udp6Model {
        Udp6Model {
            in_datagrams_pkts_per_sec: get_option_rate!(in_datagrams, sample, last),
            no_ports: sample.no_ports.map(|s| s as u64),
            in_errors: sample.in_errors.map(|s| s as u64),
            out_datagrams_pkts_per_sec: get_option_rate!(out_datagrams, sample, last),
            rcvbuf_errors: sample.rcvbuf_errors.map(|s| s as u64),
            sndbuf_errors: sample.sndbuf_errors.map(|s| s as u64),
            in_csum_errors: sample.in_csum_errors.map(|s| s as u64),
            ignored_multi: sample.ignored_multi.map(|s| s as u64),
        }
    }
}

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct SingleNetModel {
    pub interface: String,
    pub rx_bytes_per_sec: Option<f64>,
    pub tx_bytes_per_sec: Option<f64>,
    pub throughput_per_sec: Option<f64>,
    pub rx_packets_per_sec: Option<u64>,
    pub tx_packets_per_sec: Option<u64>,
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
    pub tx_timeout_per_sec: Option<u64>,
    pub raw_stats: BTreeMap<String, u64>,

    #[queriable(subquery)]
    pub queues: Vec<SingleQueueModel>,
}

pub struct SingleNetworkStat<'a> {
    iface: Option<&'a procfs::InterfaceStat>,
    nic: Option<&'a ethtool::NicStats>
}

impl SingleNetModel {
    fn add_iface_stats(
        net_model: &mut SingleNetModel,
        sample: &procfs::InterfaceStat,
        last: Option<(&procfs::InterfaceStat, Duration)>,
    ) {
        let rx_bytes_per_sec = last
            .map(|(l, d)| {
                count_per_sec!(
                    l.rx_bytes.map(|s| s as u64),
                    sample.rx_bytes.map(|s| s as u64),
                    d
                )
            })
            .unwrap_or_default();
        let tx_bytes_per_sec = last
            .map(|(l, d)| {
                count_per_sec!(
                    l.tx_bytes.map(|s| s as u64),
                    sample.tx_bytes.map(|s| s as u64),
                    d
                )
            })
            .unwrap_or_default();
        let throughput_per_sec =
            Some(rx_bytes_per_sec.unwrap_or_default() + tx_bytes_per_sec.unwrap_or_default());

        net_model.rx_bytes_per_sec = rx_bytes_per_sec;
        net_model.tx_bytes_per_sec = tx_bytes_per_sec;
        net_model.throughput_per_sec = throughput_per_sec;
        net_model.rx_packets_per_sec = last
            .map(|(l, d)| {
                count_per_sec!(
                    l.rx_packets.map(|s| s as u64),
                    sample.rx_packets.map(|s| s as u64),
                    d
                )
            })
            .unwrap_or_default()
            .map(|s| s as u64);
        net_model.tx_packets_per_sec = last
            .map(|(l, d)| {
                count_per_sec!(
                    l.tx_packets.map(|s| s as u64),
                    sample.tx_packets.map(|s| s as u64),
                    d
                )
            })
            .unwrap_or_default()
            .map(|s| s as u64);
        net_model.collisions = sample.collisions.map(|s| s as u64);
        net_model.multicast = sample.multicast.map(|s| s as u64);
        net_model.rx_bytes = sample.rx_bytes.map(|s| s as u64);
        net_model.rx_compressed = sample.rx_compressed.map(|s| s as u64);
        net_model.rx_crc_errors = sample.rx_crc_errors.map(|s| s as u64);
        net_model.rx_dropped = sample.rx_dropped.map(|s| s as u64);
        net_model.rx_errors = sample.rx_errors.map(|s| s as u64);
        net_model.rx_fifo_errors = sample.rx_fifo_errors.map(|s| s as u64);
        net_model.rx_frame_errors = sample.rx_frame_errors.map(|s| s as u64);
        net_model.rx_length_errors = sample.rx_length_errors.map(|s| s as u64);
        net_model.rx_missed_errors = sample.rx_missed_errors.map(|s| s as u64);
        net_model.rx_nohandler = sample.rx_nohandler.map(|s| s as u64);
        net_model.rx_over_errors = sample.rx_over_errors.map(|s| s as u64);
        net_model.rx_packets = sample.rx_packets.map(|s| s as u64);
        net_model.tx_aborted_errors = sample.tx_aborted_errors.map(|s| s as u64);
        net_model.tx_bytes = sample.tx_bytes.map(|s| s as u64);
        net_model.tx_carrier_errors = sample.tx_carrier_errors.map(|s| s as u64);
        net_model.tx_compressed = sample.tx_compressed.map(|s| s as u64);
        net_model.tx_dropped = sample.tx_dropped.map(|s| s as u64);
        net_model.tx_errors = sample.tx_errors.map(|s| s as u64);
        net_model.tx_fifo_errors = sample.tx_fifo_errors.map(|s| s as u64);
        net_model.tx_heartbeat_errors = sample.tx_heartbeat_errors.map(|s| s as u64);
        net_model.tx_packets = sample.tx_packets.map(|s| s as u64);
        net_model.tx_window_errors = sample.tx_window_errors.map(|s| s as u64);
    }

    fn add_ethtool_stats(
        net_model: &mut SingleNetModel,
        sample: &ethtool::NicStats,
        last: Option<(&ethtool::NicStats, Duration)>,
    ) {
        net_model.tx_timeout_per_sec = get_option_rate!(tx_timeout, sample, last);
        net_model.raw_stats = sample.raw_stats.clone();

        // set ethtool queue stats
        let s_queue_stats = &sample.queue;
        // Vec<QueueStats> are always sorted on the queue id
        for (queue_id, s_queue_stats) in s_queue_stats.iter().enumerate() {
            let idx = queue_id as u32;
            let last = last.and_then(|(l, d)| {
                let queue_stats = &l.queue;
                queue_stats.get(queue_id).map(|n| (n, d))
            });
            let queue_model = SingleQueueModel::new(&net_model.interface, idx, s_queue_stats, last);
            net_model.queues.push(queue_model);
        }
    }

    fn new(
        interface: &str,
        sample: &SingleNetworkStat,
        last: Option<(&SingleNetworkStat, Duration)>,
    ) -> SingleNetModel {
        let iface_stat = sample.iface;
        let ethtool_stat = sample.nic;

        let mut net_model = SingleNetModel {
            interface: interface.to_string(),
            ..Default::default()
        };

        // set procfs iface stats
        if let Some(iface_stat) = iface_stat {
            Self::add_iface_stats(
                &mut net_model,
                iface_stat,
                last.and_then(|(l, d)| {
                    if let Some(l) = l.iface {
                        Some((l, d))
                    } else {
                        None
                    }
                })
            );
        }

        // set ethtool stats
        if let Some(nic_stat) = ethtool_stat {
            let sample = nic_stat;
            let last = last
                .and_then(|(l, d)| {
                    l.nic.map(|l| (l, d))
                }
            );

            Self::add_ethtool_stats(&mut net_model,
                sample,
                last
            );
        }

        net_model
    }
}

impl Nameable for SingleNetModel {
    fn name() -> &'static str {
        "network"
    }
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct SingleQueueModel {
    pub interface: String,
    pub queue_id: u32,
    pub rx_bytes_per_sec: Option<u64>,
    pub tx_bytes_per_sec: Option<u64>,
    pub rx_count_per_sec: Option<u64>,
    pub tx_count_per_sec: Option<u64>,
    pub tx_missed_tx: Option<u64>,
    pub tx_unmask_interrupt: Option<u64>,
    pub raw_stats: BTreeMap<String, u64>,
}

impl SingleQueueModel {
    fn new(
        interface: &str,
        queue_id: u32,
        sample: &ethtool::QueueStats,
        last: Option<(&ethtool::QueueStats, Duration)>,
    ) -> Self {
        SingleQueueModel {
            interface: interface.to_string(),
            queue_id,
            rx_bytes_per_sec: get_option_rate!(rx_bytes, sample, last),
            tx_bytes_per_sec: get_option_rate!(tx_bytes, sample, last),
            rx_count_per_sec: get_option_rate!(rx_count, sample, last),
            tx_count_per_sec: get_option_rate!(tx_count, sample, last),
            tx_missed_tx: sample.tx_missed_tx,
            tx_unmask_interrupt: sample.tx_unmask_interrupt,
            raw_stats: sample.raw_stats.clone(),
        }
    }
}

impl Nameable for SingleQueueModel {
    fn name() -> &'static str {
        "ethtool_queue"
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn query_model() {
        let model_json = r#"
        {
            "interfaces": {
                "eth0": {
                    "interface": "eth0",
                    "rx_bytes_per_sec": 42,
                    "tx_timeout_per_sec": 10,
                    "raw_stats": {
                        "stat0": 0
                    },
                    "queues": [
                        {
                            "interface": "eth0",
                            "queue_id": 0,
                            "rx_bytes_per_sec": 42,
                            "tx_bytes_per_sec": 1337,
                            "rx_count_per_sec": 10,
                            "tx_count_per_sec": 20,
                            "tx_missed_tx": 100,
                            "tx_unmask_interrupt": 200,
                            "raw_stats": {
                                "stat1": 1,
                                "stat2": 2
                            }
                        },
                        {
                            "interface": "eth0",
                            "queue_id": 1,
                            "rx_bytes_per_sec": 1337,
                            "tx_bytes_per_sec": 42,
                            "rx_count_per_sec": 20,
                            "tx_count_per_sec": 10,
                            "tx_missed_tx": 200,
                            "tx_unmask_interrupt": 100,
                            "raw_stats": {
                                "stat3": 3,
                                "stat4": 4
                            }
                        }
                    ]
                }
            },
            "tcp": {},
            "ip": {},
            "ip6": {},
            "icmp": {},
            "icmp6": {},
            "udp": {},
            "udp6": {}
        }
        "#;
        let model: NetworkModel = serde_json::from_str(model_json).unwrap();
        assert_eq!(
            model
                .query(&NetworkModelFieldId::from_str("interfaces.eth0.rx_bytes_per_sec").unwrap()),
            Some(Field::F64(42.0))
        );

        assert_eq!(
            model.query(&NetworkModelFieldId::from_str("interfaces.eth0.tx_timeout_per_sec").unwrap()),
            Some(Field::U64(10))
        );

        assert_eq!(
            model.query(&NetworkModelFieldId::from_str("interfaces.eth0.queues.0.queue_id").unwrap()),
            Some(Field::U32(0))
        );
        assert_eq!(
            model.query(
                &NetworkModelFieldId::from_str("interfaces.eth0.queues.0.rx_bytes_per_sec").unwrap()
            ),
            Some(Field::U64(42))
        );

        assert_eq!(
            model.query(&NetworkModelFieldId::from_str("interfaces.eth0.queues.1.queue_id").unwrap()),
            Some(Field::U32(1))
        );
    }

    #[test]
    fn test_parse_ethtool_stats() {
        let l_net_stats = procfs::NetStat::default();
        let s_net_stats = procfs::NetStat::default();

        let l_ethtool_stats = ethtool::EthtoolStats {
            nic: BTreeMap::from([(
                "eth0".to_string(), ethtool::NicStats {
                    tx_timeout: Some(10),
                    raw_stats: BTreeMap::from([("stat0".to_string(), 0)]),
                    queue: vec![
                        ethtool::QueueStats {
                            rx_bytes: Some(42),
                            tx_bytes: Some(1337),
                            rx_count: Some(10),
                            tx_count: Some(20),
                            tx_missed_tx: Some(100),
                            tx_unmask_interrupt: Some(200),
                            raw_stats: vec![("stat1".to_string(), 1), ("stat2".to_string(), 2)]
                                .into_iter()
                                .collect(),
                        },
                        ethtool::QueueStats {
                            rx_bytes: Some(1337),
                            tx_bytes: Some(42),
                            rx_count: Some(20),
                            tx_count: Some(10),
                            tx_missed_tx: Some(200),
                            tx_unmask_interrupt: Some(100),
                            raw_stats: vec![("stat3".to_string(), 3), ("stat4".to_string(), 4)]
                                .into_iter()
                                .collect(),
                        },
                    ],
                },
            )])
        };

        let s_ethtool_stats = ethtool::EthtoolStats {
            nic: BTreeMap::from([(
                "eth0".to_string(), ethtool::NicStats {
                    tx_timeout: Some(20),
                    raw_stats: BTreeMap::from([("stat0".to_string(), 10)]),
                    queue: vec![
                        ethtool::QueueStats {
                            rx_bytes: Some(52),
                            tx_bytes: Some(1347),
                            rx_count: Some(20),
                            tx_count: Some(30),
                            tx_missed_tx: Some(110),
                            tx_unmask_interrupt: Some(210),
                            raw_stats: vec![("stat1".to_string(), 11), ("stat2".to_string(), 12)]
                                .into_iter()
                                .collect(),
                        },
                        ethtool::QueueStats {
                            rx_bytes: Some(1347),
                            tx_bytes: Some(52),
                            rx_count: Some(30),
                            tx_count: Some(20),
                            tx_missed_tx: Some(210),
                            tx_unmask_interrupt: Some(110),
                            raw_stats: vec![("stat3".to_string(), 13), ("stat4".to_string(), 14)]
                                .into_iter()
                                .collect(),
                        },
                    ],
                },
            )])
        };

        let prev_sample = NetworkStats { net: &l_net_stats, ethtool: &l_ethtool_stats };
        let sample = NetworkStats { net: &s_net_stats, ethtool: &s_ethtool_stats };
        let last = Some((&prev_sample, Duration::from_secs(1)));

        let model = NetworkModel::new(&sample, last);

        let iface_model = model.interfaces.get("eth0").unwrap();
        assert_eq!(iface_model.tx_timeout_per_sec, Some(10));
        let nic_raw_stat = iface_model.raw_stats.get("stat0").unwrap();
        assert_eq!(*nic_raw_stat, 10);

        let queue_model = iface_model.queues.get(0).unwrap();
        assert_eq!(queue_model.rx_bytes_per_sec, Some(10));
        // for raw stats, we just take the latest value (not the difference)
        let queue_raw_stat = queue_model.raw_stats.get("stat1").unwrap();
        assert_eq!(*queue_raw_stat, 11);

        let queue_model = iface_model.queues.get(1).unwrap();
        assert_eq!(queue_model.rx_bytes_per_sec, Some(10));
        let queue_raw_stat = queue_model.raw_stats.get("stat3").unwrap();
        assert_eq!(*queue_raw_stat, 13);
    }
}
