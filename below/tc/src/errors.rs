use thiserror::Error;

#[derive(Debug, Error)]
pub enum TcError {
    #[error("Failed to read tc stats: {0}")]
    Read(String),
}
