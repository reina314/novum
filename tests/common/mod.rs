use novum::{Interpreter, Lexer, Parser};
use novum::runtime::{ControlFlow, Value};

pub fn run(src: &str) -> Value {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.lex().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new();
    match interpreter.eval_program(&program).unwrap() {
        ControlFlow::Value(v) => v,
        _ => panic!("unexpected control flow"),
    }
}

pub fn assert_float_close(
    actual: f64,
    expected: f64,
) {
    let abs_tol = 1e-10;
    let rel_tol = 1e-10;

    let diff = (actual - expected).abs();

    let tolerance =
        abs_tol + rel_tol * expected.abs();

    assert!(
        diff <= tolerance,
        "expected {}, got {}, diff {} > tolerance {}",
        expected,
        actual,
        diff,
        tolerance,
    );
}