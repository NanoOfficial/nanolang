/**
 * @file lib.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

pub mod ast;
pub mod builder;
pub mod builtins;

pub mod machine;
pub mod optimize;
pub mod parser;

pub mod tx;

mod debruijn;
mod flat;
mod pretty;

pub use pallas_codec::utils::KeyValuePairs;
pub use pallas_crypto::hash::Hash;
pub use pallas_primitives::{
    alonzo::{BigInt, Constr, PlutusData},
    babbage::{PostAlonzoTransactionOutput, TransactionInput, TransactionOutput, Value},
};

use pallas_primitives::{Error, Fragment};

pub fn plutus_data(bytes: &[u8]) -> Result<PlutusData, Error> {
    PlutusData::decode_fragment(bytes)
}

pub fn plutus_data_to_bytes(data: &PlutusData) -> Result<Vec<u8>, Error> {
    PlutusData::encode_fragment(data)
}