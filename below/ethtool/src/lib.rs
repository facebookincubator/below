#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod errors;
mod ethtool_sys;
mod reader;
mod types;

mod test;

use std::collections::BTreeMap;
use std::collections::HashSet;

use errors::EthtoolError;
pub use reader::*;
pub use types::*;

pub type Result<T> = std::result::Result<T, errors::EthtoolError>;

/// Translate the name of a queue stat to a tuple of (queue_id, stat_name).
/// Returns None if the name is not a queue stat.
///
/// The queue stat name is expected to be in the format of "queue_{queue_id}_{stat_name}".
/// Other formats are as of now treated as non-queue stats.
///
/// # Examples
/// ```ignore
/// assert_eq!(parse_queue_stat("queue_0_rx_bytes"), Some((0, "rx_bytes")));
/// assert_eq!(parse_queue_stat("queue"), None);
/// assert_eq!(parse_queue_stat("queue_0"), None);
/// ```
fn parse_queue_stat(name: &str) -> Option<(usize, &str)> {
    if !name.starts_with("queue_") {
        return None;
    }

    let stat_segments: Vec<&str> = name.splitn(3, '_').collect();
    if stat_segments.len() != 3 {
        return None;
    }
    match stat_segments[1].parse::<usize>() {
        Ok(queue_id) => Some((queue_id, stat_segments[2])),
        Err(_) => None,
    }
}

fn insert_stat(stat: &mut QueueStats, name: &str, value: u64) {
    match name {
        "rx_bytes" => stat.rx_bytes = Some(value),
        "tx_bytes" => stat.tx_bytes = Some(value),
        "rx_cnt" => stat.rx_count = Some(value),
        "tx_cnt" => stat.tx_count = Some(value),
        "tx_missed_tx" => stat.tx_missed_tx = Some(value),
        "tx_unmask_interrupt" => stat.tx_unmask_interrupt = Some(value),
        _ => {
            stat.raw_stats.insert(name.to_string(), value);
        }
    };
}

fn translate_stats(stats: Vec<(String, u64)>) -> Result<NicStats> {
    let mut nic_stats = NicStats::default();
    let mut raw_stats = BTreeMap::new();
    let mut queue_stats_map = BTreeMap::new(); // we want the queue stats to be sorted by queue id
    for (name, value) in stats {
        match parse_queue_stat(&name) {
            Some((queue_id, stat)) => {
                let qstat = queue_stats_map
                    .entry(queue_id)
                    .or_insert_with(QueueStats::default);
                insert_stat(qstat, stat, value);
            }
            None => match name.as_str() {
                "tx_timeout" => nic_stats.tx_timeout = Some(value),
                other => {
                    raw_stats.insert(other.to_string(), value);
                }
            },
        }
    }

    let queue_stats = queue_stats_map.into_values().collect();

    nic_stats.queue = queue_stats;
    nic_stats.raw_stats = raw_stats;

    Ok(nic_stats)
}

pub struct EthtoolReader;

impl EthtoolReader {
    pub fn new() -> Self {
        Self {}
    }

    /// Read the list of interface names.
    pub fn read_interfaces(&self) -> Result<HashSet<String>> {
        let mut if_names = HashSet::new();
        match nix::ifaddrs::getifaddrs() {
            Ok(interfaces) => {
                for if_addr in interfaces {
                    if_names.insert(if_addr.interface_name);
                }
            }
            Err(errno) => {
                return Err(EthtoolError::IfNamesReadError(errno));
            }
        }
        Ok(if_names)
    }

    /// Read stats for a single NIC identified by `if_name`
    fn read_nic_stats<T: reader::EthtoolReadable>(&self, if_name: &str) -> Result<NicStats> {
        let ethtool = T::new(if_name)?;
        match ethtool.stats() {
            Ok(stats) => translate_stats(stats),
            Err(error) => Err(error),
        }
    }

    pub fn read_stats<T: reader::EthtoolReadable>(&self) -> Result<EthtoolStats> {
        let mut nic_stats_map = BTreeMap::new();
        let if_names = self.read_interfaces()?;
        for if_name in if_names {
            if let Ok(nic_stats) = self.read_nic_stats::<T>(&if_name) {
                nic_stats_map.insert(if_name.to_string(), nic_stats);
            }
        }

        Ok(EthtoolStats { nic: nic_stats_map })
    }
}
