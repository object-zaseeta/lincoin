//! Syntax tree for the v0.1 subset implemented so far (step 0: `program` + `fn` + ANF body).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub name: String,
    /// Capability names listed in `requires [...]` (§6.2). Unused until phase 4.
    pub requires: Vec<String>,
    pub doc: Option<String>,
    pub fns: Vec<FnDef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: String,
    pub examples: Vec<Example>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: String,
}

/// `example f a b == v` or `example f a b fails` (§5.2). Parsed and kept; executed from step 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    pub fn_name: String,
    pub args: Vec<Operand>,
    pub expect: Expect,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expect {
    Value(Operand),
    Fails,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// `%name = instr args...` or `let name = instr args...`
    Bind {
        name: String,
        instr: Instr,
        span: Span,
    },
    /// `ret operand`
    Ret { value: Operand, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instr {
    pub op: String,
    pub args: Vec<Operand>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    /// `%1`, `%q`, or a `let` name — all live in one namespace
    Var(String),
    /// Integer literal text as written (`10`, `-5`, `0xFF`, `1_000`); typed by the checker
    Int(String),
}
