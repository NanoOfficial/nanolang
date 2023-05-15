/**
 * @file parser.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{ops::Neg, rc::Rc, str::FromStr};
use crate::{
    ast::{Constant, Name, Program, Term, Type},
    builtins::DefaultFunction,
};

use interner::Interner;
use num_bigint::BigInt;
use pallas_primitives::{alonzo::PlutusData, Fragment};
use peg::{error::ParseError, str::LineCol};

pub mod interner;

pub fn program(src: &str) -> Result<Program<Name>, ParseError<LineCol>> {

    let mut interner = Interner::new();

    let mut program = uplc::program(src)?;

    interner.program(&mut program);

    Ok(program)
}

pub fn term(src: &str) -> Result<Term<Name>, ParseError<LineCol>> {
    let mut interner = Interner::new();

    let mut term = uplc::term(src)?;

    interner.term(&mut term);

    Ok(term)
}

fn list_sub_type(type_info: Option<&Type>) -> Option<&Type> {
    match type_info {
        Some(Type::List(t)) => Some(t),
        _ => None,
    }
}

fn pair_sub_type(type_info: Option<&Type>) -> Option<(&Type, &Type)> {
    match type_info {
        Some(Type::Pair(l, r)) => Some((l, r)),
        _ => None,
    }
}

pub fn escape(string: &str) -> String {
    string
        .chars()
        .flat_map(|c| match c {
            '\n' => vec!['\\', c],
            '\r' => vec!['\\', c],
            '\t' => vec!['\\', c],
            '\'' => vec!['\\', c],
            '\\' => vec!['\\', c],
            '"' => vec!['\\', c],
            _ => vec![c],
        })
        .collect::<String>()
}

peg::parser! {
    grammar uplc() for str {
        pub rule program() -> Program<Name>
          = _* "(" _* "program" _+ v:version() _+ t:term() _* ")" _* {
            Program {version: v, term: t}
          }

        rule version() -> (usize, usize, usize)
          = major:number() "." minor:number() "." patch:number()  {
            (major as usize, minor as usize, patch as usize)
          }

        pub rule term() -> Term<Name>
          = constant()
          / builtin()
          / var()
          / lambda()
          / apply()
          / delay()
          / force()
          / error()

        rule constant() -> Term<Name>
          = "(" _* "con" _+ con:(
            constant_integer()
            / constant_bytestring()
            / constant_string()
            / constant_unit()
            / constant_bool()
            / constant_data()
            / constant_list()
            / constant_pair()
            ) _* ")" {
            Term::Constant(con.into())
          }

        rule builtin() -> Term<Name>
          = "(" _* "builtin" _+ b:ident() _* ")" {
            Term::Builtin(DefaultFunction::from_str(&b).unwrap())
          }

        rule var() -> Term<Name>
          = n:name() { Term::Var(n.into()) }

        rule lambda() -> Term<Name>
          = "(" _* "lam" _+ parameter_name:name() _+ t:term() _* ")" {
            Term::Lambda { parameter_name: parameter_name.into(), body: Rc::new(t) }
          }

        #[cache_left_rec]
        rule apply() -> Term<Name>
          = "[" _* initial:term() _+ terms:(t:term() _* { t })+ "]" {
            terms
                .into_iter()
                .fold(initial, |lhs, rhs| Term::Apply {
                    function: Rc::new(lhs),
                    argument: Rc::new(rhs)
                })
          }

        rule delay() -> Term<Name>
          = "(" _* "delay" _* t:term() _* ")" { Term::Delay(Rc::new(t)) }

        rule force() -> Term<Name>
          = "(" _* "force" _* t:term() _* ")" { Term::Force(Rc::new(t)) }

        rule error() -> Term<Name>
          = "(" _* "error" _* ")" { Term::Error }

        rule constant_integer() -> Constant
          = "integer" _+ i:big_number() { Constant::Integer(i) }

        rule constant_bytestring() -> Constant
          = "bytestring" _+ bs:bytestring() { Constant::ByteString(bs) }

        rule constant_string() -> Constant
          = "string" _+ s:string() { Constant::String(s) }

        rule constant_bool() -> Constant
          = "bool" _+ b:boolean() { Constant::Bool(b) }

        rule constant_unit() -> Constant
          = "unit" _+ "()" { Constant::Unit }

        rule constant_data() -> Constant
          = "data" _+ d:data() { Constant::Data(d) }

        rule constant_list() -> Constant
          = "list" _* "<" _* t:type_info() _* ">" _+ ls:list(Some(&t)) {
            Constant::ProtoList(t, ls)
          }

        rule constant_pair() -> Constant
          = "pair" _* "<" _* l:type_info() _* "," r:type_info() _* ">" _+ p:pair(Some((&l, &r))) {
            Constant::ProtoPair(l, r, p.0.into(), p.1.into())
          }

        rule pair(type_info: Option<(&Type, &Type)>) -> (Constant, Constant)
          = "[" _* x:typed_constant(type_info.map(|t| t.0)) _* "," _* y:typed_constant(type_info.map(|t| t.1)) _* "]" { (x, y) }

        rule number() -> isize
          = n:$("-"* ['0'..='9']+) {? n.parse().or(Err("isize")) }

        rule big_number() -> BigInt
          = n:$("-"* ['0'..='9']+) {? (if n.starts_with('-') { BigInt::parse_bytes(&n.as_bytes()[1..], 10).map(|i| i.neg()) } else { BigInt::parse_bytes(n.as_bytes(), 10) }).ok_or("BigInt") }

        rule boolean() -> bool
          = b:$("True" / "False") { b == "True" }

        rule bytestring() -> Vec<u8>
          = "#" i:ident()* { hex::decode(String::from_iter(i)).unwrap() }

        rule string() -> String
          = "\"" s:character()* "\"" { String::from_iter(s) }

        rule character() -> char
          = "\\n"  { '\n' } 
          / "\\r"  { '\r' } 
          / "\\t"  { '\t' } 
          / "\\\"" { '\"' } 
          / "\\'"  { '\'' } 
          / "\\\\" { '\\' } 
          / [ ^ '"' ]
          / expected!("or any valid ascii character")

        rule data() -> PlutusData
          = "#" i:ident()* {
              PlutusData::decode_fragment(
                  hex::decode(String::from_iter(i)).unwrap().as_slice()
              ).unwrap()
            }

        rule list(type_info: Option<&Type>) -> Vec<Constant>
          = "[" _* xs:(typed_constant(type_info) ** (_* "," _*)) _* "]" { xs }

        rule typed_constant(type_info : Option<&Type>) -> Constant
          = "()" {?
              match type_info {
                Some(Type::Unit) => Ok(Constant::Unit),
                _ => Err("found 'Unit' instead of expected type")
              }
            }
          / b:boolean() {?
              match type_info {
                Some(Type::Bool) => Ok(Constant::Bool(b)),
                _ => Err("found 'Bool' instead of expected type")
              }
            }
          / n:big_number() {?
              match type_info {
                Some(Type::Integer) => Ok(Constant::Integer(n)),
                _ => Err("found 'Integer' instead of expected type")
              }
            }
          / bs:bytestring() {?
              match type_info {
                Some(Type::ByteString) => Ok(Constant::ByteString(bs)),
                _ => Err("found 'ByteString' instead of expected type")
              }
            }
          / s:string() {?
              match type_info {
                Some(Type::String) => Ok(Constant::String(s)),
                _ => Err("found 'String' instead of expected type")
              }
            }
          / s:data() {?
              match type_info {
                Some(Type::Data) => Ok(Constant::Data(s)),
                _ => Err("found 'Data' instead of expected type")
              }
            }
          / ls:list(list_sub_type(type_info)) {?
              match type_info {
                Some(Type::List(t)) => Ok(Constant::ProtoList(t.as_ref().clone(), ls)),
                _ => Err("found 'List' instead of expected type")
              }
            }
          / p:pair(pair_sub_type(type_info)) {?
              match type_info {
                Some(Type::Pair(l, r)) => Ok(Constant::ProtoPair(l.as_ref().clone(), r.as_ref().clone(), p.0.into(), p.1.into())),
                _ => Err("found 'Pair' instead of expected type")
              }
            }

        rule type_info() -> Type
          = _* "unit" { Type::Unit }
          / _* "bool" { Type::Bool }
          / _* "integer" { Type::Integer }
          / _* "bytestring" { Type::ByteString }
          / _* "string" { Type::String }
          / _* "data" { Type::Data }
          / _* "list" _* "<" _* t:type_info() _* ">" {
              Type::List(t.into())
            }
          / _* "pair" _* "<" l:type_info() "," r:type_info() ">" {
              Type::Pair(l.into(), r.into())
            }

        rule name() -> Name
          = text:ident() { Name { text, unique: 0.into() } }

        rule ident() -> String
          = i:['a'..='z' | 'A'..='Z' | '0'..='9' | '_']+ {
            String::from_iter(i)
          }

        rule _ = [' ' | '\n']
    }
}

