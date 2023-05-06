/**
 * @file errors.rs
 * @author Krisna Pranav
 * @brief Error Codes
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
 */

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Buffer is not byte aligned")]
    BufferNotByteAligned,
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Custom(#[from] anyhow::Error),
}