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

pub mod remote_store;

use anyhow::Result;

use crate::{DataFrame, Format};

/// Serialize a data frame. As there is no support for Thrift in open source,
/// format is ignored and we serialize as CBOR.
pub fn serialize_frame(data: &DataFrame, _format: Format) -> Result<bytes::Bytes> {
    let bytes = serde_cbor::to_vec(data)?;
    Ok(bytes::Bytes::from(bytes))
}

/// Serialize a data frame. As there is no support for Thrift in open source,
/// format is ignored and we deserialize as CBOR.
pub fn deserialize_frame(bytes: &[u8], _format: Format) -> Result<DataFrame> {
    let data_frame = serde_cbor::from_slice(bytes)?;
    Ok(data_frame)
}
