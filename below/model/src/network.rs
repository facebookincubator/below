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
    pub fn new(sample: &procfs::NetStat, last: Option<(&procfs::NetStat, Duration)>) -> Self {
        let mut interfaces: BTreeMap<String, SingleNetModel> = BTreeMap::new();

        if let Some(ifaces) = sample.interfaces.as_ref() {
            for (interface, iface_stat) in ifaces.iter() {
                interfaces.insert(
                    interface.to_string(),
                    SingleNetModel::new(
                        &interface,
                        &iface_stat,
                        last.and_then(|(n, d)| {
                            n.interfaces
                                .as_ref()
                                .and_then(|ifaces| ifaces.get(interface).map(|n| (n, d)))
                        }),
                    ),
                );
            }
        }

        NetworkModel {
            interfaces,
            tcp: TcpModel::new(
                sample.tcp.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.tcp.as_ref().map(|n| (n, d))),
            ),
            ip: IpModel::new(
                sample.ip.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.ip.as_ref().map(|n| (n, d))),
                sample.ip_ext.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.ip_ext.as_ref().map(|n| (n, d))),
            ),
            ip6: Ip6Model::new(
                sample.ip6.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.ip6.as_ref().map(|n| (n, d))),
            ),
            icmp: IcmpModel::new(
                sample.icmp.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.icmp.as_ref().map(|n| (n, d))),
            ),
            icmp6: Icmp6Model::new(
                sample.icmp6.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.icmp6.as_ref().map(|n| (n, d))),
            ),
            udp: UdpModel::new(
                sample.udp.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.udp.as_ref().map(|n| (n, d))),
            ),
            udp6: Udp6Model::new(
                sample.udp6.as_ref().unwrap_or(&Default::default()),
                last.and_then(|(n, d)| n.udp6.as_ref().map(|n| (n, d))),
            ),
        }
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
}

impl SingleNetModel {
    fn new(
        interface: &str,
        sample: &procfs::InterfaceStat,
        last: Option<(&procfs::InterfaceStat, Duration)>,
    ) -> SingleNetModel {
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

        SingleNetModel {
            interface: interface.to_string(),
            rx_bytes_per_sec,
            tx_bytes_per_sec,
            throughput_per_sec,
            rx_packets_per_sec: last
                .map(|(l, d)| {
                    count_per_sec!(
                        l.rx_packets.map(|s| s as u64),
                        sample.rx_packets.map(|s| s as u64),
                        d
                    )
                })
                .unwrap_or_default()
                .map(|s| s as u64),
            tx_packets_per_sec: last
                .map(|(l, d)| {
                    count_per_sec!(
                        l.tx_packets.map(|s| s as u64),
                        sample.tx_packets.map(|s| s as u64),
                        d
                    )
                })
                .unwrap_or_default()
                .map(|s| s as u64),
            collisions: sample.collisions.map(|s| s as u64),
            multicast: sample.multicast.map(|s| s as u64),
            rx_bytes: sample.rx_bytes.map(|s| s as u64),
            rx_compressed: sample.rx_compressed.map(|s| s as u64),
            rx_crc_errors: sample.rx_crc_errors.map(|s| s as u64),
            rx_dropped: sample.rx_dropped.map(|s| s as u64),
            rx_errors: sample.rx_errors.map(|s| s as u64),
            rx_fifo_errors: sample.rx_fifo_errors.map(|s| s as u64),
            rx_frame_errors: sample.rx_frame_errors.map(|s| s as u64),
            rx_length_errors: sample.rx_length_errors.map(|s| s as u64),
            rx_missed_errors: sample.rx_missed_errors.map(|s| s as u64),
            rx_nohandler: sample.rx_nohandler.map(|s| s as u64),
            rx_over_errors: sample.rx_over_errors.map(|s| s as u64),
            rx_packets: sample.rx_packets.map(|s| s as u64),
            tx_aborted_errors: sample.tx_aborted_errors.map(|s| s as u64),
            tx_bytes: sample.tx_bytes.map(|s| s as u64),
            tx_carrier_errors: sample.tx_carrier_errors.map(|s| s as u64),
            tx_compressed: sample.tx_compressed.map(|s| s as u64),
            tx_dropped: sample.tx_dropped.map(|s| s as u64),
            tx_errors: sample.tx_errors.map(|s| s as u64),
            tx_fifo_errors: sample.tx_fifo_errors.map(|s| s as u64),
            tx_heartbeat_errors: sample.tx_heartbeat_errors.map(|s| s as u64),
            tx_packets: sample.tx_packets.map(|s| s as u64),
            tx_window_errors: sample.tx_window_errors.map(|s| s as u64),
        }
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
                    "rx_bytes_per_sec": 42
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
    }
}
