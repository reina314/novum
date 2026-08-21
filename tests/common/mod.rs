use novum::{
    Interpreter,
    Lexer,
    Parser,
    Error,
    ErrorKind,
    runtime::{
        ControlFlow,
        Value
    }
};

pub fn run(source: &str) -> Value {
    run_result(source)
        .expect("program failed")
}

pub fn run_result(
    source: &str,
) -> Result<Value, Error> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.lex()?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    let mut interpreter = Interpreter::new();

    match interpreter.eval_program(&program)? {
        ControlFlow::Value(value) =>
            Ok(value),

        ControlFlow::Return(value) => {
            Err(
                Error::new(
                    ErrorKind::Runtime,
                    format!(
                        "unexpected return at top level: {}",
                        value
                    ),
                    None,
                )
            )
        }

        ControlFlow::Break => {
            Err(
                Error::new(
                    ErrorKind::Runtime,
                    "unexpected break at top level",
                    None,
                )
            )
        }
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