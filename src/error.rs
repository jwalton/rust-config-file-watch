use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error watching files: {0}")]
    WatchError(String),
    #[error("Load error: {0}")]
    LoadError(Box<dyn std::error::Error + Send + Sync>),
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::WatchError(err.to_string())
    }
}
