use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No files were specified to watch")]
    NoFiles,
    #[error("Path has no filename")]
    MissingFilename,
    #[error("Error watching file: {0}")]
    WatchError(String),
    #[error("Error parsing file: {0}")]
    ParseError(String),
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::WatchError(err.to_string())
    }
}
