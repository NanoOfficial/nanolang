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

pub struct Interner {
    identifiers: HashMap<String, Unique>,
    current: Unique,
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    pub fn new() -> Self {
        Interner {
            identifiers: HashMap::new(),
            current: Unique::new(0)
        }
    }

    pub fn program(&mut self, program: &mut Program<Name>) {
        self.term(&mut program.term);
    }

    pub fn term(&mut self, term: &mut Term<Name>) {
        match term {
            Term::Var(name) => {
                let name = Rc::make_mut(name);
                name.unique = self.intern(&name.text)
            }
            Term::Delay(term) => self.term(Rc::make_mut(term)),
            Term::Lambda {
                paramter_name,
                body,
            } => {
                let paramter_name = Rc::make_mut(paramter_name);
                paramter_name.unique = self.intern(&paramter_name.text);
                self.term(Rc::make_mut(body));
            }
        }

        fn intern(&mut self, text: &str) -> Unique {
            if let Some(u) = self.identifiers.get(text) {
                *u
            } else {
                let unique = self.current;
                self.identifiers.insert(text.to_string(), unique);
                self.current.increment();
                unique
            }
        }
    }
}