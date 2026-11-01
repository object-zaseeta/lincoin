//! End-to-end tests: run the `lang` binary on example programs.

use std::path::PathBuf;
use std::process::{Command, Output};

fn lang(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lang"))
        .args(args)
        .output()
        .expect("failed to spawn lang")
}

fn example(name: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("examples");
    p.push(name);
    p.to_string_lossy().into_owned()
}

#[test]
fn run_prints_the_result_of_the_fixed_calculator() {
    let out = lang(&["run", &example("calc.lang")]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "85\n");
}

fn stderr_json(out: &Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&out.stderr);
    serde_json::from_str(text.trim()).unwrap_or_else(|e| panic!("stderr is not JSON ({e}): {text}"))
}

#[test]
fn run_stops_on_overflow_with_a_structured_error() {
    let out = lang(&["run", &example("overflow.lang")]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    let err = stderr_json(&out);
    assert_eq!(err["kind"], "overflow");
    assert_eq!(err["location"]["fn"], "main");
    assert_eq!(err["location"]["line"], 7);
    assert_eq!(err["location"]["col"], 10);
    assert!(
        err["location"]["file"]
            .as_str()
            .unwrap()
            .ends_with("overflow.lang")
    );
    assert_eq!(err["expected"], "Int32");
    assert_eq!(err["actual"], "add 2147483647 1");
}

#[test]
fn run_stops_on_division_by_zero_with_a_structured_error() {
    let out = lang(&["run", &example("div_zero.lang")]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr_json(&out);
    assert_eq!(err["kind"], "division_by_zero");
    assert_eq!(err["location"]["line"], 8);
    assert_eq!(err["actual"], "div 5 0");
    assert!(err["suggestion"].is_string());
}

#[test]
fn run_reports_syntax_errors_as_structured_errors() {
    let out = lang(&["run", &example("syntax_error.lang")]);
    assert_eq!(out.status.code(), Some(2));
    let err = stderr_json(&out);
    assert_eq!(err["kind"], "syntax_error");
    assert_eq!(err["location"]["line"], 4);
    assert_eq!(err["location"]["col"], 13);
    assert_eq!(err["expected"], "'->'");
    assert_eq!(err["actual"], "Int32");
}

#[test]
fn run_reports_a_missing_file_as_a_structured_error() {
    let out = lang(&["run", &example("nope.lang")]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(stderr_json(&out)["kind"], "io_error");
}

#[test]
fn usage_error_when_no_command_is_given() {
    let out = lang(&[]);
    assert_eq!(out.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&out.stderr).contains("lang run"));
}
