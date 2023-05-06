/**
 * @file encoder.rs
 * @author Krisna Pranav
 * @brief Encoder
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
 */


use crate::{encode::Encode, zigzag};
use super::Error;

pub struct Encoder {
    pub buffer: Vec<u8>,
    used_bits: i64,
    current_byte: u8,
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Encoder {
    pub fn new() -> Encoder {
        Encoder {
            buffer: Vec::new(),
            used_bits: 0,
            current_byte: 0,
        }
    }

    pub fn encode<T: Encode>(&mut self, x: T) -> Result<&mut Self, Error> {
        x.encode(self)?;

        Ok(self)
    }

    pub fn u8(&mut self, x: u8) -> Result<&mut Self, Error> {
        if self.used_bits == 0 {
            self.current_byte = x;
            self.next_word();
        } else {
            self.byte_unaligned(x);
        }

        Ok(self)
    }

    pub fn bool(&mut self, x: bool) -> &mut Self {
        if x {
            self.one();
        } else {
            self.zero();
        }

        self
    }

    pub fn bytes(&mut self, x: &[u8]) -> Result<&mut Self, Error> {
        self.filler();

        self.byte_array(x)
    }

    pub fn byte_array(&mut self, arr: &[u8]) -> Result<&mut Self, Error> {
        if self.used_bits != 0 {
            return Err(Error::BufferNotByteAligned);
        }

        self.write_blk(arr);

        Ok(self)
    }

    pub fn integer(&mut self, i: isize) -> &mut Self {
        let i = zigzag::to_usize(i);

        self.word(i);

        self
    }

    pub fn big_integer(&mut self, i: i128) -> &mut Self {
        let i = zigzag::to_u128(i);

        self.big_word(i);

        self
    }

    pub fn char(&mut self, c: char) -> &mut Self {
        self.word(c as usize);

        self
    }

    pub fn string(&mut self, s: &str) -> &mut Self {
        for i in s.chars() {
            self.one();
            self.char(i);
        }

        self.zero();

        self
    }

    pub fn utf8(&mut self, s: &str) -> Result<&mut Self, Error> {
        self.bytes(s.as_bytes())
    }

    pub fn word(&mut self, c: usize) -> &mut Self {
        let mut d = c;
        loop {
            let mut w = (d & 127) as u8;
            d >>= 7;

            if d != 0 {
                w |= 128;
            }
            self.bits(8, w);

            if d == 0 {
                break;
            }
        }

        self
    }

    pub fn big_word(&mut self, c: u128) -> &mut Self {
        let mut d = c;
        loop {
            let mut w = (d & 127) as u8;
            d >>= 7;

            if d != 0 {
                w |= 128;
            }
            self.bits(8, w);

            if d == 0 {
                break;
            }
        }

        self
    }

    pub fn encode_list_with<T>(
        &mut self,
        list: &[T],
        encoder_func: for<'r> fn(&T, &'r mut Encoder) -> Result<(), Error>,
    ) -> Result<&mut Self, Error>
    where
        T: Encode,
    {
        for item in list {
            self.one();
            encoder_func(item, self)?;
        }

        self.zero();

        Ok(self)
    }

    pub fn bits(&mut self, num_bits: i64, val: u8) -> &mut Self {
        match (num_bits, val) {
            (1, 0) => self.zero(),
            (1, 1) => self.one(),
            (2, 0) => {
                self.zero();
                self.zero();
            }
            (2, 1) => {
                self.zero();
                self.one();
            }
            (2, 2) => {
                self.one();
                self.zero();
            }
            (2, 3) => {
                self.one();
                self.one();
            }
            (_, _) => {
                self.used_bits += num_bits;
                let unused_bits = 8 - self.used_bits;
                match unused_bits {
                    x if x > 0 => {
                        self.current_byte |= val << x;
                    }
                    x if x == 0 => {
                        self.current_byte |= val;
                        self.next_word();
                    }
                    x => {
                        let used = -x;
                        self.current_byte |= val >> used;
                        self.next_word();
                        self.current_byte = val << (8 - used);
                        self.used_bits = used;
                    }
                }
            }
        }

        self
    }

    pub(crate) fn filler(&mut self) -> &mut Self {
        self.current_byte |= 1;
        self.next_word();

        self
    }

    fn zero(&mut self) {
        if self.used_bits == 7 {
            self.next_word();
        } else {
            self.used_bits += 1;
        }
    }

    fn one(&mut self) {
        if self.used_bits == 7 {
            self.current_byte |= 1;
            self.next_word();
        } else {
            self.current_byte |= 128 >> self.used_bits;
            self.used_bits += 1;
        }
    }

    fn byte_unaligned(&mut self, x: u8) {
        let x_shift = self.current_byte | (x >> self.used_bits);
        self.buffer.push(x_shift);

        self.current_byte = x << (8 - self.used_bits);
    }

    fn next_word(&mut self) {
        self.buffer.push(self.current_byte);

        self.current_byte = 0;
        self.used_bits = 0;
    }

    fn write_blk(&mut self, arr: &[u8]) {
        let chunks = arr.chunks(255);

        for chunk in chunks {
            self.buffer.push(chunk.len() as u8);
            self.buffer.extend(chunk);
        }
        self.buffer.push(0);
    }
}