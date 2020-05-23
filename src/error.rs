use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    InvalidOption(String),
    #[error("text is longer than maximum supported length {}", u32::MAX)]
    TextTooLong,
    #[error("index is invalid or incompatible with text")]
    InvalidIndex,
}
