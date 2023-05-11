/**
 * @file pretty.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/


use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;

use super::{Type, TypeVar};
use crate::{
    docvec,
    pretty::{nil, *},
};

const INDENT: isize = 2;

// TODO: use references instead of cloning strings and vectors
#[derive(Debug, Default)]
pub struct Printer {
    names: HashMap<u64, String>,
    uid: u64,
    // A mapping of printd type names to the module that they are defined in.
    printed_types: HashMap<String, String>,
}

impl Printer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_names(&mut self, names: HashMap<u64, String>) {
        self.names = names;
    }

    /// Render a Type as a well formatted string.
    ///
    pub fn pretty_print(&mut self, typ: &Type, initial_indent: usize) -> String {
        let mut buffer = String::with_capacity(initial_indent);

        for _ in 0..initial_indent {
            buffer.push(' ');
        }

        buffer
            .to_doc()
            .append(self.print(typ))
            .nest(initial_indent as isize)
            .to_pretty_string(80)
    }

    // TODO: have this function return a Document that borrows from the Type.
    // Is this possible? The lifetime would have to go through the Arc<Refcell<Type>>
    // for TypeVar::Link'd types.
    pub fn print<'a>(&mut self, typ: &Type) -> Document<'a> {
        match typ {
            Type::App {
                name, args, module, ..
            } => {
                let doc = if self.name_clashes_if_unqualified(name, module) {
                    qualify_type_name(module, name)
                } else {
                    self.printed_types.insert(name.clone(), module.clone());
                    Document::String(name.clone())
                };
                if args.is_empty() {
                    doc
                } else {
                    doc.append("<")
                        .append(self.args_to_nano_doc(args))
                        .append(">")
                }
            }

            Type::Fn { args, ret } => "fn("
                .to_doc()
                .append(self.args_to_nano_doc(args))
                .append(") ->")
                .append(break_("", " ").append(self.print(ret)).nest(INDENT).group()),

            Type::Var { tipo: typ, .. } => self.type_var_doc(&typ.borrow()),

            Type::Tuple { elems, .. } => self.args_to_nano_doc(elems).surround("(", ")"),
        }
    }

    fn name_clashes_if_unqualified(&mut self, tipo: &String, module: &String) -> bool {
        match self.printed_types.get(tipo) {
            None => false,
            Some(previous_module) if module == previous_module => false,
            Some(_different_module) => true,
        }
    }

    fn type_var_doc<'a>(&mut self, typ: &TypeVar) -> Document<'a> {
        match typ {
            TypeVar::Link { tipo: ref typ, .. } => self.print(typ),
            TypeVar::Unbound { id, .. } | TypeVar::Generic { id, .. } => self.generic_type_var(*id),
        }
    }

    pub fn generic_type_var<'a>(&mut self, id: u64) -> Document<'a> {
        match self.names.get(&id) {
            Some(n) => {
                let typ_name = n.clone();

                self.printed_types.insert(typ_name, "".to_string());

                Document::String(n.clone())
            }
            None => {
                let n = self.next_letter();

                self.names.insert(id, n.clone());

                self.printed_types.insert(n.clone(), "".to_string());

                Document::String(n)
            }
        }
    }

    fn next_letter(&mut self) -> String {
        let alphabet_length = 26;
        let char_offset = 97;
        let mut chars = vec![];
        let mut n;
        let mut rest = self.uid;

        loop {
            n = rest % alphabet_length;

            rest /= alphabet_length;

            chars.push((n as u8 + char_offset) as char);

            if rest == 0 {
                break;
            }

            rest -= 1
        }

        self.uid += 1;

        chars.into_iter().rev().collect()
    }

    fn args_to_nano_doc<'a>(&mut self, args: &[Arc<Type>]) -> Document<'a> {
        if args.is_empty() {
            return nil();
        }

        let args = concat(Itertools::intersperse(
            args.iter().map(|t| self.print(t).group()),
            break_(",", ", "),
        ));

        break_("", "")
            .append(args)
            .nest(INDENT)
            .append(break_(",", ""))
            .group()
    }
}

fn qualify_type_name(module: &String, typ_name: &str) -> Document<'static> {
    if module.is_empty() {
        docvec!["aiken.", Document::String(typ_name.to_string())]
    } else {
        Document::String([module, typ_name].join("."))
    }
}

