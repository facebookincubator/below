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

use std::time::SystemTime;

use anyhow::{bail, Result};

use crate::Direction;
use below_thrift::DataFrame;

pub struct RemoteStore {}

impl RemoteStore {
    pub fn new(_host: String, _port: Option<u16>) -> Result<RemoteStore> {
        bail!("Remote client not supported")
    }

    pub fn get_frame(
        &mut self,
        _timestamp: u64,
        _direction: Direction,
    ) -> Result<Option<(SystemTime, DataFrame)>> {
        bail!("Remote client not supported")
    }
}
