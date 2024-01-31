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
    pub if_index: u32,
    pub if_name: String,
    pub handle: u32,
    pub parent: u32,
    pub kind: String,
    pub stats: Stats,
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
    pub bytes: Option<u64>,
    pub packets: Option<u32>,

    // Stats2::StatsQueue
    pub qlen: Option<u32>,
    pub backlog: Option<u32>,
    pub drops: Option<u32>,
    pub requeues: Option<u32>,
    pub overlimits: Option<u32>,

    // XStats
    pub xstats: Option<XStats>,

    pub bps: Option<u32>,
    pub pps: Option<u32>,
}

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
    pub target: u32,
    pub limit: u32,
    pub interval: u32,
    pub ecn: u32,
    pub flows: u32,
    pub quantum: u32,
    pub ce_threshold: u32,
    pub drop_batch_size: u32,
    pub memory_limit: u32,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FqCodelXStats {
    pub maxpacket: u32,
    pub drop_overlimit: u32,
    pub ecn_mark: u32,
    pub new_flow_count: u32,
    pub new_flows_len: u32,
    pub old_flows_len: u32,
    pub ce_mark: u32,
    pub memory_usage: u32,
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
