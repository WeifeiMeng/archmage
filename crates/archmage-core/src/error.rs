#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("storage_failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("state_conflict: stale revision or invalid transition")]
    Conflict,
    #[error("preflight_failed: {0}")]
    Preflight(String),
    #[error("spawn_failed: {0}")]
    Spawn(String),
    #[error("timed_out")]
    TimedOut,
    #[error("resource_limit: {0}")]
    Limit(String),
    #[error("workspace_failed: {0}")]
    Workspace(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0} is not implemented in this batch")]
    NotImplemented(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;
