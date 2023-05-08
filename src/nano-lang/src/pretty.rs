/**
 * @file pretty.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

#![allow(clippy::wrong_self_convention)]

use std::collections::VecDeque;
use itertools::Itertools;

#[macro_export]
macro_rules! docvec {
    () => {
        Document::Vec(Vec::new())
    };

    ($($x:expr), + $(,)?) => {
        Document::Vec(vec![$($x.to_doc()), +])
    }
}

pub trait Documtable<'a> {
    fn to_doc(self) -> Document<'a>;
}

impl<'a> Documtable<'a> for char {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

