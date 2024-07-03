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

struct FakeEthtool;

impl EthtoolReadable for FakeEthtool {
    fn new(_: &str) -> Result<Self> {
        Ok(Self {})
    }

    fn stats(&self) -> Result<Vec<(String, u64)>> {
        let res = vec![
            ("tx_timeout", 10),
            ("suspend", 0),
            ("resume", 0),
            ("wd_expired", 0),
            ("interface_up", 2),
            ("interface_down", 1),
            ("admin_q_pause", 0),
            ("queue_0_tx_cnt", 73731),
            ("queue_0_tx_bytes", 24429449),
            ("queue_0_tx_queue_stop", 0),
            ("queue_0_tx_queue_wakeup", 0),
            ("queue_0_tx_napi_comp", 161484),
            ("queue_0_tx_tx_poll", 161809),
            ("queue_0_tx_bad_req_id", 0),
            ("queue_0_tx_missed_tx", 0),
            ("queue_0_tx_unmask_interrupt", 161484),
            ("queue_0_rx_cnt", 180759),
            ("queue_0_rx_bytes", 159884486),
            ("queue_0_rx_refil_partial", 0),
            ("queue_0_rx_csum_bad", 0),
            ("queue_0_rx_page_alloc_fail", 0),
            ("queue_1_tx_cnt", 24228),
            ("queue_1_tx_bytes", 6969177),
            ("queue_1_tx_queue_stop", 0),
            ("queue_1_tx_queue_wakeup", 0),
            ("queue_1_tx_napi_comp", 45365),
            ("queue_1_tx_tx_poll", 45366),
            ("queue_1_tx_bad_req_id", 0),
            ("queue_1_tx_llq_buffer_copy", 0),
            ("queue_1_tx_missed_tx", 0),
            ("queue_1_tx_unmask_interrupt", 45365),
            ("queue_1_rx_cnt", 25191),
            ("queue_1_rx_bytes", 6562216),
            ("queue_1_rx_refil_partial", 0),
            ("queue_1_rx_csum_bad", 0),
            ("queue_1_rx_page_alloc_fail", 0),
        ];
        Ok(res
            .iter()
            .map(|stat| (stat.0.to_string(), stat.1))
            .collect())
    }
}

#[cfg(test)]
#[test]
fn test_read_stats() {
    let reader = EthtoolReader {};

    let if_names = reader.read_interfaces();
    assert!(if_names.is_ok());

    let eth_stats = reader.read_stats::<FakeEthtool>();
    assert!(eth_stats.is_ok());

    let ethtool_stats = eth_stats.as_ref().unwrap();
    let nic_stats = ethtool_stats.nic.values().next();
    assert!(nic_stats.is_some());

    let stats = nic_stats.unwrap();
    assert_eq!(stats.tx_timeout, Some(10));
    assert!(!stats.raw_stats.is_empty());

    let queue_stats = stats.queue.first();
    assert!(queue_stats.is_some());

    let qstats = queue_stats.unwrap();
    assert_eq!(qstats.rx_bytes, Some(159884486));
    assert!(!qstats.raw_stats.is_empty());
}
