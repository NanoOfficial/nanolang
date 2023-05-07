 /**
 * @file optimize.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
 */


use crate::{
    ast::{Name, NamedDeBruijn, Program},
    parser::interner::Interner,
};

pub mod shrinker;

pub fn nano_optimize_and_intern(program: Program<Name>) -> Program<Name> {
    let mut program = program.builtin_force_reduce();

    let mut interner = Interner::new();

    interner.program(&mut program);

    let program_named: Program<NamedDeBruijn> = program.try_into().unwrap();

    let program: Program<Name> = program_named.try_into().unwrap();

    program
        .lambda_reduce()
        .inline_reduce()
        .lambda_reduce()
        .inline_reduce()
        .force_delay_reduce()
        .wrap_data_reduce()
        .lambda_reduce()
        .inline_reduce()
}