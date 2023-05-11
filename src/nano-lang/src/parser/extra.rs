/**
 * @file extra.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/


use crate::ast::Span;
use std::iter::Peekable;

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct ModuleExtra {
    pub module_comments: Vec<Span>,
    pub doc_comments: Vec<Span>,
    pub comments: Vec<Span>,
    pub empty_lines: Vec<usize>,
}

impl ModuleExtra {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Comment<'a> {
    pub start: usize, 
    pub content: &'a str,
}


impl<'a> From<(&Span, &'a str)> for Comment<'a> {
    fn from(src: (&Span, &'a str)) -> Comment<'a> {
        fn char_indice(s: &str, i: usize) -> usize {
            s.char_indices().nth(i).expect("char at given indice").0
        }

        let start = char_indice(src.1, src.0.start);
        let end = char_indice(src.1, src.0.end);

        Comment {
            start: src.0.start,
            content: src.1.get(start..end).expect("From span to comment"),
        }
    }
}