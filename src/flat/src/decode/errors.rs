use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Reached end of the buffer")]
    EndOfBuffer,
    #[error("Buffer is not bye algined")]
    BufferNotByteAligned,
    #[error("Incorrect value of num bits, must be less than 9")]
    IncorrectNumBits,
    #[error("Not enough data available, require {0} bytes")]
    NotEnoughBytes(usize),
    #[error(transparent)]
    DecodeUtf8(#[from] std::string::FromUtf8Error),
    #[error("Decoding u23 to char {}0")]
    DecodeChar(u32),
}