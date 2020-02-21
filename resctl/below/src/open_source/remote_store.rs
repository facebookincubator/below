use std::time::SystemTime;

use anyhow::{bail, Result};

use crate::store;
use below_thrift::DataFrame;

pub struct RemoteStore {}

impl RemoteStore {
    pub fn new(_host: String, _port: Option<u16>) -> Result<RemoteStore> {
        bail!("Remote client not supported")
    }

    pub fn get_frame(
        &mut self,
        _timestamp: u64,
        _direction: store::Direction,
    ) -> Result<Option<(SystemTime, DataFrame)>> {
        bail!("Remote client not supported")
    }
}
