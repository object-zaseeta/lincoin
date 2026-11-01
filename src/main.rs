//! `lang` — command-line entry point (§12.1). Step 0 implements only `lang run`.

use std::process::ExitCode;

use lincoin::error::Diagnostic;
use lincoin::{interp, lexer, parser};

/// Exit codes: 0 ran to completion; 1 the program stopped (contract/overflow/division);
/// 2 the source could not be compiled or read; 64 the command line was wrong (EX_USAGE).
const EXIT_STOPPED: u8 = 1;
const EXIT_COMPILE: u8 = 2;
const EXIT_USAGE: u8 = 64;

const USAGE: &str = "usage:\n  lang run <file>    run the program with the interpreter\n";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, file] if cmd == "run" => run(file),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn run(file: &str) -> ExitCode {
    let fail = |d: Diagnostic, code: u8| {
        eprintln!("{}", d.to_json());
        ExitCode::from(code)
    };
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => return fail(Diagnostic::io(file, e.to_string()), EXIT_COMPILE),
    };
    let tokens = match lexer::lex(&src) {
        Ok(t) => t,
        Err(e) => return fail(Diagnostic::from_lex(file, &e), EXIT_COMPILE),
    };
    let program = match parser::parse(&tokens) {
        Ok(p) => p,
        Err(e) => return fail(Diagnostic::from_parse(file, &e), EXIT_COMPILE),
    };
    match interp::run_main(&program) {
        Ok(v) => {
            println!("{v}");
            ExitCode::SUCCESS
        }
        Err(e) => fail(Diagnostic::from_runtime(file, &e), EXIT_STOPPED),
    }
}
