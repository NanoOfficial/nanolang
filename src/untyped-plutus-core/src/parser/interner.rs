/**
 * @file interner.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-07
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{collections::HashMap, rc::Rc};
use crate::ast::{Name, Program, Term, Unique};