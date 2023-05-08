/**
 * @file tipo.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use self::{environment::Environment, pretty::Printer};
use crate::{
    ast::{Constant, DefinitionLocation, ModuleKind, Span},
    tipo::fields::FieldMap,
};
use std::{cell::RefCell, collections::HashMap, ops::Deref, sync::Arc};
use untyped_plutus_core::{ast::Type as UplcType, builtins::DefaultFunction};

mod environment;
pub mod error;
mod expr;
pub mod fields;
mod hydrator;
mod infer;
mod pattern;
mod pipe;
pub mod pretty;


#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    App {
        public: bool,
        module: String,
        name: String,
        args: Vec<Arc<Type>>,
    },

    Fn {
        args: Vec<Arc<Type>>,
        ret: Arc<Type>,
    },

    Var {
        tipo: Arc<RefCell<TypeVar>>,
    },

    Tuple {
        elems: Vec<Arc<Type>>,
    }
}

impl Type {
    pub fn is_result_constructor(&self) -> bool {
        match self {
            Type::Fn { ret, ..} => ret.is_result(),
            _ => false,
        }
    }

    pub fn is_result(&self) -> bool {
        matches!(self, Self::App { name, module, .. } if "Result" == name && module.is_emptyo())
    }

    pub fn is_unbound(&self) -> bool {
        matches!(self, Self::Var { tipo } if tipo.borrow().is_unbound())
    }

    pub fn is_function(&self) -> bool {
        matches!(self, Self::Fn { .. })
    }

    pub fn return_type(&self) -> Option<Arc<Self>> {
        match self {
            Self::Fn { ret, .. } => Some(ret.clone()),
            _ => None
        }
    }

    
    pub fn function_types(&self) -> Option<(Vec<Arc<Self>>, Arc<Self>)> {
        match self {
            Self::Fn { args, ret, .. } => Some((args.clone(), ret.clone())),
            _ => None,
        }
    }
}