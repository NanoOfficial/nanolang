 /**
 * @file fields.rs
 * @author Krisna Pranav
 * @brief Field
 * @version 0.1
 * @date 2023-05-11
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::collections::{HashMap, HashSet};
use itertools::Itertools; 
use super::error::{Error, UnknownLabels};
use crate::ast::{CallArg, Span};
 

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMap {
    pub arity: usize,
    pub fields: HashMap<String, (usize, Span)>,
    pub is_function: bool,
}

impl FieldMap {
    pub fn new(arity: usize, is_function: bool) -> Self {
        Self {
            arity,
            fields: HashMap::new(),
            is_function,
        }
    }

    pub fn insert(&mut self, label: String, index: usize, location: &Span) -> Result<(), Error> {
        match self.fields.insert(label.clone(), (index, *location)) {
            Some((_, location_other)) => {
                if self.is_function {
                    Err(Error::DuplicateArgument {
                        label,
                        location: *location,
                        duplicate_location: location_other,
                    })
                } else {
                    Err(Error::DuplicateArgument {
                        label,
                        location: *location,
                        duplicate_location: location_other
                    }) 
                }
            }
            None => Ok(()),
        }
    }

    
}