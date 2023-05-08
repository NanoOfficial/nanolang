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