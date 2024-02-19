use std::collections::BTreeMap;

use netlink_packet_route::tc;
use netlink_packet_route::tc::{
    TcAttribute, TcFqCodelXstats, TcMessage, TcOption, TcQdiscFqCodelOption,
};
use serde::{Deserialize, Serialize};

const FQ_CODEL: &str = "fq_codel";

/// `Tc` represents a traffic control qdisc.
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tc {
    /// Index of the network interface.
    pub if_index: u32,
    /// Name of the network interface.
    pub if_name: String,
    /// A unique identifier for the qdisc.
    pub handle: u32,
    /// Identifier of the parent qdisc.
    pub parent: u32,
    /// Type of the queueing discipline, e.g. `fq_codel`, `htb`, etc.
    pub kind: String,
    /// Detailed statistics of the qdisc, such as bytes, packets, qlen, etc.
    pub stats: Stats,
    /// qdisc wraps the specific qdisc type, e.g. `fq_codel`.
    pub qdisc: Option<QDisc>,
}

impl Tc {
    pub fn new(interfaces: &BTreeMap<u32, String>, tc_msg: &TcMessage) -> Self {
        let if_index = tc_msg.header.index as u32;
        let if_name = interfaces
            .get(&if_index)
            .map_or_else(String::new, |iface| iface.to_string());
        let mut tc = Self {
            if_index,
            if_name,
            handle: tc_msg.header.handle.into(),
            parent: tc_msg.header.parent.into(),
            ..Default::default()
        };
        let mut opts = Vec::new();

        for attr in &tc_msg.attributes {
            match attr {
                TcAttribute::Kind(name) => tc.kind = name.clone(),
                TcAttribute::Options(tc_opts) => opts = tc_opts.to_vec(),
                TcAttribute::Stats(tc_stats) => {
                    tc.stats.bps = Some(tc_stats.bps);
                    tc.stats.pps = Some(tc_stats.pps);
                }
                TcAttribute::Stats2(tc_stats) => {
                    for stat in tc_stats {
                        match stat {
                            tc::TcStats2::Basic(basic) => {
                                tc.stats.bytes = Some(basic.bytes);
                                tc.stats.packets = Some(basic.packets);
                            }
                            tc::TcStats2::Queue(queue) => {
                                tc.stats.qlen = Some(queue.qlen);
                                tc.stats.backlog = Some(queue.backlog);
                                tc.stats.drops = Some(queue.drops);
                                tc.stats.requeues = Some(queue.requeues);
                                tc.stats.overlimits = Some(queue.overlimits);
                            }
                            _ => {}
                        }
                    }
                }
                TcAttribute::Xstats(tc_xstats) => match tc_xstats {
                    tc::TcXstats::FqCodel(fq_codel_xstats) => {
                        tc.stats.xstats = Some(XStats::FqCodel(FqCodelXStats::new(fq_codel_xstats)))
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        tc.qdisc = QDisc::new(&tc.kind, opts);

        tc
    }
}

/// `Stats` represents the statistics of a qdisc.
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stats {
    // Stats2::StatsBasic
    /// Number of enqueued bytes.
    pub bytes: Option<u64>,
    /// Number of enqueued packets.
    pub packets: Option<u32>,

    // Stats2::StatsQueue
    /// Length of the queue.
    pub qlen: Option<u32>,
    /// Number of bytes pending in the queue.
    pub backlog: Option<u32>,
    /// Packets dropped because of lack of resources.
    pub drops: Option<u32>,
    pub requeues: Option<u32>,
    /// Number of throttle events when this flow goes out of allocated bandwidth.
    pub overlimits: Option<u32>,

    // XStats
    /// xstats wraps extended statistics of the qdisc.
    pub xstats: Option<XStats>,

    /// Current flow byte rate.
    pub bps: Option<u32>,
    /// Current flow packet rate.
    pub pps: Option<u32>,
}

/// `QDisc` represents the queueing discipline of a network interface.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum QDisc {
    FqCodel(FqCodelQDisc),
}

impl QDisc {
    fn new(kind: &str, opts: Vec<TcOption>) -> Option<Self> {
        if kind == FQ_CODEL {
            let mut fq_codel = FqCodelQDisc::default();
            for opt in opts {
                match opt {
                    TcOption::FqCodel(fq_codel_opt) => match fq_codel_opt {
                        TcQdiscFqCodelOption::Target(target) => fq_codel.target = target,
                        TcQdiscFqCodelOption::Limit(limit) => fq_codel.limit = limit,
                        TcQdiscFqCodelOption::Interval(interval) => fq_codel.interval = interval,
                        TcQdiscFqCodelOption::Ecn(ecn) => fq_codel.ecn = ecn,
                        TcQdiscFqCodelOption::Flows(flows) => fq_codel.flows = flows,
                        TcQdiscFqCodelOption::Quantum(quantum) => fq_codel.quantum = quantum,
                        TcQdiscFqCodelOption::CeThreshold(ce_threshold) => {
                            fq_codel.ce_threshold = ce_threshold
                        }
                        TcQdiscFqCodelOption::DropBatchSize(drop_batch_size) => {
                            fq_codel.drop_batch_size = drop_batch_size
                        }
                        TcQdiscFqCodelOption::MemoryLimit(memory_limit) => {
                            fq_codel.memory_limit = memory_limit
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            return Some(Self::FqCodel(fq_codel));
        }
        None
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum XStats {
    FqCodel(FqCodelXStats),
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FqCodelQDisc {
    /// Accceptable minimum standing/persistent queue delay.
    pub target: u32,
    /// Hard limit on the real queue size.
    pub limit: u32,
    /// Used to ensure that the measured minimum delay does not become too stale.
    pub interval: u32,
    /// Used to mark packets instead of dropping them.
    pub ecn: u32,
    /// Number of flows into which the incoming packets are classified.
    pub flows: u32,
    /// Number of bytes used as 'deficit' in the fair queuing algorithm.
    pub quantum: u32,
    /// Sets a threshold above which all packets are marked with ECN Congestion Experienced.
    pub ce_threshold: u32,
    /// Sets the maximum number of packets to drop when limit or memory_limit is exceeded.
    pub drop_batch_size: u32,
    /// Sets a limit on the total number of bytes that can be queued in this FQ-CoDel instance.
    pub memory_limit: u32,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FqCodelXStats {
    /// Largest packet we've seen so far
    pub maxpacket: u32,
    /// Number of time max qdisc packet limit was hit.
    pub drop_overlimit: u32,
    /// Number of packets ECN marked instead of being dropped.
    pub ecn_mark: u32,
    /// Number of time packets created a 'new flow'.
    pub new_flow_count: u32,
    /// Count of flows in new list.
    pub new_flows_len: u32,
    /// Count of flows in old list.
    pub old_flows_len: u32,
    /// Packets above ce_threshold.
    pub ce_mark: u32,
    /// Memory usage (bytes).
    pub memory_usage: u32,
    /// Number of time packets were dropped due to memory limit.
    pub drop_overmemory: u32,
}

impl FqCodelXStats {
    pub fn new(xstats: &TcFqCodelXstats) -> Self {
        match xstats {
            TcFqCodelXstats::Qdisc(qdisc) => Self {
                maxpacket: qdisc.maxpacket,
                drop_overlimit: qdisc.drop_overlimit,
                ecn_mark: qdisc.ecn_mark,
                new_flow_count: qdisc.new_flow_count,
                new_flows_len: qdisc.new_flows_len,
                old_flows_len: qdisc.old_flows_len,
                ce_mark: qdisc.ce_mark,
                memory_usage: qdisc.memory_usage,
                drop_overmemory: qdisc.drop_overmemory,
            },
            _ => Self::default(),
        }
    }
}
