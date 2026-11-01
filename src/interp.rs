//! Interpreter for the step-0 subset: Int32 arithmetic in ANF, stopping on overflow and
//! division by zero (§3.1, §8-1 of the design record).

use crate::ast::*;

/// Why execution stopped. Rendered as a structured error by the CLI (§12.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// Arithmetic left the Int32 range. `expr` is the instruction as evaluated (`add 2147483647 1`).
    Overflow {
        fn_name: String,
        span: Span,
        expr: String,
    },
    DivisionByZero {
        fn_name: String,
        span: Span,
        expr: String,
    },
    UnknownInstruction {
        fn_name: String,
        span: Span,
        op: String,
    },
    /// Wrong number of operands for a known instruction.
    Arity {
        fn_name: String,
        span: Span,
        op: String,
        expected: usize,
        actual: usize,
    },
    UndefinedVariable {
        fn_name: String,
        span: Span,
        name: String,
    },
    /// Literal does not fit the declared type (`Int32`).
    LiteralOutOfRange {
        fn_name: String,
        span: Span,
        literal: String,
    },
    /// A path reached the end of the body without `ret`.
    MissingRet {
        fn_name: String,
        span: Span,
    },
    NoMain,
}

pub fn run_main(program: &Program) -> Result<i32, RuntimeError> {
    let main = program
        .fns
        .iter()
        .find(|f| f.name == "main")
        .ok_or(RuntimeError::NoMain)?;
    run_fn(main)
}

fn run_fn(f: &FnDef) -> Result<i32, RuntimeError> {
    let mut env: Vec<(String, i32)> = Vec::new();
    let fname = f.name.as_str();
    for stmt in &f.body {
        match stmt {
            Stmt::Bind { name, instr, .. } => {
                let v = eval_instr(fname, instr, &env)?;
                env.push((name.clone(), v));
            }
            Stmt::Ret { value, span } => return eval_operand(fname, value, span, &env),
        }
    }
    Err(RuntimeError::MissingRet {
        fn_name: f.name.clone(),
        span: f.span.clone(),
    })
}

fn eval_instr(fname: &str, instr: &Instr, env: &[(String, i32)]) -> Result<i32, RuntimeError> {
    let arity = match instr.op.as_str() {
        "add" | "sub" | "mul" | "div" | "rem" => 2,
        "neg" => 1,
        _ => {
            return Err(RuntimeError::UnknownInstruction {
                fn_name: fname.into(),
                span: instr.span.clone(),
                op: instr.op.clone(),
            });
        }
    };
    if instr.args.len() != arity {
        return Err(RuntimeError::Arity {
            fn_name: fname.into(),
            span: instr.span.clone(),
            op: instr.op.clone(),
            expected: arity,
            actual: instr.args.len(),
        });
    }
    let args: Vec<i32> = instr
        .args
        .iter()
        .map(|a| eval_operand(fname, a, &instr.span, env))
        .collect::<Result<_, _>>()?;

    // The instruction with its operands already evaluated, for error reports.
    let expr = || {
        let mut s = instr.op.clone();
        for a in &args {
            s.push(' ');
            s.push_str(&a.to_string());
        }
        s
    };
    let overflow = || RuntimeError::Overflow {
        fn_name: fname.into(),
        span: instr.span.clone(),
        expr: expr(),
    };
    let div_zero = || RuntimeError::DivisionByZero {
        fn_name: fname.into(),
        span: instr.span.clone(),
        expr: expr(),
    };

    let result = match instr.op.as_str() {
        "add" => args[0].checked_add(args[1]).ok_or_else(overflow)?,
        "sub" => args[0].checked_sub(args[1]).ok_or_else(overflow)?,
        "mul" => args[0].checked_mul(args[1]).ok_or_else(overflow)?,
        "neg" => args[0].checked_neg().ok_or_else(overflow)?,
        "div" | "rem" => {
            if args[1] == 0 {
                return Err(div_zero());
            }
            if instr.op == "div" {
                args[0].checked_div(args[1]).ok_or_else(overflow)?
            } else {
                args[0].checked_rem(args[1]).ok_or_else(overflow)?
            }
        }
        _ => unreachable!("arity table and op table agree"),
    };
    Ok(result)
}

fn eval_operand(
    fname: &str,
    operand: &Operand,
    span: &Span,
    env: &[(String, i32)],
) -> Result<i32, RuntimeError> {
    match operand {
        Operand::Var(name) => env
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .ok_or_else(|| RuntimeError::UndefinedVariable {
                fn_name: fname.into(),
                span: span.clone(),
                name: name.clone(),
            }),
        Operand::Int(text) => parse_i32(text).ok_or_else(|| RuntimeError::LiteralOutOfRange {
            fn_name: fname.into(),
            span: span.clone(),
            literal: text.clone(),
        }),
    }
}

