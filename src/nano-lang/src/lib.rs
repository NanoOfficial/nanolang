/**
 * @file lib.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-11
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

pub mod ast;
pub mod builtins;
pub mod expr;
pub mod format;
pub mod gen_uplc;
pub mod levenshtein;
pub mod parser;
pub mod pretty;
pub mod tipo;

#[derive(Debug, Default, Clone)]
pub struct IdGenerator {
    id: Arc<AtomicU64>,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }
}
