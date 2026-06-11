use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] toml::de::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error("{0}")]
    Other(String),
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Error::Parse(ref err) => Error::Parse(err.clone()),
            Error::Io(ref err) => Error::Io(io::Error::new(err.kind(), err.to_string())),
            Error::Utf8(ref err) => Error::Utf8(*err),
            Error::Other(msg) => Error::Other(msg.clone()),
        }
    }
}
