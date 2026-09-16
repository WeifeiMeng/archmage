#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("preflight_failed: {0}")]
    Preflight(String),
    #[error("spawn_failed: {0}")]
    Spawn(String),
    #[error("timed_out")]
    TimedOut,
    #[error("resource_limit: {0}")]
    Limit(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0} is not implemented in this batch")]
    NotImplemented(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;
