/**
 * @file error.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use crate::{ast::Span, parser::token::Token};
use indoc::formatdoc;
use miette::Diagnostic;
use owo_colors::{OwoColorize, Stream::Stdout};
use std::collections::HashSet;

#[derive(Debug, Clone, Diagnostic, thiserror::Error)]
#[error("{kind}\n")]
pub struct ParseError {
    pub kind: ErrorKind,
    #[label]
    pub span: Span,
    #[allow(dead_code)]
    while_parsing: Option<(Span, &'static str)>,
    expected: HashSet<Pattern>,
    label: Option<&'static str>,
}

impl ParseError {
    pub fn merge(mut self, other: Self) -> Self {
        for expected in other.expected.into_iter() {
            self.expected.insert(expected);
        }
        self
    }

    pub fn invalid_tuple_index(span: Span, index: String, suffix: Option<String>) -> Self {
        let hint = suffix.map(|suffix| format!("Did you mean: '{index}{suffix}'"));
        Self {
            kind: ErrorKind::InvalidTupleIndex { hint },
            span,
            while_parsing: None,
            expected: HashSet::new(),
            label: None,
        }
    }
}