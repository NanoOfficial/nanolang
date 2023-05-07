/**
 * @file ast.rs
 * @author Krisna Pranav
 * @brief AST functionalities
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use crate::{
    builtins::DefaultFunction,
    debruijn::{self, Converter},
    flat::Binder,
    machine::{
        cost_model::{initialize_cost_model, CostModel, ExBudget},
        eval_result::EvalResult,
        Machine,
    },
};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use pallas_addresses::{Network, ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart};
use pallas_primitives::{
    alonzo::{self as pallas, Constr, PlutusData},
    babbage::{self as cardano, Language},
};
use pallas_traverse::ComputeHash;
use serde::{
    self,
    de::{self, Deserialize, Deserializer, MapAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};
use std::{
    fmt::{self, Display},
    hash::{self, Hash},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Program<T> {
    pub version: (usize, usize, usize),
    pub term: Term<T>,
}

impl<T> Program<T>
where
    T: Clone,
{
    pub fn apply(&self, program: &Self) -> Self {
        let applied_term = Term::Apply {
            function: Rc::new(self.term.clone()),
            argument: Rc::new(program.term.clone()),
        };

        Program {
            version: self.version,
            term: applied_term,
        }
    }

    pub fn apply_term(&self, term: &Term<T>) -> Self {
        let applied_term = Term::Apply {
            function: Rc::new(self.term.clone()),
            argument: Rc::new(term.clone()),
        };

        Program {
            version: self.version,
            term: applied_term,
        }
    }

    pub fn apply_data(&self, plutus_data: PlutusData) -> Self {
        let applied_term = Term::Apply {
            function: Rc::new(self.term.clone()),
            argument: Rc::new(Term::Constant(Constant::Data(plutus_data).into())),
        };

        Program {
            version: self.version,
            term: applied_term,
        }
    }
}

impl<'a, T> Display for Program<T>
where
    T: Binder<'a>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_pretty())
    }
}

impl Serialize for Program<DeBruijn> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let cbor = self.to_cbor().unwrap();
        let mut s = serializer.serialize_struct("Program<DeBruijn>", 2)?;
        s.serialize_field("compiledCode", &hex::encode(&cbor))?;
        s.serialize_field("hash", &cardano::PlutusV2Script(cbor.into()).compute_hash())?;
        s.end()
    }
}

impl<'a> Deserialize<'a> for Program<DeBruijn> {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Fields {
            CompiledCode,
        }

        struct ProgramVisitor;

        impl<'a> Visitor<'a> for ProgramVisitor {
            type Value = Program<DeBruijn>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Program<Visitor>")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Program<DeBruijn>, V::Error>
            where
                V: MapAccess<'a>,
            {
                let mut compiled_code: Option<String> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::CompiledCode => {
                            if compiled_code.is_some() {
                                return Err(de::Error::duplicate_field("compiledCode"));
                            }
                            compiled_code = Some(map.next_value()?);
                        }
                    }
                }
                let compiled_code =
                    compiled_code.ok_or_else(|| de::Error::missing_field("compiledCode"))?;

                let mut cbor_buffer = Vec::new();
                let mut flat_buffer = Vec::new();

                Program::<DeBruijn>::from_hex(&compiled_code, &mut cbor_buffer, &mut flat_buffer)
                    .map_err(|e| {
                        de::Error::invalid_value(
                            de::Unexpected::Other(&format!("{e}")),
                            &"a base16-encoded CBOR-serialized UPLC program",
                        )
                    })
            }
        }

        const FIELDS: &[&str] = &["compiledCode"];
        deserializer.deserialize_struct("Program<DeBruijn>", FIELDS, ProgramVisitor)
    }
}

impl Program<DeBruijn> {
    pub fn address(&self, network: Network, delegation: ShelleyDelegationPart) -> ShelleyAddress {
        let cbor = self.to_cbor().unwrap();
        let validator_hash = cardano::PlutusV2Script(cbor.into()).compute_hash();
        ShelleyAddress::new(
            network,
            ShelleyPaymentPart::Script(validator_hash),
            delegation,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term<T> {
    Var(Rc<T>),
    Delay(Rc<Term<T>>),
    Lambda {
        parameter_name: Rc<T>,
        body: Rc<Term<T>>,
    },
    Apply {
        function: Rc<Term<T>>,
        argument: Rc<Term<T>>,
    },
    Constant(Rc<Constant>),
    Force(Rc<Term<T>>),
    Error,
    Builtin(DefaultFunction),
}

impl<T> Term<T> {
    pub fn is_unit(&self) -> bool {
        matches!(self, Term::Constant(c) if c.as_ref() == &Constant::Unit)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Term::Constant(c) if matches!(c.as_ref(), &Constant::Integer(_)))
    }
}

impl<'a, T> Display for Term<T>
where
    T: Binder<'a>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_pretty())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Integer(BigInt),
    ByteString(Vec<u8>),
    String(String),
    Unit,
    Bool(bool),
    ProtoList(Type, Vec<Constant>),
    ProtoPair(Type, Type, Rc<Constant>, Rc<Constant>),
    Data(PlutusData),
}

pub struct Data {}

impl Data {
    pub fn integer(i: BigInt) -> PlutusData {
        match i.to_i64() {
            Some(i) => PlutusData::BigInt(pallas::BigInt::Int(i.into())),
            None => {
                let (sign, bytes) = i.to_bytes_be();
                match sign {
                    num_bigint::Sign::Minus => {
                        PlutusData::BigInt(pallas::BigInt::BigNInt(bytes.into()))
                    }
                    _ => PlutusData::BigInt(pallas::BigInt::BigUInt(bytes.into())),
                }
            }
        }
    }

    pub fn bytestring(bytes: Vec<u8>) -> PlutusData {
        PlutusData::BoundedBytes(bytes.into())
    }

    pub fn map(kvs: Vec<(PlutusData, PlutusData)>) -> PlutusData {
        PlutusData::Map(kvs.into())
    }

    pub fn list(xs: Vec<PlutusData>) -> PlutusData {
        PlutusData::Array(xs)
    }

    pub fn constr(ix: u64, fields: Vec<PlutusData>) -> PlutusData {
        if ix < 7 {
            PlutusData::Constr(Constr {
                tag: 121 + ix,
                any_constructor: None,
                fields,
            })
        } else if ix < 128 {
            PlutusData::Constr(Constr {
                tag: 1280 + ix - 7,
                any_constructor: None,
                fields,
            })
        } else {
            PlutusData::Constr(Constr {
                tag: 102,
                any_constructor: Some(ix),
                fields,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    Integer,
    String,
    ByteString,
    Unit,
    List(Rc<Type>),
    Pair(Rc<Type>, Rc<Type>),
    Data,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Bool => write!(f, "bool"),
            Type::Integer => write!(f, "integer"),
            Type::String => write!(f, "string"),
            Type::ByteString => write!(f, "bytestring"),
            Type::Unit => write!(f, "unit"),
            Type::List(t) => write!(f, "list {t}"),
            Type::Pair(t1, t2) => write!(f, "pair {t1} {t2}"),
            Type::Data => write!(f, "data"),
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Name {
    pub text: String,
    pub unique: Unique,
}

impl Name {
    pub fn text(t: impl ToString) -> Name {
        Name {
            text: t.to_string(),
            unique: 0.into(),
        }
    }
}

impl hash::Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.unique.hash(state);
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.unique == other.unique
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub struct Unique(isize);

impl Unique {
    pub fn new(unique: isize) -> Self {
        Unique(unique)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl From<isize> for Unique {
    fn from(i: isize) -> Self {
        Unique(i)
    }
}

impl From<Unique> for isize {
    fn from(d: Unique) -> Self {
        d.0
    }
}

impl Display for Unique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Eq)]
pub struct NamedDeBruijn {
    pub text: String,
    pub index: DeBruijn,
}

impl PartialEq for NamedDeBruijn {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

#[derive(Debug, Clone)]
pub struct FakeNamedDeBruijn(pub(crate) NamedDeBruijn);

impl From<DeBruijn> for FakeNamedDeBruijn {
    fn from(d: DeBruijn) -> Self {
        FakeNamedDeBruijn(d.into())
    }
}

impl From<FakeNamedDeBruijn> for DeBruijn {
    fn from(d: FakeNamedDeBruijn) -> Self {
        d.0.into()
    }
}

impl From<FakeNamedDeBruijn> for NamedDeBruijn {
    fn from(d: FakeNamedDeBruijn) -> Self {
        d.0
    }
}

impl From<NamedDeBruijn> for FakeNamedDeBruijn {
    fn from(d: NamedDeBruijn) -> Self {
        FakeNamedDeBruijn(d)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct DeBruijn(usize);

impl DeBruijn {
    pub fn new(index: usize) -> Self {
        DeBruijn(index)
    }

    pub fn inner(&self) -> usize {
        self.0
    }
}

impl From<usize> for DeBruijn {
    fn from(i: usize) -> Self {
        DeBruijn(i)
    }
}

impl From<DeBruijn> for usize {
    fn from(d: DeBruijn) -> Self {
        d.0
    }
}

impl Display for DeBruijn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NamedDeBruijn> for DeBruijn {
    fn from(n: NamedDeBruijn) -> Self {
        n.index
    }
}

impl From<DeBruijn> for NamedDeBruijn {
    fn from(index: DeBruijn) -> Self {
        NamedDeBruijn {
            text: String::from("i"),
            index,
        }
    }
}

impl TryFrom<Program<Name>> for Program<NamedDeBruijn> {
    type Error = debruijn::Error;

    fn try_from(value: Program<Name>) -> Result<Self, Self::Error> {
        Ok(Program::<NamedDeBruijn> {
            version: value.version,
            term: value.term.try_into()?,
        })
    }
}

impl TryFrom<Term<Name>> for Term<NamedDeBruijn> {
    type Error = debruijn::Error;

    fn try_from(value: Term<Name>) -> Result<Self, debruijn::Error> {
        let mut converter = Converter::new();

        let term = converter.name_to_named_debruijn(&value)?;

        Ok(term)
    }
}

impl TryFrom<Program<Name>> for Program<DeBruijn> {
    type Error = debruijn::Error;

    fn try_from(value: Program<Name>) -> Result<Self, Self::Error> {
        Ok(Program::<DeBruijn> {
            version: value.version,
            term: value.term.try_into()?,
        })
    }
}

impl TryFrom<Term<Name>> for Term<DeBruijn> {
    type Error = debruijn::Error;

    fn try_from(value: Term<Name>) -> Result<Self, debruijn::Error> {
        let mut converter = Converter::new();

        let term = converter.name_to_debruijn(&value)?;

        Ok(term)
    }
}

impl TryFrom<Program<NamedDeBruijn>> for Program<Name> {
    type Error = debruijn::Error;

    fn try_from(value: Program<NamedDeBruijn>) -> Result<Self, Self::Error> {
        Ok(Program::<Name> {
            version: value.version,
            term: value.term.try_into()?,
        })
    }
}

impl TryFrom<Term<NamedDeBruijn>> for Term<Name> {
    type Error = debruijn::Error;

    fn try_from(value: Term<NamedDeBruijn>) -> Result<Self, debruijn::Error> {
        let mut converter = Converter::new();

        let term = converter.named_debruijn_to_name(&value)?;

        Ok(term)
    }
}

impl From<Program<NamedDeBruijn>> for Program<DeBruijn> {
    fn from(value: Program<NamedDeBruijn>) -> Self {
        Program::<DeBruijn> {
            version: value.version,
            term: value.term.into(),
        }
    }
}

impl From<Term<NamedDeBruijn>> for Term<DeBruijn> {
    fn from(value: Term<NamedDeBruijn>) -> Self {
        let mut converter = Converter::new();

        converter.named_debruijn_to_debruijn(&value)
    }
}

impl From<Program<NamedDeBruijn>> for Program<FakeNamedDeBruijn> {
    fn from(value: Program<NamedDeBruijn>) -> Self {
        Program::<FakeNamedDeBruijn> {
            version: value.version,
            term: value.term.into(),
        }
    }
}

impl From<Term<NamedDeBruijn>> for Term<FakeNamedDeBruijn> {
    fn from(value: Term<NamedDeBruijn>) -> Self {
        let mut converter = Converter::new();

        converter.named_debruijn_to_fake_named_debruijn(&value)
    }
}

impl TryFrom<Program<DeBruijn>> for Program<Name> {
    type Error = debruijn::Error;

    fn try_from(value: Program<DeBruijn>) -> Result<Self, Self::Error> {
        Ok(Program::<Name> {
            version: value.version,
            term: value.term.try_into()?,
        })
    }
}

impl TryFrom<Term<DeBruijn>> for Term<Name> {
    type Error = debruijn::Error;

    fn try_from(value: Term<DeBruijn>) -> Result<Self, debruijn::Error> {
        let mut converter = Converter::new();

        let term = converter.debruijn_to_name(&value)?;

        Ok(term)
    }
}

impl From<Program<DeBruijn>> for Program<NamedDeBruijn> {
    fn from(value: Program<DeBruijn>) -> Self {
        Program::<NamedDeBruijn> {
            version: value.version,
            term: value.term.into(),
        }
    }
}

impl From<Term<DeBruijn>> for Term<NamedDeBruijn> {
    fn from(value: Term<DeBruijn>) -> Self {
        let mut converter = Converter::new();

        converter.debruijn_to_named_debruijn(&value)
    }
}

impl From<Program<FakeNamedDeBruijn>> for Program<NamedDeBruijn> {
    fn from(value: Program<FakeNamedDeBruijn>) -> Self {
        Program::<NamedDeBruijn> {
            version: value.version,
            term: value.term.into(),
        }
    }
}

impl From<Term<FakeNamedDeBruijn>> for Term<NamedDeBruijn> {
    fn from(value: Term<FakeNamedDeBruijn>) -> Self {
        let mut converter = Converter::new();

        converter.fake_named_debruijn_to_named_debruijn(&value)
    }
}

impl Program<NamedDeBruijn> {
    pub fn eval(self, initial_budget: ExBudget) -> EvalResult {
        let mut machine = Machine::new(
            Language::PlutusV2,
            CostModel::default(),
            initial_budget,
            200,
        );

        let term = machine.run(self.term);

        EvalResult::new(term, machine.ex_budget, initial_budget, machine.logs)
    }

    pub fn eval_v1(self) -> EvalResult {
        let mut machine = Machine::new(Language::PlutusV1, CostModel::v1(), ExBudget::v1(), 200);

        let term = machine.run(self.term);

        EvalResult::new(term, machine.ex_budget, ExBudget::v1(), machine.logs)
    }

    pub fn eval_as(
        self,
        version: &Language,
        costs: &[i64],
        initial_budget: Option<&ExBudget>,
    ) -> EvalResult {
        let budget = match initial_budget {
            Some(b) => *b,
            None => ExBudget::default(),
        };

        let mut machine = Machine::new(
            version.clone(),
            initialize_cost_model(version, costs),
            budget,
            200, 
        );

        let term = machine.run(self.term);

        EvalResult::new(term, machine.ex_budget, budget, machine.logs)
    }
}

impl Program<DeBruijn> {
    pub fn eval(&self, initial_budget: ExBudget) -> EvalResult {
        let program: Program<NamedDeBruijn> = self.clone().into();

        program.eval(initial_budget)
    }
}

impl Term<NamedDeBruijn> {
    pub fn is_valid_script_result(&self) -> bool {
        !matches!(self, Term::Error)
    }
}
