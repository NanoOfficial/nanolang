/**
 * @file eval_result.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-07
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/


use crate::ast::{Constant, NamedDeBruijn, Term};
use super::{cost_model::ExBudget, Error};

pub struct EvalResult {
    result: Result<Term<NamedDeBruijn>, Error>,
    remaining_budget: ExBudget,
    initial_budget: ExBudget,
    logs: Vec<String>,
}

impl EvalResult {
    pub fn new(
        result: Result<Term<NamedDeBruijn>, Error>,
        remaining_budget: ExBudget,
        initial_budget: ExBudget,
        logs: Vec<String>,
    ) -> EvalResult {
        EvalResult {
            result,
            remaining_budget,
            initial_budget,
            logs,
        }
    }

    pub fn cost(&self) -> ExBudget {
        self.initial_budget - self.remaining_budget
    }

    pub fn logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    pub fn failed(&self) -> bool {
        matches!(self.result, Err(_))
            || matches!(self.result, Ok(Term::Error))
            || matches!(self.result, Ok(Term::Constant(ref con)) if matches!(con.as_ref(), Constant::Bool(false)))
    }

    pub fn result(self) -> Result<Term<NamedDeBruijn>, Error> {
        self.result
    }
}