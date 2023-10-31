use super::*;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tc {
    pub index: u32,
    pub handle: u32,
    pub parent: u32,
    pub kind: String,
    pub stats: Stats,
    pub qdisc: Option<QDisc>,
}

impl Tc {
    pub fn new(tc: &tc::Tc) -> Self {
        let tc_stats = tc.attr.stats.as_ref();
        let tc_stats2 = tc.attr.stats2.as_ref();
        let bps = tc_stats.map(|stats| stats.bps);
        let pps = tc_stats.map(|stats| stats.pps);

        let mut stats = Stats {
            bps,
            pps,
            ..Default::default()
        };
        if let Some(basic) = tc_stats2.and_then(|s| s.basic.as_ref()) {
            stats.bytes = Some(basic.bytes);
            stats.packets = Some(basic.packets);
        }

        if let Some(queue) = tc_stats2.and_then(|s| s.queue.as_ref()) {
            stats.qlen = Some(queue.qlen);
            stats.backlog = Some(queue.backlog);
            stats.drops = Some(queue.drops);
            stats.requeues = Some(queue.requeues);
            stats.overlimits = Some(queue.overlimits);
        }

        stats.xstats = if let Some(xstats) = tc.attr.xstats.as_ref() {
            match xstats {
                tc::XStats::FqCodel(fq_codel_xstats) => {
                    Some(XStats::FqCodel(FqCodelXStats::new(fq_codel_xstats)))
                }
                _ => None,
            }
        } else {
            None
        };

        let qdisc = if let Some(tc_qdisc) = tc.attr.qdisc.as_ref() {
            match tc_qdisc {
                tc::QDisc::FqCodel(fq_codel) => Some(QDisc::FqCodel(FqCodelQDisc::new(fq_codel))),
                _ => None,
            }
        } else {
            None
        };

        Self {
            index: tc.msg.index,
            handle: tc.msg.handle,
            parent: tc.msg.parent,
            kind: tc.attr.kind.clone(),
            stats,
            qdisc,
        }
    }
}

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

impl FqCodelQDisc {
    pub fn new(fq_codel: &tc::FqCodel) -> Self {
        Self {
            target: fq_codel.target,
            limit: fq_codel.limit,
            interval: fq_codel.interval,
            ecn: fq_codel.ecn,
            flows: fq_codel.flows,
            quantum: fq_codel.quantum,
            ce_threshold: fq_codel.ce_threshold,
            drop_batch_size: fq_codel.drop_batch_size,
            memory_limit: fq_codel.memory_limit,
        }
    }
}

impl From<FqCodelQDisc> for QDisc {
    fn from(val: FqCodelQDisc) -> Self {
        QDisc::FqCodel(val)
    }
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
    pub fn new(xstats: &tc::FqCodelXStats) -> Self {
        Self {
            maxpacket: xstats.maxpacket,
            drop_overlimit: xstats.drop_overlimit,
            ecn_mark: xstats.ecn_mark,
            new_flow_count: xstats.new_flow_count,
            new_flows_len: xstats.new_flows_len,
            old_flows_len: xstats.old_flows_len,
            ce_mark: xstats.ce_mark,
            memory_usage: xstats.memory_usage,
            drop_overmemory: xstats.drop_overmemory,
        }
    }
}
