 /**
 * @file tx.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
 */


use pallas_primitives::{
    babbage::{CostMdls, MintedTx, Redeemer, TransactionInput, TransactionOutput},
    Fragment,
};
use pallas_traverse::{Era, MultiEraTx};

use error::Error;
pub use eval::get_script_and_datum_lookup_table;
pub use phase_one::eval_phase_one;
use script_context::{ResolvedInput, SlotConfig};

use crate::{
    ast::{DeBruijn, Program},
    machine::cost_model::ExBudget,
    PlutusData,
};

pub mod error;
mod eval;
mod phase_one;
pub mod script_context;
#[cfg(test)]
mod tests;
pub mod to_plutus_data;

pub fn eval_phase_two(
    tx: &MintedTx,
    utxos: &[ResolvedInput],
    cost_mdls: Option<&CostMdls>,
    initial_budget: Option<&ExBudget>,
    slot_config: &SlotConfig,
    run_phase_one: bool,
    with_redeemer: fn(&Redeemer) -> (),
) -> Result<Vec<Redeemer>, Error> {
    let redeemers = tx.transaction_witness_set.redeemer.as_ref();

    let lookup_table = get_script_and_datum_lookup_table(tx, utxos);

    if run_phase_one {
        eval_phase_one(tx, utxos, &lookup_table)?;
    }

    match redeemers {
        Some(rs) => {
            let mut collected_redeemers = vec![];

            let mut remaining_budget = *initial_budget.unwrap_or(&ExBudget::default());

            for redeemer in rs.iter() {
                with_redeemer(redeemer);

                let redeemer = eval::eval_redeemer(
                    tx,
                    utxos,
                    slot_config,
                    redeemer,
                    &lookup_table,
                    cost_mdls,
                    &remaining_budget,
                )?;

                remaining_budget.cpu -= redeemer.ex_units.steps as i64;
                remaining_budget.mem -= redeemer.ex_units.mem as i64;

                collected_redeemers.push(redeemer)
            }

            Ok(collected_redeemers)
        }
        None => Ok(vec![]),
    }
}

pub fn eval_phase_two_raw(
    tx_bytes: &[u8],
    utxos_bytes: &[(Vec<u8>, Vec<u8>)],
    cost_mdls_bytes: &[u8],
    initial_budget: (u64, u64),
    slot_config: (u64, u64, u32),
    run_phase_one: bool,
    with_redeemer: fn(&Redeemer) -> (),
) -> Result<Vec<Vec<u8>>, Error> {
    let multi_era_tx = MultiEraTx::decode(Era::Babbage, tx_bytes)
        .or_else(|_| MultiEraTx::decode(Era::Alonzo, tx_bytes))?;

    let cost_mdls = CostMdls::decode_fragment(cost_mdls_bytes)?;

    let budget = ExBudget {
        cpu: initial_budget.0 as i64,
        mem: initial_budget.1 as i64,
    };

    let mut utxos = Vec::new();

    for (input, output) in utxos_bytes {
        utxos.push(ResolvedInput {
            input: TransactionInput::decode_fragment(input)?,
            output: TransactionOutput::decode_fragment(output)?,
        });
    }

    let sc = SlotConfig {
        zero_time: slot_config.0,
        zero_slot: slot_config.1,
        slot_length: slot_config.2,
    };

    match multi_era_tx {
        MultiEraTx::Babbage(tx) => {
            match eval_phase_two(
                &tx,
                &utxos,
                Some(&cost_mdls),
                Some(&budget),
                &sc,
                run_phase_one,
                with_redeemer,
            ) {
                Ok(redeemers) => Ok(redeemers
                    .iter()
                    .map(|r| r.encode_fragment().unwrap())
                    .collect()),
                Err(err) => Err(err),
            }
        }
        _ => todo!("Wrong era. Please use babbage"),
    }
}

pub fn apply_params_to_script(
    params_bytes: &[u8], 
    plutus_script_bytes: &[u8],
) -> Result<Vec<u8>, Error> {
    let params = match PlutusData::decode_fragment(params_bytes).unwrap() {
        PlutusData::Array(res) => res,
        _ => unreachable!(),
    };

    let mut buffer = Vec::new();
    let mut program = Program::<DeBruijn>::from_cbor(plutus_script_bytes, &mut buffer)?;

    for param in params {
        program = program.apply_data(param);
    }

    match program.to_cbor() {
        Ok(res) => Ok(res),
        Err(_) => Err(Error::ApplyParamsError),
    }
}