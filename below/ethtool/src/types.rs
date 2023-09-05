use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EthtoolStats {
    pub nic: BTreeMap<String, NicStats>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NicStats {
    pub queue: Vec<QueueStats>,
    pub tx_timeout: Option<u64>,
    pub raw_stats: BTreeMap<String, u64>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct QueueStats {
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
    pub rx_count: Option<u64>,
    pub tx_count: Option<u64>,
    pub tx_missed_tx: Option<u64>,
    pub tx_unmask_interrupt: Option<u64>,
    pub raw_stats: BTreeMap<String, u64>,
}
