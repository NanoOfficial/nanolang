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
    Alternative(Vec<String>),
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

                self.environment.insert_variable(
                    name.to_string(),
                    ValueConstructorVariant::LocalVariable { location },
                    typ,
                );
                Ok(())
            }

            PatternMode::Alternative(assigned) => {
                match self.environment.scope.get(name) {
                    Some(initial) if self.initial_pattern_vars.contains(name) => {
                        assigned.push(name.to_string());
                        let initial_typ = initial.tipo.clone();
                        self.environment
                            .unify(initial_typ, typ, err_location, false)
                    }

                    _ => Err(Error::ExtraVarInAlternativePattern {
                        name: name.to_string(),
                        location: err_location,
                    }),
                }
            }
        }
    }

    pub fn infer_alternative_pattern(
        &mut self,
        pattern: UntypedPattern,
        subject: &Type,
        location: &Span,
    ) -> Result<TypedPattern, Error> {
        self.mode = PatternMode::Alternative(vec![]);
        let typed_pattern = self.infer_pattern(pattern, subject)?;
        match &self.mode {
            PatternMode::Initial => panic!("Pattern mode switched from Alternative to Initial"),
            PatternMode::Alternative(assigned)
                if assigned.len() != self.initial_pattern_vars.len() =>
            {
                for name in assigned {
                    self.initial_pattern_vars.remove(name);
                }
                Err(Error::MissingVarInAlternativePattern {
                    location: *location,
                    name: self
                        .initial_pattern_vars
                        .iter()
                        .next()
                        .expect("Getting undefined pattern variable")
                        .clone(),
                })
            }
            PatternMode::Alternative(_) => Ok(typed_pattern),
        }
    }

    pub fn infer_pattern(
        &mut self,
        pattern: UntypedPattern,
        subject: &Type,
    ) -> Result<TypedPattern, Error> {
        self.unify(pattern, Arc::new(subject.clone()), None, false)
    }

    pub fn unify(
        &mut self,
        pattern: UntypedPattern,
        tipo: Arc<Type>,
        ann_type: Option<Arc<Type>>,
        is_assignment: bool,
    ) -> Result<TypedPattern, Error> {
        match pattern {
            Pattern::Discard { name, location } => {
                if is_assignment {
                    self.environment
                        .init_usage(name.to_string(), EntityKind::Variable, location);
                };
                Ok(Pattern::Discard { name, location })
            }

            Pattern::Var { name, location } => {
                self.insert_variable(&name, ann_type.unwrap_or(tipo), location, location)?;

                Ok(Pattern::Var { name, location })
            }

            Pattern::Assign {
                name,
                pattern,
                location,
            } => {
                self.insert_variable(
                    &name,
                    ann_type.clone().unwrap_or_else(|| tipo.clone()),
                    location,
                    pattern.location(),
                )?;

                let pattern = self.unify(*pattern, tipo, ann_type, false)?;

                Ok(Pattern::Assign {
                    name,
                    pattern: Box::new(pattern),
                    location,
                })
            }

            Pattern::Int { location, value } => {
                self.environment.unify(tipo, int(), location, false)?;

                Ok(Pattern::Int { location, value })
            }

            Pattern::List {
                location,
                elements,
                tail,
            } => match tipo.get_app_args(true, "", "List", 1, self.environment) {
                Some(args) => {
                    let tipo = args
                        .get(0)
                        .expect("Failed to get type argument of List")
                        .clone();

                    let elements = elements
                        .into_iter()
                        .map(|element| self.unify(element, tipo.clone(), None, false))
                        .try_collect()?;

                    let tail = match tail {
                        Some(tail) => Some(Box::new(self.unify(*tail, list(tipo), None, false)?)),
                        None => None,
                    };

                    Ok(Pattern::List {
                        location,
                        elements,
                        tail,
                    })
                }

                None => Err(Error::CouldNotUnify {
                    given: list(self.environment.new_unbound_var()),
                    expected: tipo.clone(),
                    situation: None,
                    location,
                    rigid_type_names: HashMap::new(),
                }),
            },

            Pattern::Tuple { elems, location } => match collapse_links(tipo.clone()).deref() {
                Type::Tuple { elems: type_elems } => {
                    if elems.len() != type_elems.len() {
                        return Err(Error::IncorrectTupleArity {
                            location,
                            expected: type_elems.len(),
                            given: elems.len(),
                        });
                    }

                    let mut patterns = vec![];

                    for (pattern, typ) in elems.into_iter().zip(type_elems) {
                        let typed_pattern = self.unify(pattern, typ.clone(), None, false)?;

                        patterns.push(typed_pattern);
                    }

                    Ok(Pattern::Tuple {
                        elems: patterns,
                        location,
                    })
                }

                Type::Var { .. } => {
                    let elems_types: Vec<_> = (0..(elems.len()))
                        .map(|_| self.environment.new_unbound_var())
                        .collect();

                    self.environment
                        .unify(tuple(elems_types.clone()), tipo, location, false)?;

                    let mut patterns = vec![];

                    for (pattern, type_) in elems.into_iter().zip(elems_types) {
                        let typed_pattern = self.unify(pattern, type_, None, false)?;

                        patterns.push(typed_pattern);
                    }

                    Ok(Pattern::Tuple {
                        elems: patterns,
                        location,
                    })
                }

                _ => {
                    let elems_types = (0..(elems.len()))
                        .map(|_| self.environment.new_unbound_var())
                        .collect();

                    Err(Error::CouldNotUnify {
                        given: tuple(elems_types),
                        expected: tipo,
                        situation: None,
                        location,
                        rigid_type_names: HashMap::new(),
                    })
                }
            },

            Pattern::Constructor {
                location,
                module,
                name,
                arguments: mut pattern_args,
                with_spread,
                is_record,
                ..
            } => {
                self.environment.increment_usage(&name);

                let cons =
                    self.environment
                        .get_value_constructor(module.as_ref(), &name, location)?;

                match cons.field_map() {
                    Some(field_map) => {
                        if with_spread {
                            if pattern_args.len() == field_map.arity {
                                return Err(Error::UnnecessarySpreadOperator {
                                    location: Span {
                                        start: location.end - 3,
                                        end: location.end - 1,
                                    },
                                    arity: field_map.arity,
                                });
                            }

                            let spread_location = Span {
                                start: location.end - 3,
                                end: location.end - 1,
                            };

                            let index_of_first_labelled_arg = pattern_args
                                .iter()
                                .position(|a| a.label.is_some())
                                .unwrap_or(pattern_args.len());

                            while pattern_args.len() < field_map.arity {
                                let new_call_arg = CallArg {
                                    value: Pattern::Discard {
                                        name: "_".to_string(),
                                        location: spread_location,
                                    },
                                    location: spread_location,
                                    label: None,
                                };

                                pattern_args.insert(index_of_first_labelled_arg, new_call_arg);
                            }
                        }

                        field_map.reorder(&mut pattern_args, location)?
                    }

                    None => assert_no_labeled_arguments(&pattern_args)
                        .map(|(location, label)| {
                            Err(Error::UnexpectedLabeledArgInPattern {
                                location,
                                label,
                                name: name.clone(),
                                args: pattern_args.clone(),
                                module: module.clone(),
                                with_spread,
                            })
                        })
                        .unwrap_or(Ok(()))?,
                }

                let constructor_typ = cons.tipo.clone();
                let constructor = match cons.variant {
                    ValueConstructorVariant::Record { ref name, .. } => {
                        PatternConstructor::Record {
                            name: name.clone(),
                            field_map: cons.field_map().cloned(),
                        }
                    }
                    ValueConstructorVariant::LocalVariable { .. }
                    | ValueConstructorVariant::ModuleConstant { .. }
                    | ValueConstructorVariant::ModuleFn { .. } => {
                        panic!("Unexpected value constructor type for a constructor pattern.",)
                    }
                };

                let instantiated_constructor_type = self.environment.instantiate(
                    constructor_typ,
                    &mut HashMap::new(),
                    self.hydrator,
                );
                match instantiated_constructor_type.deref() {
                    Type::Fn { args, ret } => {
                        if args.len() == pattern_args.len() {
                            let pattern_args = pattern_args
                                .into_iter()
                                .zip(args)
                                .map(|(arg, typ)| {
                                    let CallArg {
                                        value,
                                        location,
                                        label,
                                    } = arg;

                                    let value = self.unify(value, typ.clone(), None, false)?;

                                    Ok::<_, Error>(CallArg {
                                        value,
                                        location,
                                        label,
                                    })
                                })
                                .try_collect()?;

                            self.environment.unify(tipo, ret.clone(), location, false)?;

                            Ok(Pattern::Constructor {
                                location,
                                module,
                                name,
                                arguments: pattern_args,
                                constructor,
                                with_spread,
                                tipo: instantiated_constructor_type,
                                is_record,
                            })
                        } else {
                            Err(Error::IncorrectPatternArity {
                                location,
                                given: pattern_args,
                                expected: args.len(),
                                name: name.clone(),
                                module: module.clone(),
                                is_record,
                            })
                        }
                    }

                    Type::App { .. } => {
                        if pattern_args.is_empty() {
                            self.environment.unify(
                                tipo,
                                instantiated_constructor_type.clone(),
                                location,
                                false,
                            )?;

                            Ok(Pattern::Constructor {
                                location,
                                module,
                                name,
                                arguments: vec![],
                                constructor,
                                with_spread,
                                tipo: instantiated_constructor_type,
                                is_record,
                            })
                        } else {
                            Err(Error::IncorrectPatternArity {
                                location,
                                given: pattern_args,
                                expected: 0,
                                name: name.clone(),
                                module: module.clone(),
                                is_record,
                            })
                        }
                    }

                    _ => panic!("Unexpected constructor type for a constructor pattern.",),
                }
            }
        }
    }
}