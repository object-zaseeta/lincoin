//! Structured errors (§12.2). Every failure the toolchain reports is one JSON object with the
//! required fields `kind`, `location`, `expected`, `actual`, and optional `suggestion`,
//! `counterexample`, `related`. AI agents read this; `--human` prose comes later.

use serde::Serialize;

use crate::ast::Span;
use crate::interp::RuntimeError;
use crate::lexer::LexError;
use crate::parser::ParseError;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#fn: Option<String>,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: String,
    pub location: Location,
    pub expected: String,
    pub actual: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("diagnostic serializes")
    }

    pub fn from_lex(file: &str, e: &LexError) -> Self {
        Diagnostic {
            kind: "syntax_error".into(),
            location: Location {
                file: file.into(),
                r#fn: None,
                line: e.line,
                col: e.col,
            },
            expected: "valid token".into(),
            actual: e.message.clone(),
            suggestion: None,
        }
    }

    pub fn from_parse(file: &str, e: &ParseError) -> Self {
        Diagnostic {
            kind: "syntax_error".into(),
            location: Location {
                file: file.into(),
                r#fn: None,
                line: e.line,
                col: e.col,
            },
            expected: e.expected.clone(),
            actual: e.actual.clone(),
            suggestion: None,
        }
    }

    pub fn io(file: &str, message: String) -> Self {
        Diagnostic {
            kind: "io_error".into(),
            location: Location {
                file: file.into(),
                r#fn: None,
                line: 0,
                col: 0,
            },
            expected: "readable source file".into(),
            actual: message,
            suggestion: None,
        }
    }

    pub fn from_runtime(file: &str, e: &RuntimeError) -> Self {
        let loc = |fn_name: &str, span: &Span| Location {
            file: file.into(),
            r#fn: Some(fn_name.into()),
            line: span.line,
            col: span.col,
        };
        match e {
            RuntimeError::Overflow { fn_name, span, expr } => Diagnostic {
                kind: "overflow".into(),
                location: loc(fn_name, span),
                expected: "Int32".into(),
                actual: expr.clone(),
                suggestion: Some("結果が Int32 の範囲(-2147483648..=2147483647)を超えました。Int64 に広げるか、値を小さくしてください".into()),
            },
            RuntimeError::DivisionByZero { fn_name, span, expr } => Diagnostic {
                kind: "division_by_zero".into(),
                location: loc(fn_name, span),
                expected: "Int32 where x != 0".into(),
                actual: expr.clone(),
                suggestion: Some("除数が 0 です。除算の前に 0 を検査するか、除数に篩型 `where x != 0` を付けてください".into()),
            },
            RuntimeError::UnknownInstruction { fn_name, span, op } => Diagnostic {
                kind: "unknown_instruction".into(),
                location: loc(fn_name, span),
                expected: "one of add sub mul div rem neg".into(),
                actual: op.clone(),
                suggestion: None,
            },
            RuntimeError::Arity { fn_name, span, op, expected, actual } => Diagnostic {
                kind: "arity_mismatch".into(),
                location: loc(fn_name, span),
                expected: format!("{op} with {expected} operand(s)"),
                actual: format!("{op} with {actual} operand(s)"),
                suggestion: None,
            },
            RuntimeError::UndefinedVariable { fn_name, span, name } => Diagnostic {
                kind: "undefined_variable".into(),
                location: loc(fn_name, span),
                expected: "a name bound earlier in this function".into(),
                actual: name.clone(),
                suggestion: None,
            },
            RuntimeError::LiteralOutOfRange { fn_name, span, literal } => Diagnostic {
                kind: "literal_out_of_range".into(),
                location: loc(fn_name, span),
                expected: "Int32".into(),
                actual: literal.clone(),
                suggestion: None,
            },
            RuntimeError::MissingRet { fn_name, span } => Diagnostic {
                kind: "missing_ret".into(),
                location: loc(fn_name, span),
                expected: "every path ends with `ret`".into(),
                actual: "function body ends without `ret`".into(),
                suggestion: None,
            },
            RuntimeError::NoMain => Diagnostic {
                kind: "no_main".into(),
                location: Location { file: file.into(), r#fn: None, line: 0, col: 0 },
                expected: "fn main() -> Int32".into(),
                actual: "no function named main".into(),
                suggestion: None,
            },
        }
    }
}
