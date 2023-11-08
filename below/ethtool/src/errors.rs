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
