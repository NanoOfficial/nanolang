/**
 * @file error.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-06
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/

use crate::{ast::Span, parser::token::Token};
use indoc::formatdoc;
use miette::Diagnostic;
use owo_colors::{OwoColorize, Stream::Stdout};
use std::collections::HashSet;

#[derive(Debug, Clone, Diagnostic, thiserror::Error)]
#[error("{kind}\n")]
pub struct ParseError {
    pub kind: ErrorKind,
    #[label]
    pub span: Span,
    #[allow(dead_code)]
    while_parsing: Option<(Span, &'static str)>,
    expected: HashSet<Pattern>,
    label: Option<&'static str>,
}

impl ParseError {
    pub fn merge(mut self, other: Self) -> Self {
        for expected in other.expected.into_iter() {
            self.expected.insert(expected);
        }
        self
    }

    pub fn invalid_tuple_index(span: Span, index: String, suffix: Option<String>) -> Self {
        let hint = suffix.map(|suffix| format!("Did you mean '{index}{suffix}'?"));
        Self {
            kind: ErrorKind::InvalidTupleIndex { hint },
            span,
            while_parsing: None,
            expected: HashSet::new(),
            label: None,
        }
    }

    pub fn invalid_when_clause_guard(span: Span) -> Self {
        Self {
            kind: ErrorKind::InvalidWhenClause,
            span,
            while_parsing: None,
            expected: HashSet::new(),
            label: Some("invalid clause guard"),
        }
    }

    pub fn malformed_base16_string_literal(span: Span) -> Self {
        Self {
            kind: ErrorKind::MalformedBase16StringLiteral,
            span,
            while_parsing: None,
            expected: HashSet::new(),
            label: None,
        }
    }
}

impl PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.span == other.span && self.label == other.label
    }
}

impl<T: Into<Pattern>> chumsky::Error<T> for ParseError {
    type Span = Span;

    type Label = &'static str;

    fn expected_input_found<Iter: IntoIterator<Item = Option<T>>>(
        span: Self::Span,
        expected: Iter,
        found: Option<T>,
    ) -> Self {
        Self {
            kind: found
                .map(Into::into)
                .map(ErrorKind::Unexpected)
                .unwrap_or(ErrorKind::UnexpectedEnd),
            span,
            while_parsing: None,
            expected: expected
                .into_iter()
                .map(|x| x.map(Into::into).unwrap_or(Pattern::End))
                .collect(),
            label: None,
        }
    }

    fn with_label(mut self, label: Self::Label) -> Self {
        self.label.get_or_insert(label);
        self
    }

    fn merge(self, other: Self) -> Self {
        ParseError::merge(self, other)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Diagnostic, thiserror::Error)]
pub enum ErrorKind {
    #[error("Arrived at the end of the file unexpectedly.")]
    UnexpectedEnd,

    #[error("{0}")]
    #[diagnostic(help("{}", .0.help().unwrap_or_else(|| Box::new(""))))]
    Unexpected(Pattern),

    #[error("Discovered an invalid tuple index.")]
    #[diagnostic()]
    InvalidTupleIndex {
        #[help]
        hint: Option<String>,
    },

    #[error("Tripped over a malformed base16-encoded string literal.")]
    #[diagnostic(help("{}", formatdoc! {
        r#"You can declare literal bytearrays from base16-encoded (a.k.a. hexadecimal) string literals.

           For example:

             ┍━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
             │ {} my_policy_id {}
             │   #{}
        "#,
        "pub const".if_supports_color(Stdout, |s| s.bright_blue()),
        "=".if_supports_color(Stdout, |s| s.yellow()),
        "\"f4c9f9c4252d86702c2f4c2e49e6648c7cffe3c8f2b6b7d779788f50\""
            .if_supports_color(Stdout, |s| s.bright_purple())
    }))]
    MalformedBase16StringLiteral,

    #[error("Failed to understand a when clause guard.")]
    #[diagnostic(url(""))]
    #[diagnostic(help("{}", formatdoc! {
        r#"Clause guards are not as capable as standard expressions. While you can combine multiple clauses using '{operator_or}' and '{operator_and}', you can't do any arithmetic in there. They are mainly meant to compare pattern variables to some known constants using simple binary operators.

           For example, the following clauses are well-formed:

           {good}   (x, _) if x == 10 -> ...
           {good}   (_, y) if y > 0 && y < 10 -> ...
           {good}   (x, y) if x && (y > 0 || y < 10) -> ...

           However, those aren't:

           {bad}   (x, _) if x % 3 == 0 -> ...
           {bad}   (x, y) if x + y > 42 -> ...
        "#
        , operator_or = "||".if_supports_color(Stdout, |s| s.yellow())
        , operator_and = "&&".if_supports_color(Stdout, |s| s.yellow())
        , good = "✔️".if_supports_color(Stdout, |s| s.green())
        , bad = "✖️".if_supports_color(Stdout, |s| s.red())
    }))]
    InvalidWhenClause,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Diagnostic, thiserror::Error)]
pub enum Pattern {
    #[error("Found an unexpected char '{0:?}'.")]
    #[diagnostic(help("Try removing it! and try again"))]
    Char(char),
    #[error("Found an unexpected token '{0}'.")]
    #[diagnostic(help("Try removing it! and try again"))]
    Token(Token),
    #[error("Found an unexpected literal value.")]
    #[diagnostic(help("Try removing it! and try again"))]
    Literal,
    #[error("Found an unexpected type name.")]
    #[diagnostic(help("Try removing it! and try again"))]
    TypeIdent,
    #[error("Found an unexpected identifier.")]
    #[diagnostic(help("Try removing it! and try again"))]
    TermIdent,
    #[error("Found an unexpected end of input.")]
    End,
    #[error("Found a malformed list spread pattern.")]
    #[diagnostic(help("List spread in matches can use a discard '_' or var."))]
    Match,
    #[error("Found an out-of-bound byte literal.")]
    #[diagnostic(help("Bytes must be between 0-255."))]
    Byte,
    #[error("Found an unexpected pattern.")]
    #[diagnostic(help(
        "If no label is provided then only variables\nmatching a field name are allowed."
    ))]
    RecordPunning,
    #[error("Found an unexpected label.")]
    #[diagnostic(help("You can only use labels surrounded by curly braces"))]
    Label,
    #[error("Found an unexpected discard '_'.")]
    #[diagnostic(help("You can only use capture syntax with functions not constructors."))]
    Discard,
}

impl From<char> for Pattern {
    fn from(c: char) -> Self {
        Self::Char(c)
    }
}
impl From<Token> for Pattern {
    fn from(tok: Token) -> Self {
        Self::Token(tok)
    }
}