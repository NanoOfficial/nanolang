/**
 * @file pattern.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

use itertools::Itertools;

use super::{
    environment::{assert_no_labeled_arguments, collapse_links, EntityKind, Environment},
    error::Error,
    hydrator::Hydrator,
    PatternConstructor, Type, ValueConstructorVariant,
};
use crate::{
    ast::{CallArg, Pattern, Span, TypedPattern, UntypedPattern},
    builtins::{int, list, tuple},
};

pub struct PatternTyper<'a, 'b> {
    environment: &'a mut Environment<'b>,
    hydrator: &'a Hydrator,
    mode: PatternMode,
    initial_pattern_vars: HashSet<String>,
}

enum PatternMode {
    Initial,
    Alternative(Vec<String>)
}

impl<'a, 'b> PatternTyper<'a, 'b> {
    pub fn new(environment: &'a mut Environment<'b>, hydrator: &'a Hydrator) -> Self {
        Self {
            environment,
            hydrator,
            mode: PatternMode::Initial,
            initial_pattern_vars: HashSet::new(),
        }
    }

    fn insert_variable(
        &mut self,
        name: &str,
        typ: Arc<Type>,
        location: Span,
        err_location: Span,
    ) -> Result<(), Error> {
        match &mut self.mode {
            PatternMode::Initial => {
                self.environment
                    .init_usage(name.to_string(), EntityKind::Variable, location);

                if self.initial_pattern_vars.contains(name) {
                    return Err(Error::DuplicateVarInPattern {
                        name: name.to_string(),
                        location: err_location,
                    });
                }

                self.initial_pattern_vars.insert(name.to_string());

                Ok();
            }
        }
    }
}