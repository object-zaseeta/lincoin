//! Recursive-descent parser for the v0.1 subset (§16 EBNF, restricted to what step 0 needs).

use crate::ast::*;
use crate::lexer::{Tok, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: u32,
    pub col: u32,
    pub expected: String,
    pub actual: String,
}

const RESERVED: &[&str] = &[
    "fn", "pub", "type", "pool", "cap", "requires", "ensures", "example", "let", "ret", "err",
    "match", "if", "else", "with", "par", "scope", "spawn", "perf", "schedule", "layout", "use",
    "import", "program", "where",
];

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut p = Parser { tokens, pos: 0 };
    let program = p.program()?;
    p.skip_newlines();
    if p.pos < p.tokens.len() {
        return Err(p.error("end of input"));
    }
    Ok(program)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    // ----- token access -----

    fn peek(&self) -> Option<&'a Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn span(&self) -> Span {
        match self.tokens.get(self.pos) {
            Some(t) => Span {
                line: t.line,
                col: t.col,
            },
            None => match self.tokens.last() {
                // Just past the last token.
                Some(t) => Span {
                    line: t.line,
                    col: t.col + token_width(&t.tok),
                },
                None => Span { line: 1, col: 1 },
            },
        }
    }

    fn advance(&mut self) -> Option<&'a Tok> {
        let t = self.peek();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn error(&self, expected: &str) -> ParseError {
        let Span { line, col } = self.span();
        let actual = match self.peek() {
            Some(t) => describe(t),
            None => "end of input".to_string(),
        };
        ParseError {
            line,
            col,
            expected: expected.to_string(),
            actual,
        }
    }

    fn expect(&mut self, want: &Tok, expected: &str) -> Result<(), ParseError> {
        if self.peek() == Some(want) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(expected))
        }
    }

    fn keyword(&mut self, kw: &str) -> Result<(), ParseError> {
        match self.peek() {
            Some(Tok::Ident(s)) if s == kw => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(self.error(&format!("'{kw}'"))),
        }
    }

    fn at_keyword(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s == kw)
    }

    fn ident(&mut self, expected: &str) -> Result<String, ParseError> {
        match self.peek() {
            Some(Tok::Ident(s)) if !RESERVED.contains(&s.as_str()) => {
                self.pos += 1;
                Ok(s.clone())
            }
            _ => Err(self.error(expected)),
        }
    }

    fn type_ident(&mut self, expected: &str) -> Result<String, ParseError> {
        match self.peek() {
            Some(Tok::TypeIdent(s)) => {
                self.pos += 1;
                Ok(s.clone())
            }
            _ => Err(self.error(expected)),
        }
    }

    fn skip_newlines(&mut self) {
        while self.peek() == Some(&Tok::Newline) {
            self.pos += 1;
        }
    }

    /// A statement ends at a newline or at the closing brace of its block.
    fn end_of_line(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            Some(Tok::Newline) => {
                self.pos += 1;
                Ok(())
            }
            Some(Tok::RBrace) => Ok(()),
            _ => Err(self.error("end of line")),
        }
    }

    // ----- grammar -----

    fn program(&mut self) -> Result<Program, ParseError> {
        self.skip_newlines();
        let span = self.span();
        self.keyword("program")?;
        let name = self.ident("program name")?;
        self.skip_newlines();

        self.keyword("requires")?;
        self.expect(&Tok::LBracket, "'['")?;
        let mut requires = Vec::new();
        if self.peek() != Some(&Tok::RBracket) {
            loop {
                requires.push(self.type_ident("capability name")?);
                if self.peek() == Some(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        self.expect(&Tok::RBracket, "']'")?;
        self.skip_newlines();

        let doc = if self.at_keyword("doc") {
            self.pos += 1;
            match self.advance() {
                Some(Tok::Str(s)) => Some(s.clone()),
                _ => {
                    self.pos -= 1;
                    return Err(self.error("string"));
                }
            }
        } else {
            None
        };
        self.skip_newlines();

        self.expect(&Tok::LBrace, "'{'")?;
        self.skip_newlines();
        let mut fns = Vec::new();
        while self.at_keyword("fn") {
            fns.push(self.fn_def()?);
            self.skip_newlines();
        }
        self.expect(&Tok::RBrace, "'}' or 'fn'")?;
        Ok(Program {
            name,
            requires,
            doc,
            fns,
            span,
        })
    }

    fn fn_def(&mut self) -> Result<FnDef, ParseError> {
        let span = self.span();
        self.keyword("fn")?;
        let name = self.ident("function name")?;
        self.expect(&Tok::LParen, "'('")?;
        let mut params = Vec::new();
        if self.peek() != Some(&Tok::RParen) {
            loop {
                let pname = self.ident("parameter name")?;
                self.expect(&Tok::Colon, "':'")?;
                let ty = self.type_ident("type")?;
                params.push(Param { name: pname, ty });
                if self.peek() == Some(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        self.expect(&Tok::RParen, "')'")?;
        self.expect(&Tok::Arrow, "'->'")?;
        let ret_type = self.type_ident("return type")?;
        self.skip_newlines();

        let mut examples = Vec::new();
        while self.at_keyword("example") {
            examples.push(self.example()?);
            self.skip_newlines();
        }

        self.expect(&Tok::LBrace, "'{'")?;
        self.skip_newlines();
        let mut body = Vec::new();
        while self.peek() != Some(&Tok::RBrace) {
            body.push(self.stmt()?);
            self.skip_newlines();
        }
        self.expect(&Tok::RBrace, "'}'")?;
        Ok(FnDef {
            name,
            params,
            ret_type,
            examples,
            body,
            span,
        })
    }

    fn example(&mut self) -> Result<Example, ParseError> {
        let span = self.span();
        self.keyword("example")?;
        let fn_name = self.ident("function name")?;
        let mut args = Vec::new();
        while let Some(arg) = self.operand_opt() {
            args.push(arg);
        }
        let expect = if self.at_keyword("fails") {
            self.pos += 1;
            Expect::Fails
        } else {
            self.expect(&Tok::EqEq, "'==' or 'fails'")?;
            let value = self
                .operand_opt()
                .ok_or_else(|| self.error("expected value"))?;
            Expect::Value(value)
        };
        self.end_of_line()?;
        Ok(Example {
            fn_name,
            args,
            expect,
            span,
        })
    }

    fn stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.span();
        match self.peek() {
            Some(Tok::Ident(s)) if s == "ret" => {
                self.pos += 1;
                let value = self.operand_opt().ok_or_else(|| self.error("value"))?;
                self.end_of_line()?;
                Ok(Stmt::Ret { value, span })
            }
            Some(Tok::Ident(s)) if s == "let" => {
                self.pos += 1;
                let name = self.ident("name")?;
                self.bind_rest(name, span)
            }
            Some(Tok::Temp(name)) => {
                self.pos += 1;
                self.bind_rest(name.clone(), span)
            }
            _ => Err(self.error("'%name =', 'let', or 'ret'")),
        }
    }

    fn bind_rest(&mut self, name: String, span: Span) -> Result<Stmt, ParseError> {
        self.expect(&Tok::Eq, "'='")?;
        let ispan = self.span();
        let op = self.ident("instruction")?;
        let mut args = Vec::new();
        while let Some(arg) = self.operand_opt() {
            args.push(arg);
        }
        self.end_of_line()?;
        Ok(Stmt::Bind {
            name,
            instr: Instr {
                op,
                args,
                span: ispan,
            },
            span,
        })
    }

    /// `%name`, a plain (non-reserved) identifier, or an integer literal.
    fn operand_opt(&mut self) -> Option<Operand> {
        let op = match self.peek()? {
            Tok::Temp(s) => Operand::Var(s.clone()),
            Tok::Ident(s) if !RESERVED.contains(&s.as_str()) && s != "fails" => {
                Operand::Var(s.clone())
            }
            Tok::Int(s) => Operand::Int(s.clone()),
            _ => return None,
        };
        self.pos += 1;
        Some(op)
    }
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(s) | Tok::TypeIdent(s) | Tok::Int(s) => s.clone(),
        Tok::Temp(s) => format!("%{s}"),
        Tok::Str(s) => format!("{s:?}"),
        Tok::LBrace => "'{'".into(),
        Tok::RBrace => "'}'".into(),
        Tok::LParen => "'('".into(),
        Tok::RParen => "')'".into(),
        Tok::LBracket => "'['".into(),
        Tok::RBracket => "']'".into(),
        Tok::Arrow => "'->'".into(),
        Tok::Eq => "'='".into(),
        Tok::EqEq => "'=='".into(),
        Tok::Colon => "':'".into(),
        Tok::Comma => "','".into(),
        Tok::Newline => "end of line".into(),
    }
}

fn token_width(t: &Tok) -> u32 {
    match t {
        Tok::Ident(s) | Tok::TypeIdent(s) | Tok::Int(s) => s.chars().count() as u32,
        Tok::Temp(s) => s.chars().count() as u32 + 1,
        Tok::Str(s) => s.chars().count() as u32 + 2,
        Tok::Arrow | Tok::EqEq => 2,
        Tok::Newline => 0,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    const CALC: &str = r#"program calc
  requires []
  doc "固定式の電卓"
{
  fn main() -> Int32
    example main == 85
  {
    %1 = add 10 20
    %2 = mul %1 3
    %3 = sub %2 5
    ret %3
  }
}
"#;

    fn parse_src(src: &str) -> Result<Program, ParseError> {
        parse(&lex(src).unwrap())
    }

    #[test]
    fn parses_program_header() {
        let p = parse_src(CALC).unwrap();
        assert_eq!(p.name, "calc");
        assert_eq!(p.requires, Vec::<String>::new());
        assert_eq!(p.doc.as_deref(), Some("固定式の電卓"));
        assert_eq!(p.fns.len(), 1);
    }

    #[test]
    fn parses_function_signature_and_example() {
        let f = &parse_src(CALC).unwrap().fns[0];
        assert_eq!(f.name, "main");
        assert!(f.params.is_empty());
        assert_eq!(f.ret_type, "Int32");
        assert_eq!(
            f.examples,
            vec![Example {
                fn_name: "main".into(),
                args: vec![],
                expect: Expect::Value(Operand::Int("85".into())),
                span: Span { line: 6, col: 5 },
            }]
        );
    }

    #[test]
    fn parses_anf_body_as_binds_then_ret() {
        let f = &parse_src(CALC).unwrap().fns[0];
        assert_eq!(f.body.len(), 4);
        assert_eq!(
            f.body[0],
            Stmt::Bind {
                name: "1".into(),
                instr: Instr {
                    op: "add".into(),
                    args: vec![Operand::Int("10".into()), Operand::Int("20".into())],
                    span: Span { line: 8, col: 10 },
                },
                span: Span { line: 8, col: 5 },
            }
        );
        assert_eq!(
            f.body[1],
            Stmt::Bind {
                name: "2".into(),
                instr: Instr {
                    op: "mul".into(),
                    args: vec![Operand::Var("1".into()), Operand::Int("3".into())],
                    span: Span { line: 9, col: 10 },
                },
                span: Span { line: 9, col: 5 },
            }
        );
        assert_eq!(
            f.body[3],
            Stmt::Ret {
                value: Operand::Var("3".into()),
                span: Span { line: 11, col: 5 }
            }
        );
    }

    #[test]
    fn parses_let_binding_and_requires_list() {
        let src = "program p\n  requires [Stdin, Stdout]\n{\n  fn main() -> Int32\n  {\n    let x = neg 1\n    ret x\n  }\n}\n";
        let p = parse_src(src).unwrap();
        assert_eq!(p.requires, vec!["Stdin".to_string(), "Stdout".to_string()]);
        assert!(matches!(&p.fns[0].body[0], Stmt::Bind { name, .. } if name == "x"));
    }

    #[test]
    fn parses_params_and_fails_example() {
        let src = "program p\n  requires []\n{\n  fn safe_div(a: Int32, b: Int32) -> Int32\n    example safe_div 1 0 fails\n  {\n    %q = div a b\n    ret %q\n  }\n}\n";
        let f = &parse_src(src).unwrap().fns[0];
        assert_eq!(
            f.params,
            vec![
                Param {
                    name: "a".into(),
                    ty: "Int32".into()
                },
                Param {
                    name: "b".into(),
                    ty: "Int32".into()
                }
            ]
        );
        assert_eq!(
            f.examples[0].args,
            vec![Operand::Int("1".into()), Operand::Int("0".into())]
        );
        assert_eq!(f.examples[0].expect, Expect::Fails);
        assert!(
            matches!(&f.body[0], Stmt::Bind { instr, .. } if instr.args == vec![Operand::Var("a".into()), Operand::Var("b".into())])
        );
    }

    #[test]
    fn reports_missing_arrow_with_position_and_expectation() {
        let src = "program p\n  requires []\n{\n  fn main() Int32\n  {\n    ret 1\n  }\n}\n";
        let e = parse_src(src).unwrap_err();
        assert_eq!((e.line, e.col), (4, 13));
        assert_eq!(e.expected, "'->'");
        assert_eq!(e.actual, "Int32");
    }

    #[test]
    fn reports_unexpected_end_of_input() {
        let e = parse_src("program p\n  requires []\n{\n").unwrap_err();
        assert_eq!(e.actual, "end of input");
    }

    #[test]
    fn rejects_two_instructions_on_one_line() {
        let src = "program p\n  requires []\n{\n  fn main() -> Int32\n  {\n    %1 = add 1 2 ret %1\n  }\n}\n";
        let e = parse_src(src).unwrap_err();
        assert_eq!((e.line, e.col), (6, 18));
        assert_eq!(e.expected, "end of line");
    }
}
