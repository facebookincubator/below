use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use tc::QDisc;
use tc::TcStat;
use tc::TcStats;
use tc::XStats;

use crate::Field;
use crate::FieldId;
use crate::Nameable;
use crate::Queriable;

/// rate! macro calculates the rate of a field for given sample and last objects.
/// It basically calls count_per_sec! macro after extracting the field from the objects.
macro_rules! rate {
    ($field:ident, $sample:ident, $last:ident, $target_type:ty) => {{
        $last.and_then(|(last, d)| {
            let s = $sample.$field;
            let l = last.$field;
            count_per_sec!(l, s, d, $target_type)
        })
    }};
}

#[below_derive::queriable_derives]
pub struct TcModel {
    #[queriable(subquery)]
    pub tc: Vec<SingleTcModel>,
}

impl TcModel {
    pub fn new(sample: &TcStats, last: Option<(&TcStats, Duration)>) -> Self {
        // Assumption: sample and last are always ordered
        let tc = match last {
            Some((last_tcs, d)) if last_tcs.len() == sample.len() => sample
                .iter()
                .zip(last_tcs.iter())
                .map(|(sample, last)| SingleTcModel::new(sample, Some((last, d))))
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };

        Self { tc }
    }
}

#[below_derive::queriable_derives]
pub struct SingleTcModel {
    /// Name of the interface
    pub interface: String,
    /// Name of the qdisc
    pub kind: String,

    pub qlen: Option<u32>,
    pub bps: Option<u32>,
    pub pps: Option<u32>,

    pub bytes_per_sec: Option<u64>,
    pub packets_per_sec: Option<u32>,
    pub backlog_per_sec: Option<u32>,
    pub drops_per_sec: Option<u32>,
    pub requeues_per_sec: Option<u32>,
    pub overlimits_per_sec: Option<u32>,

    #[queriable(subquery)]
    pub qdisc: Option<QDiscModel>,

    #[queriable(subquery)]
    pub xstats: Option<XStatsModel>,
}

impl Nameable for SingleTcModel {
    fn name() -> &'static str {
        "tc"
    }
}

impl SingleTcModel {
    pub fn new(sample: &TcStat, last: Option<(&TcStat, Duration)>) -> Self {
        let mut tc_model = SingleTcModel {
            interface: sample.if_name.clone(),
            kind: sample.kind.clone(),
            ..Default::default()
        };

        let stats = &sample.stats;
        tc_model.qlen = stats.qlen;
        tc_model.bps = stats.bps;
        tc_model.pps = stats.pps;

        if let Some((l, d)) = last {
            let last = &l.stats;
            tc_model.bytes_per_sec = count_per_sec!(last.bytes, stats.bytes, d, u64);
            tc_model.packets_per_sec = count_per_sec!(last.packets, stats.packets, d, u32);
            tc_model.backlog_per_sec = count_per_sec!(last.backlog, stats.backlog, d, u32);
            tc_model.drops_per_sec = count_per_sec!(last.drops, stats.drops, d, u32);
            tc_model.requeues_per_sec = count_per_sec!(last.requeues, stats.requeues, d, u32);
            tc_model.overlimits_per_sec = count_per_sec!(last.overlimits, stats.overlimits, d, u32);
        }

        if let Some(sample) = stats.xstats.as_ref() {
            let last = last.and_then(|(last, d)| last.stats.xstats.as_ref().map(|l| (l, d)));

            tc_model.xstats = XStatsModel::new(sample, last);
        }

        if let Some(sample) = sample.qdisc.as_ref() {
            let last = last.and_then(|(last, d)| last.qdisc.as_ref().map(|l| (l, d)));

            tc_model.qdisc = Some(QDiscModel::new(sample, last));
        }

        tc_model
    }
}

#[below_derive::queriable_derives]
pub struct QDiscModel {
    #[queriable(subquery)]
    pub fq_codel: Option<FqCodelQDiscModel>,
}

impl QDiscModel {
    fn new(sample: &QDisc, last: Option<(&QDisc, Duration)>) -> Self {
        match sample {
            QDisc::FqCodel(sample) => Self {
                fq_codel: {
                    last.map(|(l, d)| match l {
                        QDisc::FqCodel(last) => {
                            let last = Some((last, d));
                            FqCodelQDiscModel::new(sample, last)
                        }
                    })
                },
            },
        }
    }
}

#[below_derive::queriable_derives]
pub struct FqCodelQDiscModel {
    pub target: u32,
    pub limit: u32,
    pub interval: u32,
    pub ecn: u32,
    pub quantum: u32,
    pub ce_threshold: u32,
    pub drop_batch_size: u32,
    pub memory_limit: u32,
    pub flows: u32,
}

impl FqCodelQDiscModel {
    fn new(sample: &tc::FqCodelQDisc, last: Option<(&tc::FqCodelQDisc, Duration)>) -> Self {
        Self {
            target: sample.target,
            limit: sample.limit,
            interval: sample.interval,
            ecn: sample.ecn,
            quantum: sample.quantum,
            ce_threshold: sample.ce_threshold,
            drop_batch_size: sample.drop_batch_size,
            memory_limit: sample.memory_limit,
            flows: sample.flows,
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct XStatsModel {
    #[queriable(subquery)]
    pub fq_codel: Option<FqCodelXStatsModel>,
}

impl XStatsModel {
    fn new(sample: &XStats, last: Option<(&XStats, Duration)>) -> Option<Self> {
        match (sample, last) {
            (XStats::FqCodel(sample), Some((XStats::FqCodel(last), d))) => match (sample, last) {
                (
                    tc::FqCodelXStats::FqCodelQdiscStats(sample),
                    tc::FqCodelXStats::FqCodelQdiscStats(last),
                ) => Some(Self {
                    fq_codel: Some(FqCodelXStatsModel::new(sample, Some((last, d)))),
                }),
            },
            _ => None,
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct FqCodelXStatsModel {
    // FqCodelQdXStats
    pub maxpacket: u32,
    pub ecn_mark: u32,
    pub new_flows_len: u32,
    pub old_flows_len: u32,
    pub ce_mark: u32,
    pub drop_overlimit_per_sec: Option<u32>,
    pub new_flow_count_per_sec: Option<u32>,
    pub memory_usage_per_sec: Option<u32>,
    pub drop_overmemory_per_sec: Option<u32>,
}

impl FqCodelXStatsModel {
    fn new(sample: &tc::FqCodelQdStats, last: Option<(&tc::FqCodelQdStats, Duration)>) -> Self {
        Self {
            maxpacket: sample.maxpacket,
            ecn_mark: sample.ecn_mark,
            new_flows_len: sample.new_flows_len,
            old_flows_len: sample.old_flows_len,
            ce_mark: sample.ce_mark,
            drop_overlimit_per_sec: rate!(drop_overlimit, sample, last, u32),
            new_flow_count_per_sec: rate!(drop_overlimit, sample, last, u32),
            memory_usage_per_sec: rate!(drop_overlimit, sample, last, u32),
            drop_overmemory_per_sec: rate!(drop_overlimit, sample, last, u32),
        }
    }
}
