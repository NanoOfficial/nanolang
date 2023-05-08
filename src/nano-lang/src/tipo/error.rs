/**
 * @file error.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use super::Type;
use crate::{
    ast::{Annotation, BinOp, CallArg, Span, UntypedPattern},
    expr::{self, UntypedExpr},
    format::Formatter,
    levenshtein,
    pretty::Documentable,
};
use indoc::formatdoc;
use miette::{Diagnostic, LabeledSpan};
use ordinal::Ordinal;
use owo_colors::{
    OwoColorize,
    Stream::{Stderr, Stdout},
};
use std::{collections::HashMap, fmt::Display, sync::Arc};

#[derive(Debug, thiserror::Error, Diagnostic, Clone)]
#[error("Something is wrong here..")]
pub struct Snippet {
    #[label]
    pub location: Span,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "some of the lables used in this expression is unkown, highlighed below"
)]
pub struct UnkownLabels {
    pub unkown: Vec<Span>,
    pub valid: Vec<String>,
    pub suppleid: Vec<String>,
}
