/**
 * @file value.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-07
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use std::{collections::VecDeque, ops::Deref, rc::Rc};

use num_bigint::BigInt;
use num_traits::Signed;
use pallas_primitives::babbage::{self as pallas, PlutusData};

use crate::{
    ast::{Constant, NamedDeBruijn, Term, Type},
    builtins::DefaultFunction,
};

use super::{runtime::BuiltinRuntime, Error};

pub(super) type Env = Rc<Vec<Value>>;

#[derive(Clone, Debug)]
pub enum Value {
    Con(Rc<Constant>),
    Delay(Rc<Term<NamedDeBruijn>>, Env),
    Lambda {
        parameter_name: Rc<NamedDeBruijn>,
        body: Rc<Term<NamedDeBruijn>>,
        env: Env,
    },
    Builtin {
        fun: DefaultFunction,
        runtime: BuiltinRuntime,
    },
}

impl Value {
    pub fn integer(n: BigInt) -> Self {
        let constant = Constant::Integer(n);

        Value::Con(constant.into())
    }

    pub fn bool(n: bool) -> Self {
        let constant = Constant::Bool(n);

        Value::Con(constant.into())
    }

    pub fn byte_string(n: Vec<u8>) -> Self {
        let constant = Constant::ByteString(n);

        Value::Con(constant.into())
    }

    pub fn string(n: String) -> Self {
        let constant = Constant::String(n);

        Value::Con(constant.into())
    }

    pub fn list(typ: Type, n: Vec<Constant>) -> Self {
        let constant = Constant::ProtoList(typ, n);

        Value::Con(constant.into())
    }

    pub fn data(d: PlutusData) -> Self {
        let constant = Constant::Data(d);

        Value::Con(constant.into())
    }

    pub(super) fn unwrap_integer(&self) -> &BigInt {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::Integer(integer) = inner.as_ref() else {unreachable!()};

        integer
    }

    pub(super) fn unwrap_byte_string(&self) -> &Vec<u8> {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::ByteString(byte_string) = inner.as_ref() else {unreachable!()};

        byte_string
    }

    pub(super) fn unwrap_string(&self) -> &String {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::String(string) = inner.as_ref() else {unreachable!()};

        string
    }

    pub(super) fn unwrap_bool(&self) -> &bool {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::Bool(condition) = inner.as_ref() else {unreachable!()};

        condition
    }

    pub(super) fn unwrap_pair(&self) -> (&Type, &Type, &Rc<Constant>, &Rc<Constant>) {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::ProtoPair(t1, t2, first, second) = inner.as_ref() else {unreachable!()};

        (t1, t2, first, second)
    }

    pub(super) fn unwrap_list(&self) -> (&Type, &Vec<Constant>) {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::ProtoList(t, list) = inner.as_ref() else {unreachable!()};

        (t, list)
    }

    pub(super) fn unwrap_constant(&self) -> &Constant {
        let Value::Con(item) = self else {unreachable!()};

        item.as_ref()
    }

    pub(super) fn unwrap_data_list(&self) -> &Vec<Constant> {
        let Value::Con(inner) = self else {unreachable!()};
        let Constant::ProtoList(Type::Data, list) = inner.as_ref() else {unreachable!()};

        list
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Con(i) if matches!(i.as_ref(), Constant::Integer(_)))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Con(b) if matches!(b.as_ref(), Constant::Bool(_)))
    }

    pub fn to_ex_mem(&self) -> i64 {
        match self {
            Value::Con(c) => match c.as_ref() {
                Constant::Integer(i) => {
                    if *i == 0.into() {
                        1
                    } else {
                        (integer_log2(i.abs()) / 64) + 1
                    }
                }
                Constant::ByteString(b) => {
                    if b.is_empty() {
                        1
                    } else {
                        ((b.len() as i64 - 1) / 8) + 1
                    }
                }
                Constant::String(s) => s.chars().count() as i64,
                Constant::Unit => 1,
                Constant::Bool(_) => 1,
                Constant::ProtoList(_, items) => items.iter().fold(0, |acc, constant| {
                    acc + Value::Con(constant.clone().into()).to_ex_mem()
                }),
                Constant::ProtoPair(_, _, l, r) => {
                    Value::Con(l.clone()).to_ex_mem() + Value::Con(r.clone()).to_ex_mem()
                }
                Constant::Data(item) => self.data_to_ex_mem(item),
            },
            Value::Delay(_, _) => 1,
            Value::Lambda { .. } => 1,
            Value::Builtin { .. } => 1,
        }
    }

    pub fn data_to_ex_mem(&self, data: &PlutusData) -> i64 {
        let mut stack: VecDeque<&PlutusData> = VecDeque::new();
        let mut total = 0;
        stack.push_front(data);
        while let Some(item) = stack.pop_front() {
            total += 4;
            match item {
                PlutusData::Constr(c) => {
                    let mut new_stack: VecDeque<&PlutusData> =
                        VecDeque::from_iter(c.fields.deref().iter());
                    new_stack.append(&mut stack);
                    stack = new_stack;
                }
                PlutusData::Map(m) => {
                    let mut new_stack: VecDeque<&PlutusData>;
                    new_stack = m.iter().fold(VecDeque::new(), |mut acc, d| {
                        acc.push_back(&d.0);
                        acc.push_back(&d.1);
                        acc
                    });

                    new_stack.append(&mut stack);
                    stack = new_stack;
                }
                PlutusData::BigInt(i) => {
                    let i = from_pallas_bigint(i);

                    total += Value::Con(Constant::Integer(i).into()).to_ex_mem();
                }
                PlutusData::BoundedBytes(b) => {
                    let byte_string: Vec<u8> = b.deref().clone();
                    total += Value::Con(Constant::ByteString(byte_string).into()).to_ex_mem();
                }
                PlutusData::Array(a) => {
                    let mut new_stack: VecDeque<&PlutusData> =
                        VecDeque::from_iter(a.deref().iter());
                    new_stack.append(&mut stack);
                    stack = new_stack;
                }
            }
        }
        total
    }

    pub fn expect_type(&self, r#type: Type) -> Result<(), Error> {
        let constant: Constant = self.clone().try_into()?;

        let constant_type = Type::from(&constant);

        if constant_type == r#type {
            Ok(())
        } else {
            Err(Error::TypeMismatch(r#type, constant_type))
        }
    }

    pub fn expect_list(&self) -> Result<(), Error> {
        let constant: Constant = self.clone().try_into()?;

        let constant_type = Type::from(&constant);

        if matches!(constant_type, Type::List(_)) {
            Ok(())
        } else {
            Err(Error::ListTypeMismatch(constant_type))
        }
    }

    pub fn expect_pair(&self) -> Result<(), Error> {
        let constant: Constant = self.clone().try_into()?;

        let constant_type = Type::from(&constant);

        if matches!(constant_type, Type::Pair(_, _)) {
            Ok(())
        } else {
            Err(Error::PairTypeMismatch(constant_type))
        }
    }
}

impl TryFrom<Value> for Type {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let constant: Constant = value.try_into()?;

        let constant_type = Type::from(&constant);

        Ok(constant_type)
    }
}

impl TryFrom<&Value> for Type {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let constant: Constant = value.try_into()?;

        let constant_type = Type::from(&constant);

        Ok(constant_type)
    }
}

impl TryFrom<Value> for Constant {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Con(constant) => Ok(constant.as_ref().clone()),
            rest => Err(Error::NotAConstant(rest)),
        }
    }
}

impl TryFrom<&Value> for Constant {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Con(constant) => Ok(constant.as_ref().clone()),
            rest => Err(Error::NotAConstant(rest.clone())),
        }
    }
}

fn integer_log2(i: BigInt) -> i64 {
    let (_, bytes) = i.to_bytes_be();
    match bytes.first() {
        None => unreachable!("empty number?"),
        Some(u) => (8 - u.leading_zeros() - 1) as i64 + 8 * (bytes.len() - 1) as i64,
    }
}

pub fn from_pallas_bigint(n: &pallas::BigInt) -> BigInt {
    match n {
        pallas::BigInt::Int(i) => i128::from(*i).into(),
        pallas::BigInt::BigUInt(bytes) => BigInt::from_bytes_be(num_bigint::Sign::Plus, bytes),
        pallas::BigInt::BigNInt(bytes) => BigInt::from_bytes_be(num_bigint::Sign::Minus, bytes),
    }
}

pub fn to_pallas_bigint(n: &BigInt) -> pallas::BigInt {
    if n.bits() <= 64 {
        let regular_int: i64 = n.try_into().unwrap();
        let pallas_int: pallas_codec::utils::Int = regular_int.into();

        pallas::BigInt::Int(pallas_int)
    } else if n.is_positive() {
        let (_, bytes) = n.to_bytes_be();
        pallas::BigInt::BigUInt(bytes.into())
    } else {
        let (_, bytes) = n.to_bytes_be();
        pallas::BigInt::BigNInt(bytes.into())
    }
}

