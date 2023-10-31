use super::*;

use tc::{QDisc, Tc, XStats};

macro_rules! rate {
    ($field:ident, $sample:ident, $last:ident, $target_type:ty) => {
        $last.and_then(|(l, d)| {
            let s = $sample.$field;
            let l = l.$field;
            count_per_sec!(s, l, d, $target_type)
        })
    };
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
pub struct TcModel {
    #[queriable(subquery)]
    pub tc: Vec<SingleTcModel>,
}

impl TcModel {
    pub fn new(
        sample: &BTreeMap<u32, Vec<Tc>>,
        last: Option<(&BTreeMap<u32, Vec<Tc>>, Duration)>,
    ) -> Self {
        let tc = sample
            .iter()
            .flat_map(|(dev, tcs)| {
                tcs.iter().enumerate().map(|(seq, tc)| {
                    SingleTcModel::new(
                        tc,
                        last.and_then(|(last, d)| {
                            last.get(dev).and_then(|tcs| tcs.get(seq)).map(|tc| (tc, d))
                        }),
                        *dev,
                    )
                })
            })
            .collect();

        Self { tc }
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
pub struct SingleTcModel {
    /// Index for identifying the interface
    pub dev: u32,
    /// Name of the qdisc or class
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
    pub fn new(sample: &tc::Tc, last: Option<(&tc::Tc, Duration)>, dev: u32) -> Self {
        let mut tc_model = SingleTcModel {
            dev,
            kind: sample.kind.clone(),
            ..Default::default()
        };

        let stats = &sample.stats;
        tc_model.qlen = stats.qlen;
        tc_model.bps = stats.bps;
        tc_model.pps = stats.pps;

        last.map(|(l, d)| {
            let last = &l.stats;
            tc_model.bytes_per_sec = count_per_sec!(stats.bytes, last.bytes, d, u64);
            tc_model.packets_per_sec = count_per_sec!(stats.packets, last.packets, d, u32);
            tc_model.backlog_per_sec = count_per_sec!(stats.backlog, last.backlog, d, u32);
            tc_model.drops_per_sec = count_per_sec!(stats.drops, last.drops, d, u32);
            tc_model.requeues_per_sec = count_per_sec!(stats.requeues, last.requeues, d, u32);
            tc_model.overlimits_per_sec = count_per_sec!(stats.overlimits, last.overlimits, d, u32);
        });

        if let Some(sample) = stats.xstats.as_ref() {
            let last =
                last.and_then(|(last, d)| last.stats.xstats.as_ref().map(|l| (l, d)));

            tc_model.xstats = Some(XStatsModel::new(sample, last));
        }

        if let Some(sample) = sample.qdisc.as_ref() {
            let last = last.and_then(|(last, d)| last.qdisc.as_ref().map(|l| (l, d)));

            tc_model.qdisc = Some(QDiscModel::new(sample, last));
        }

        tc_model
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
pub struct QDiscModel {
    #[queriable(subquery)]
    pub fq_codel: Option<FqCodelQDiscModel>,
}

impl QDiscModel {
    fn new(sample: &tc::QDisc, last: Option<(&tc::QDisc, Duration)>) -> Self {
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

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    below_derive::Queriable
)]
pub struct FqCodelQDiscModel {
    pub target: u32,
    pub limit: u32,
    pub interval: u32,
    pub ecn: u32,
    pub quantum: u32,
    pub ce_threshold: u32,
    pub drop_batch_size: u32,
    pub memory_limit: u32,
    pub flows_per_sec: Option<u32>,
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
            flows_per_sec: {
                last.and_then(|(l, d)| {
                    let last = Some((l, d));
                    rate!(flows, sample, last, u32)
                })
            },
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
    fn new(sample: &tc::XStats, last: Option<(&tc::XStats, Duration)>) -> Self {
        match sample {
            XStats::FqCodel(sample) => Self {
                fq_codel: {
                    last.map(|(l, d)| match l {
                        XStats::FqCodel(last) => {
                            let last = Some((last, d));
                            FqCodelXStatsModel::new(sample, last)
                        }
                    })
                },
            },
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
    fn new(sample: &tc::FqCodelXStats, last: Option<(&tc::FqCodelXStats, Duration)>) -> Self {
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
