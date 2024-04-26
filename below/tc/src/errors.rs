use thiserror::Error;

#[derive(Debug, Error)]
pub enum TcError {
    #[error("Netlink error: {0}")]
    Netlink(String),

    #[error("Read interfaces error: {0}")]
    ReadInterfaces(String),

    #[error("Failed to read tc stats: {0}")]
    Read(String),
}
