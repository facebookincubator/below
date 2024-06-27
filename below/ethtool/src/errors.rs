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

use std::alloc;

use nix::errno::Errno;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EthtoolError {
    #[error("Failed to open a socket, error={0:}")]
    SocketError(Errno),

    #[error("Failed to read interface names, error={0:}")]
    IfNamesReadError(Errno),

    #[error("Failed to initialize struct, error={0:}")]
    CStructInitError(#[from] alloc::LayoutError),

    #[error("Failed to read data from struct pointer")]
    CStructReadError(),

    #[error("Failed to read number of stats using ETHTOOL_GSSET_INFO, error={0:}")]
    GSSetInfoReadError(Errno),

    #[error("Failed to read names of stats using ETHTOOL_GSTRINGS, error={0:}")]
    GStringsReadError(Errno),

    #[error("Failed to read values of stats using ETHTOOL_GSTATS, error={0:}")]
    GStatsReadError(Errno),

    #[error("Failed to parse stats, error={0:}")]
    ParseError(String),

    #[error("Failed to allocate memory")]
    AllocationFailure(),
}
