use super::*;

struct FakeNetlink;

impl tc::NetlinkConnection for FakeNetlink {
    fn new() -> std::result::Result<Self, tc::errors::NetlinkError>
    where
        Self: Sized {
        Ok(Self {})
    }

    fn qdiscs(&self) -> std::result::Result<Vec<tc::TcMsg>, tc::errors::NetlinkError> {
        Ok(
            vec![
                tc::TcMsg {
                    header: tc::TcHeader {
                        index: 2,
                        handle: 0,
                        parent: 2,
                    },
                    attrs: vec![
                        tc::TcAttr::Kind("fq_codel".to_string()),
                        tc::TcAttr::Options(vec![
                            tc::TcOption {
                                kind: 1,
                                bytes: vec![135, 19, 0, 0],
                            },
                            tc::TcOption {
                                kind: 2,
                                bytes: vec![0, 40, 0, 0],
                            },
                            tc::TcOption {
                                kind: 3,
                                bytes: vec![159, 134, 1, 0],
                            },
                            tc::TcOption {
                                kind: 4,
                                bytes: vec![1, 0, 0, 0],
                            },
                            tc::TcOption {
                                kind: 6,
                                bytes: vec![234, 5, 0, 0],
                            },
                            tc::TcOption {
                                kind: 8,
                                bytes: vec![64, 0, 0, 0],
                            },
                            tc::TcOption {
                                kind: 9,
                                bytes: vec![0, 0, 0, 2],
                            },
                            tc::TcOption {
                                kind: 5,
                                bytes: vec![0, 4, 0, 0],
                            },
                        ]),
                        tc::TcAttr::Stats(vec![
                            76, 222, 96, 2, 0, 0, 0, 0, 55, 135, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        ]),
                        tc::TcAttr::Stats2(vec![
                            tc::TcStats2::StatsBasic(vec![76, 222, 96, 2, 0, 0, 0, 0, 55, 135, 2, 0, 0, 0, 0, 0]),
                            tc::TcStats2::StatsQueue(vec![
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0,
                            ]),
                            tc::TcStats2::StatsApp(vec![
                                0, 0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 91, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ]),
                        ]),
                        tc::TcAttr::Xstats(vec![
                            0, 0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 91, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        ]),
                        tc::TcAttr::HwOffload(0),
                    ],
                }
        ])
    }

    fn classes(&self, _index: i32) -> std::result::Result<Vec<tc::TcMsg>, tc::errors::NetlinkError> {
        Ok(Vec::new())
    }

    fn links(&self) -> std::result::Result<Vec<tc::LinkMsg>, tc::errors::NetlinkError> {
        Ok(Vec::new())
    }
}

#[test]
fn test_tc_stats() {
    let tc_map = read_tc_stats::<FakeNetlink>().unwrap();

    let tc = tc_map.get(&2).unwrap().get(0).unwrap();
    assert_eq!(tc.index, 2);
    assert_eq!(tc.handle, 0);
    assert_eq!(tc.parent, 2);

    assert_eq!(tc.kind, "fq_codel");
    assert_eq!(tc.stats.bytes, Some(39902796));
    assert_eq!(tc.stats.packets, Some(165687));
    assert_eq!(tc.stats.qlen, Some(0));
    assert_eq!(tc.stats.bps, Some(0));
    assert_eq!(tc.stats.pps, Some(0));

    assert!(tc.qdisc.is_some());
    assert!(tc.stats.xstats.is_some());
}