/// Integer literal syntax from §2: optional `-`, then decimal (`1_000`) or hex (`0xFF`).
fn parse_i32(text: &str) -> Option<i32> {
    let (neg, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let digits: String = body.chars().filter(|c| *c != '_').collect();
    let magnitude: i64 = if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).ok()?
    } else {
        digits.parse().ok()?
    };
    let value = if neg { -magnitude } else { magnitude };
    i32::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn run(body: &str) -> Result<i32, RuntimeError> {
        let src =
            format!("program p\n  requires []\n{{\n  fn main() -> Int32\n  {{\n{body}\n  }}\n}}\n");
        run_main(&parse(&lex(&src).unwrap()).unwrap())
    }

    #[test]
    fn evaluates_the_fixed_calculator_to_85() {
        assert_eq!(
            run("    %1 = add 10 20\n    %2 = mul %1 3\n    %3 = sub %2 5\n    ret %3"),
            Ok(85)
        );
    }

    #[test]
    fn div_truncates_toward_zero_and_rem_keeps_sign_of_dividend() {
        assert_eq!(run("    %q = div -7 2\n    ret %q"), Ok(-3));
        assert_eq!(run("    %r = rem -7 2\n    ret %r"), Ok(-1));
    }

    #[test]
    fn neg_and_hex_and_underscore_literals() {
        assert_eq!(
            run("    %a = neg 0x10\n    %b = add %a 1_000\n    ret %b"),
            Ok(984)
        );
    }

    #[test]
    fn let_names_and_temps_share_one_namespace() {
        assert_eq!(
            run("    let x = add 1 2\n    %y = mul x x\n    ret %y"),
            Ok(9)
        );
    }

    #[test]
    fn stops_on_add_overflow_with_evaluated_expression() {
        let e = run("    %1 = add 2147483647 1\n    ret %1").unwrap_err();
        assert_eq!(
            e,
            RuntimeError::Overflow {
                fn_name: "main".into(),
                span: Span { line: 6, col: 10 },
                expr: "add 2147483647 1".into(),
            }
        );
    }

    #[test]
    fn stops_on_mul_overflow_and_neg_of_min() {
        assert!(matches!(
            run("    %1 = mul 65536 32768\n    ret %1"),
            Err(RuntimeError::Overflow { .. })
        ));
        assert!(matches!(
            run("    %1 = neg -2147483648\n    ret %1"),
            Err(RuntimeError::Overflow { .. })
        ));
        assert!(matches!(
            run("    %1 = div -2147483648 -1\n    ret %1"),
            Err(RuntimeError::Overflow { .. })
        ));
    }

    #[test]
    fn stops_on_division_by_zero_with_evaluated_operands() {
        let e = run("    %z = sub 1 1\n    %q = div 5 %z\n    ret %q").unwrap_err();
        assert_eq!(
            e,
            RuntimeError::DivisionByZero {
                fn_name: "main".into(),
                span: Span { line: 7, col: 10 },
                expr: "div 5 0".into(),
            }
        );
        assert!(matches!(
            run("    %r = rem 5 0\n    ret %r"),
            Err(RuntimeError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn rejects_literals_outside_int32() {
        assert!(matches!(
            run("    %1 = add 2147483648 0\n    ret %1"),
            Err(RuntimeError::LiteralOutOfRange { literal, .. }) if literal == "2147483648"
        ));
    }

    #[test]
    fn reports_undefined_variable_unknown_instruction_and_arity() {
        assert!(
            matches!(run("    ret %nope"), Err(RuntimeError::UndefinedVariable { name, .. }) if name == "nope")
        );
        assert!(
            matches!(run("    %1 = pow 2 3\n    ret %1"), Err(RuntimeError::UnknownInstruction { op, .. }) if op == "pow")
        );
        assert!(matches!(
            run("    %1 = add 1\n    ret %1"),
            Err(RuntimeError::Arity { op, expected: 2, actual: 1, .. }) if op == "add"
        ));
    }

    #[test]
    fn reports_missing_ret_and_missing_main() {
        assert!(matches!(
            run("    %1 = add 1 2"),
            Err(RuntimeError::MissingRet { .. })
        ));
        let src = "program p\n  requires []\n{\n  fn other() -> Int32\n  {\n    ret 1\n  }\n}\n";
        assert_eq!(
            run_main(&parse(&lex(src).unwrap()).unwrap()),
            Err(RuntimeError::NoMain)
        );
    }
}
