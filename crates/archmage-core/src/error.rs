#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} is not implemented in this batch")]
    NotImplemented(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;
