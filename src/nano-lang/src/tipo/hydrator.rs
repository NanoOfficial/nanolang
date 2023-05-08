/**
 * @file hydrator.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{collections::HashMap, sync::Arc};
use crate::{
    ast::Annotation,
    builtins::{function, tuple},
    tipo::Span,
};

use super::{
    environment::Environment,
    error::{Error, Warning},
    Type, TypeConstructor,
};

#[derive(Debug)]
pub struct hydrator {
    created_type_variables: HashMap<String, Arc<Type>>,
    rigid_type_names: HashMap<u64, String>,
    permit_new_type_variables: bool,
}

#[derive(Debug)]
pub struct ScopeResetData {
    created_type_variables: HashMap<String, Arc<Type>>,
    rigid_type_names: HashMap<u64, String>,
}

impl Default for Hydrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Hydrator {
    pub fn new() -> Hydrator {
        Hydrator {
            created_type_variables: HashMap::new(),
            rigid_type_names: HashMap::new(),
            permit_new_type_variables: true,
        }
    }

    fn do_type_from_annotation<'a>(
        &mut self,
        annotation: &'a Annotation,
        environment: &mut Environment,
        unbounds: &mut Vec<&'a Span>,
    ) -> Result<Arc<Type>, Error> {
        match annotation {
            Annotation::Constructor {
                location,
                module,
                name,
                arguments: args 
            } => {
                let mut argument_types = Vec::with_capacity(args.len());
                for t in args {
                    let typ = self.do_type_from_annotation(t, environment, unbounds)?;
                    
                }
            }
        }
    }
}