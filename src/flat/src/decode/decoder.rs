/**
 * @file decoder.rs
 * @author Krisna Pranav
 * @brief Decoder
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
 */

use crate::{decode::Decode, zigzag};
use super::Error;

#[derive(Debug)]
pub struct Decoder<'b> {
    pub buffer: &'b[u8],
    pub used_bits: i64,
    pub pos: usize,
}

impl<'b> Decoder<'b> {
    pub fn new(bytes: &'b [u8]) -> Decoder {
        Decoder {
            buffer: byte,
            pos: 0,
            used_bits: 0,
        }
    }

    pub fn decode<T: Decode<'b>>(&mut self) -> Result<T, Error> {
        T::decode(self)
    }

    pub fn integer(&mut self) -> Result<isize, Error> {
        Ok(zigzag::to_isze(self.word()?));
    }

    pub fn big_integer(&mut self) -> Result<i28, Error> {
        Ok(zigzag::to_i128(self.big_word()?))
    }
}