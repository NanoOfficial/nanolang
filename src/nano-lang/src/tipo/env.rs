/**
 * @file env.rs
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

use crate::{
    ast::{
        Annotation, CallArg, DataType, Definition, Function, ModuleConstant, ModuleKind, Pattern,
        RecordConstructor, RecordConstructorArg, Span, TypeAlias, TypedDefinition,
        UnqualifiedImport, UntypedArg, UntypedDefinition, Use, Validator, PIPE_VARIABLE,
    },
    builtins::{self, function, generic_var, tuple, unbound_var},
    tipo::fields::FieldMap,
    IdGenerator,
};

use super::{
    error::{Error, Snippet, Warning},
    hydrator::Hydrator,
    AccessorsMap, PatternConstructor, RecordAccessor, Type, TypeConstructor, TypeInfo, TypeVar,
    ValueConstructor, ValueConstructorVariant,
};

#[derive(Debug)]
pub struct ScopeResetData {
    local_values: HashMap<String, ValueConstructor>,
}

#[derive(Debug)]
pub struct Environment<'a> {
    pub accessors: HashMap<String, AccessorsMap>,
    pub current_module: &'a String,
    pub entity_usages: Vec<HashMap<String, (EntityKind, Span, bool)>>,
    pub id_gen: IdGenerator,
    pub importable_modules: &'a HashMap<String, TypeInfo>,
    pub imported_modules: HashMap<String, (Span, &'a TypeInfo)>,
    pub imported_types: HashSet<String>,
    pub module_types: HashMap<String, TypeConstructor>,
    pub module_types_constructors: HashMap<String, Vec<String>>,
    pub module_values: HashMap<String, ValueConstructor>,

    previous_id: u64,

    pub scope: HashMap<String, ValueConstructor>,

    pub ungeneralised_functions: HashSet<String>,

    pub unqualified_imported_names: HashMap<String, Span>,

    pub unused_modules: HashMap<String, Span>,

    pub warnings: &'a mut Vec<Warning>,
}