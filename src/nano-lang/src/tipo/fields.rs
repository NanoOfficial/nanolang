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
                    Err(Error::DuplicateField {
                        label,
                        location: *location,
                        duplicate_location: location_other,
                    })
                }
            }
            None => Ok(()),
        }
    }

    pub fn into_option(self) -> Option<Self> {
        if self.fields.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    pub fn reorder<A>(&self, args: &mut [CallArg<A>], location: Span) -> Result<(), Error> {
        let mut last_labeled_arguments_given: Option<&CallArg<A>> = None;
        let mut seen_labels = std::collections::HashSet::new();
        let mut unknown_labels = Vec::new();

        if self.arity != args.len() {
            return Err(Error::IncorrectFieldsArity {
                labels: self.incorrect_arity_labels(args),
                location,
                expected: self.arity,
                given: args.len(),
            });
        }

        for arg in args.iter() {
            match &arg.label {
                Some(_) => {
                    last_labeled_arguments_given = Some(arg);
                }

                None => {
                    if let Some(label) = last_labeled_arguments_given {
                        return Err(Error::PositionalArgumentAfterLabeled {
                            location: arg.location,
                            labeled_arg_location: label.location,
                        });
                    }
                }
            }
        }

        let mut i = 0;
        while i < args.len() {
            let label = &args.get(i).expect("Field indexing to get label").label;

            let (label, &location) = match label {
                Some(l) => (
                    l,
                    &args
                        .get(i)
                        .expect("Indexing in labelled field reordering")
                        .location,
                ),

                None => {
                    i += 1;
                    continue;
                }
            };

            let (position, duplicate_location) = match self.fields.get(label) {
                None => {
                    unknown_labels.push(location);
                    i += 1;
                    continue;
                }
                Some(&p) => p,
            };

            if position == i {
                seen_labels.insert(label.clone());
                i += 1;
            } else {
                if seen_labels.contains(label) {
                    return Err(Error::DuplicateArgument {
                        location,
                        duplicate_location,
                        label: label.to_string(),
                    });
                }

                seen_labels.insert(label.clone());

                args.swap(position, i);
            }
        }

        if unknown_labels.is_empty() {
            Ok(())
        } else {
            let valid = self.fields.keys().map(|t| t.to_string()).sorted().collect();

            Err(Error::UnknownLabels(vec![UnknownLabels {
                valid,
                unknown: unknown_labels,
                supplied: seen_labels.into_iter().collect(),
            }]))
        }
    }

    pub fn incorrect_arity_labels<A>(&self, args: &[CallArg<A>]) -> Vec<String> {
        let given: HashSet<_> = args.iter().filter_map(|arg| arg.label.as_ref()).collect();

        self.fields
            .keys()
            .cloned()
            .filter(|f| !given.contains(f))
            .sorted()
            .collect()
    }
}