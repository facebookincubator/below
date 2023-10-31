mod errors;
mod types;

#[cfg(test)]
mod test;

use std::collections::BTreeMap;

use netlink_tc as tc;

pub use errors::*;
pub use types::*;

pub type TcStats = BTreeMap<u32, Vec<Tc>>;
pub type Result<T> = std::result::Result<T, errors::TcError>;

/// Get list of all `tc` qdiscs and classes.
pub fn tc_stats() -> Result<TcStats> {
    read_tc_stats::<tc::Netlink>()
}

fn read_tc_stats<T: tc::NetlinkConnection>() -> Result<TcStats> {
    let mut tc_map = BTreeMap::new();
    let tc_stats = tc::qdiscs::<T>().map_err(|e| TcError::Read(e.to_string()))?;
    for tc_stat in tc_stats {
        let tc = Tc::new(&tc_stat);
        tc_map.entry(tc.index).or_insert_with(Vec::new).push(tc);
    }
    Ok(tc_map)
}
