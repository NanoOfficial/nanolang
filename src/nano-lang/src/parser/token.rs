/**
 * @file token.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-11
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::fmt;

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum Token {
    Error(char),
    Name { name: String },
    Ordinal { index: u32 },
    UpName { name: String },
    DiscardName { name: String },
    Int { value: String },
    ByteString { value: String },
    String { value: String }, 
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let index_str;
        let s = match self {
            Token::Error(c) => {
                write!(f, "\"{c}\"")?;
                return Ok(());
            }
            Token::Name { name } => name,
            
        }
    }
}