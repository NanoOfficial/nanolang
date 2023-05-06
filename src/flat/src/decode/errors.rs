/**
 * @file errors.rs
 * @author Krisna Pranav
 * @brief Errors[Decode]
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
 */

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Reached end of the buffer")]
    EndOfBuffer,
    #[error("Buffer is not byte aligned")]
    BufferNotByteAligned,
    #[error("Incorrect value of num bits, must be less than 9")]
    IncorrectNumBits,
    #[error("Not enough data available, required {0} bytes")]
    NotEnoughBytes(usize),
    #[error("Not enough data available, required {0} bits")]
    NotEnoughBits(usize),
    #[error(transparent)]
    DecodeUtf8(#[from] std::string::FromUtf8Error),
    #[error("Decoding u32 to char {0}")]
    DecodeChar(u32),
    #[error("{0}")]
    Message(String),
    #[error("Parse error: till now we parsed\n\n{0}\n\nand we ran into error: {1}")]
    ParseError(String, anyhow::Error),
    #[error("Unknown term constructor tag: {0}.\n\nHere are the buffer bytes ({1} preceding) {2}\n\nBuffer position is {3} and buffer length is {4}")]
    UnknownTermConstructor(u8, usize, String, usize, usize),
    #[error(transparent)]
    Custom(#[from] anyhow::Error),
}