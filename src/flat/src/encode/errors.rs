use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Buffer is not byte algined")]
    BufferNotByteAligned,
    #[error("{0}")]
    Message(String),
}