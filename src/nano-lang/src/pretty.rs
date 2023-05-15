/**
 * @file pretty.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

#[allow(clippy::wrong_self_convention)]

use std::collections::VecDeque;

use itertools::Itertools;

#[macro_export]
macro_rules! docvec {
    () => {
        Document::Vec(Vec::new())
    };

    ($($x:expr),+ $(,)?) => {
        Document::Vec(vec![$($x.to_doc()),+])
    };
}

pub trait Documentable<'a> {
    fn to_doc(self) -> Document<'a>;
}

impl<'a> Documentable<'a> for char {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for &'a str {
    fn to_doc(self) -> Document<'a> {
        Document::Str(self)
    }
}

impl<'a> Documentable<'a> for isize {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for i64 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for usize {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for f64 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self:?}"))
    }
}

impl<'a> Documentable<'a> for u64 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self:?}"))
    }
}

impl<'a> Documentable<'a> for u32 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for u16 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for u8 {
    fn to_doc(self) -> Document<'a> {
        Document::String(format!("{self}"))
    }
}

impl<'a> Documentable<'a> for Document<'a> {
    fn to_doc(self) -> Document<'a> {
        self
    }
}

impl<'a> Documentable<'a> for Vec<Document<'a>> {
    fn to_doc(self) -> Document<'a> {
        Document::Vec(self)
    }
}

impl<'a, D: Documentable<'a>> Documentable<'a> for Option<D> {
    fn to_doc(self) -> Document<'a> {
        self.map(Documentable::to_doc).unwrap_or_else(nil)
    }
}

pub fn concat<'a>(docs: impl IntoIterator<Item = Document<'a>>) -> Document<'a> {
    Document::Vec(docs.into_iter().collect())
}

pub fn join<'a>(
    docs: impl IntoIterator<Item = Document<'a>>,
    separator: Document<'a>,
) -> Document<'a> {
    concat(Itertools::intersperse(docs.into_iter(), separator))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Document<'a> {
    Line(usize),
    ForceBroken(Box<Self>),
    Break {
        broken: &'a str,
        unbroken: &'a str,
        break_first: bool,
        kind: BreakKind,
    },

    Vec(Vec<Self>),

    Nest(isize, Box<Self>),

    Group(Box<Self>),

    String(String),

    Str(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Broken,
    Unbroken,

    ForcedBroken,
}

impl Mode {
    fn is_forced(&self) -> bool {
        matches!(self, Mode::ForcedBroken)
    }
}

fn fits(
    mut limit: isize,
    mut current_width: isize,
    mut docs: VecDeque<(isize, Mode, &Document<'_>)>,
) -> bool {
    loop {
        if current_width > limit {
            return false;
        };

        let (indent, mode, document) = match docs.pop_front() {
            Some(x) => x,
            None => return true,
        };

        match document {
            Document::ForceBroken(_) => {
                return false;
            }

            Document::Line(_) => return true,

            Document::Nest(i, doc) => docs.push_front((i + indent, mode, doc)),

            Document::Group(doc) if mode.is_forced() => docs.push_front((indent, mode, doc)),

            Document::Group(doc) => docs.push_front((indent, Mode::Unbroken, doc)),

            Document::Str(s) => limit -= s.len() as isize,

            Document::String(s) => limit -= s.len() as isize,

            Document::Break { unbroken, .. } => match mode {
                Mode::Broken | Mode::ForcedBroken => return true,
                Mode::Unbroken => current_width += unbroken.len() as isize,
            },

            Document::Vec(vec) => {
                for doc in vec.iter().rev() {
                    docs.push_front((indent, mode, doc));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakKind {
    Flex,
    Strict,
}

fn format(
    writer: &mut String,
    limit: isize,
    mut width: isize,
    mut docs: VecDeque<(isize, Mode, &Document<'_>)>,
) {
    while let Some((indent, mode, document)) = docs.pop_front() {
        match document {
            Document::Line(i) => {
                for _ in 0..*i {
                    writer.push('\n');
                }

                for _ in 0..indent {
                    writer.push(' ');
                }

                width = indent;
            }

            Document::Break {
                broken,
                unbroken,
                break_first,
                kind: BreakKind::Flex,
            } => {
                let unbroken_width = width + unbroken.len() as isize;

                if fits(limit, unbroken_width, docs.clone()) {
                    writer.push_str(unbroken);
                    width = unbroken_width;
                    continue;
                }

                if *break_first {
                    writer.push('\n');
                    for _ in 0..indent {
                        writer.push(' ');
                    }
                    writer.push_str(broken);
                } else {
                    writer.push_str(broken);
                    writer.push('\n');
                    for _ in 0..indent {
                        writer.push(' ');
                    }
                }

                width = indent;
            }

            Document::Break {
                broken,
                unbroken,
                break_first,
                kind: BreakKind::Strict,
            } => {
                width = match mode {
                    Mode::Unbroken => {
                        writer.push_str(unbroken);

                        width + unbroken.len() as isize
                    }

                    Mode::Broken | Mode::ForcedBroken if *break_first => {
                        writer.push('\n');

                        for _ in 0..indent {
                            writer.push(' ');
                        }

                        writer.push_str(broken);

                        indent
                    }

                    Mode::Broken | Mode::ForcedBroken => {
                        writer.push_str(broken);

                        writer.push('\n');

                        for _ in 0..indent {
                            writer.push(' ');
                        }

                        indent
                    }
                };
            }

            Document::String(s) => {
                width += s.len() as isize;

                writer.push_str(s);
            }

            Document::Str(s) => {
                width += s.len() as isize;

                writer.push_str(s);
            }

            Document::Vec(vec) => {
                for doc in vec.iter().rev() {
                    docs.push_front((indent, mode, doc));
                }
            }

            Document::Nest(i, doc) => {
                docs.push_front((indent + i, mode, doc));
            }

            Document::Group(doc) => {
                let mut group_docs = VecDeque::new();

                group_docs.push_front((indent, Mode::Unbroken, doc.as_ref()));

                if fits(limit, width, group_docs) {
                    docs.push_front((indent, Mode::Unbroken, doc));
                } else {
                    docs.push_front((indent, Mode::Broken, doc));
                }
            }

            Document::ForceBroken(document) => {
                docs.push_front((indent, Mode::ForcedBroken, document));
            }
        }
    }
}

pub fn nil<'a>() -> Document<'a> {
    Document::Vec(vec![])
}

pub fn line<'a>() -> Document<'a> {
    Document::Line(1)
}

pub fn lines<'a>(i: usize) -> Document<'a> {
    Document::Line(i)
}

pub fn break_<'a>(broken: &'a str, unbroken: &'a str) -> Document<'a> {
    Document::Break {
        broken,
        unbroken,
        kind: BreakKind::Strict,
        break_first: false,
    }
}

pub fn prebreak<'a>(broken: &'a str, unbroken: &'a str) -> Document<'a> {
    Document::Break {
        broken,
        unbroken,
        kind: BreakKind::Strict,
        break_first: true,
    }
}

pub fn flex_break<'a>(broken: &'a str, unbroken: &'a str) -> Document<'a> {
    Document::Break {
        broken,
        unbroken,
        kind: BreakKind::Flex,
        break_first: false,
    }
}

pub fn flex_prebreak<'a>(broken: &'a str, unbroken: &'a str) -> Document<'a> {
    Document::Break {
        broken,
        unbroken,
        kind: BreakKind::Flex,
        break_first: true,
    }
}

impl<'a> Document<'a> {
    pub fn group(self) -> Self {
        Self::Group(Box::new(self))
    }

    pub fn nest(self, indent: isize) -> Self {
        Self::Nest(indent, Box::new(self))
    }

    pub fn force_break(self) -> Self {
        Self::ForceBroken(Box::new(self))
    }

    pub fn append(self, second: impl Documentable<'a>) -> Self {
        match self {
            Self::Vec(mut vec) => {
                vec.push(second.to_doc());
                Self::Vec(vec)
            }
            first => Self::Vec(vec![first, second.to_doc()]),
        }
    }

    pub fn to_pretty_string(self, limit: isize) -> String {
        let mut buffer = String::new();

        self.pretty_print(limit, &mut buffer);

        buffer
    }

    pub fn surround(self, open: impl Documentable<'a>, closed: impl Documentable<'a>) -> Self {
        open.to_doc().append(self).append(closed)
    }

    pub fn pretty_print(&self, limit: isize, writer: &mut String) {
        let mut docs = VecDeque::new();

        docs.push_front((0, Mode::Unbroken, self));

        format(writer, limit, 0, docs);
    }

    pub fn is_empty(&self) -> bool {
        use Document::*;
        match self {
            Line(n) => *n == 0,
            String(s) => s.is_empty(),
            Str(s) => s.is_empty(),
            Break { broken, .. } => broken.is_empty(),
            ForceBroken(d) | Nest(_, d) | Group(d) => d.is_empty(),
            Vec(docs) => docs.iter().all(|d| d.is_empty()),
        }
    }
}