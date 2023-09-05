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
