use std::collections::BTreeMap;

use netlink_packet_route::tc::TcAttribute;
use netlink_packet_route::tc::TcFqCodelQdStats;
use netlink_packet_route::tc::TcFqCodelXstats;
use netlink_packet_route::tc::TcHandle;
use netlink_packet_route::tc::TcHeader;
use netlink_packet_route::tc::TcMessage;
use netlink_packet_route::tc::TcOption;
use netlink_packet_route::tc::TcQdiscFqCodelOption;
use netlink_packet_route::tc::TcStats;
use netlink_packet_route::tc::TcStats2;
use netlink_packet_route::tc::TcStatsBasic;
use netlink_packet_route::tc::TcStatsQueue;
use netlink_packet_route::tc::TcXstats;

use crate::types::XStats;
use crate::FqCodelQDisc;
use crate::FqCodelQdStats;
use crate::FqCodelXStats;
use crate::QDisc;
use crate::Result;

fn fake_netlink_qdiscs() -> Result<Vec<TcMessage>> {
    let mut tc_msgs = Vec::new();

    let msg1 = TcMessage::from_parts(
        TcHeader {
            index: 2,
            handle: TcHandle::from(0),
            parent: TcHandle::from(2),
            ..Default::default()
        },
        vec![
            TcAttribute::Kind("fq_codel".to_string()),
            TcAttribute::Options(vec![
                TcOption::FqCodel(TcQdiscFqCodelOption::Target(4999u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::Limit(10240u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::Interval(99999u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::Ecn(1u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::Flows(1024u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::Quantum(1514u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::CeThreshold(0u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::DropBatchSize(64u32)),
                TcOption::FqCodel(TcQdiscFqCodelOption::MemoryLimit(33554432u32)),
            ]),
            TcAttribute::Stats({
                let mut stats = TcStats::default();
                stats.bytes = 39902796u64;
                stats.packets = 165687u32;
                stats.drops = 100u32;
                stats.overlimits = 200u32;
                stats.bps = 300u32;
                stats.pps = 400u32;
                stats.qlen = 500u32;
                stats.backlog = 10u32;
                stats
            }),
            TcAttribute::Stats2(vec![
                TcStats2::Basic({
                    let mut basic = TcStatsBasic::default();
                    basic.bytes = 39902796u64;
                    basic.packets = 165687u32;
                    basic
                }),
                TcStats2::Queue({
                    let mut queue = TcStatsQueue::default();
                    queue.qlen = 500u32;
                    queue.drops = 100u32;
                    queue.requeues = 50u32;
                    queue.overlimits = 20u32;
                    queue
                }),
            ]),
            TcAttribute::Xstats(TcXstats::FqCodel(TcFqCodelXstats::Qdisc({
                let mut fq_codel = TcFqCodelQdStats::default();
                fq_codel.maxpacket = 258;
                fq_codel.drop_overlimit = 0;
                fq_codel.ecn_mark = 0;
                fq_codel.new_flow_count = 91;
                fq_codel.new_flows_len = 0;
                fq_codel.old_flows_len = 0;
                fq_codel.ce_mark = 0;
                fq_codel.memory_usage = 0;
                fq_codel.drop_overmemory = 0;
                fq_codel
            }))),
        ],
    );
    tc_msgs.push(msg1);

    Ok(tc_msgs)
}

#[test]
fn test_tc_stats() {
    let ifaces = BTreeMap::from_iter(vec![(2, "eth0".to_string())]);
    let tc_map = crate::read_tc_stats(ifaces, &fake_netlink_qdiscs).unwrap();

    let tc = tc_map.first().unwrap();
    assert_eq!(tc.if_index, 2);
    assert_eq!(tc.handle, 0);
    assert_eq!(tc.parent, 2);

    assert_eq!(tc.kind, "fq_codel");
    assert_eq!(tc.stats.bytes, Some(39902796));
    assert_eq!(tc.stats.packets, Some(165687));
    assert_eq!(tc.stats.qlen, Some(500));
    assert_eq!(tc.stats.bps, Some(300));
    assert_eq!(tc.stats.pps, Some(400));

    // qdisc
    assert_eq!(
        tc.qdisc,
        Some(QDisc::FqCodel(FqCodelQDisc {
            target: 4999,
            limit: 10240,
            interval: 99999,
            ecn: 1,
            flows: 1024,
            quantum: 1514,
            ce_threshold: 0,
            drop_batch_size: 64,
            memory_limit: 33554432,
        }))
    );

    // xstats
    assert_eq!(
        tc.stats.xstats,
        Some(XStats::FqCodel(FqCodelXStats::FqCodelQdiscStats(
            FqCodelQdStats {
                maxpacket: 258,
                drop_overlimit: 0,
                ecn_mark: 0,
                new_flow_count: 91,
                new_flows_len: 0,
                old_flows_len: 0,
                ce_mark: 0,
                memory_usage: 0,
                drop_overmemory: 0,
            }
        )))
    );
}
