use std::rc::Rc;

use novum::{
    Lexer,
    Parser,
    runtime::Value,
};
use novum::vm::{
    Compiler,
    Vm,
};

fn run(source: &str) -> Value {
    let tokens =
        Lexer::new(source)
            .lex()
            .unwrap_or_else(|error| {
                panic!(
                    "lexer error:\n{error:?}\nsource:\n{source}"
                );
            });

    let mut parser =
        Parser::new(tokens);

    let program =
        parser.parse()
            .unwrap_or_else(|error| {
                panic!(
                    "parser error:\n{error:?}\nsource:\n{source}"
                );
            });

    let chunk =
        Compiler::new()
            .compile(&program)
            .unwrap_or_else(|error| {
                panic!(
                    "compiler error:\n{error:?}\nsource:\n{source}"
                );
            });

    let mut vm =
        Vm::new();

    vm.run(
        Rc::new(chunk)
    )
    .unwrap_or_else(|error| {
        panic!(
            "VM error:\n{error:?}\nsource:\n{source}"
        );
    })
}

fn assert_int(
    source: &str,
    expected: i64,
) {
    match run(source) {
        Value::Int(actual) => {
            assert_eq!(
                actual,
                expected,
                "\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Int({expected}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

fn assert_float(
    source: &str,
    expected: f64,
) {
    match run(source) {
        Value::Float(actual) => {
            assert!(
                (actual - expected).abs()
                    < 1e-10,
                "expected Float({expected}), got {actual:?}\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Float({expected}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

fn assert_bool(
    source: &str,
    expected: bool,
) {
    match run(source) {
        Value::Bool(actual) => {
            assert_eq!(
                actual,
                expected,
                "\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Bool({expected}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

fn assert_string(
    source: &str,
    expected: &str,
) {
    match run(source) {
        Value::Str(actual) => {
            assert_eq!(
                actual.as_str(),
                expected,
                "\nsource:\n{source}"
            );
        }

        actual => {
            panic!(
                "expected Str({expected:?}), got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

fn assert_list(
    source: &str,
    expected: &[i64],
) {
    match run(source) {
        Value::List(list) => {
            assert_eq!(
                list.len(),
                expected.len(),
                "\nsource:\n{source}"
            );

            for (index, expected_value) in
                expected.iter().enumerate()
            {
                let actual =
                    list.get(index)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing list element at index {index}\nsource:\n{source}"
                            )
                        });

                match actual {
                    Value::Int(actual_value) => {
                        assert_eq!(
                            actual_value,
                            *expected_value,
                            "element {index}\nsource:\n{source}"
                        );
                    }

                    other => {
                        panic!(
                            "expected Int at index {index}, got {other:?}\nsource:\n{source}"
                        );
                    }
                }
            }
        }

        actual => {
            panic!(
                "expected List, got {actual:?}\nsource:\n{source}"
            );
        }
    }
}

// ============================================================
// Scalars
// ============================================================

#[test]
fn vm_scalars() {
    assert_int(
        "1 + 2 * 3",
        7,
    );

    assert_int(
        "(2 + 3) * 4",
        20,
    );

    assert_float(
        "1.5 + 2.5",
        4.0,
    );

    assert_bool(
        "3 < 4",
        true,
    );

    assert_bool(
        "3 == 3",
        true,
    );
}

// ============================================================
// Variables / assignment
// ============================================================

#[test]
fn vm_variables() {
    assert_int(
        "
        let a = 3
        a
        ",
        3,
    );

    assert_int(
        "
        let a = 3
        a = 4
        a
        ",
        4,
    );

    assert_int(
        "
        a = 3
        a += 4
        a
        ",
        7,
    );
}

// ============================================================
// Tuple
// ============================================================

#[test]
fn vm_tuple() {
    assert_int(
        "
        let x = (10, 20)
        x.0
        ",
        10,
    );

    assert_int(
        "
        let (a, b) = (10, 20)
        a + b
        ",
        30,
    );

    assert_int(
        "
        let (a, (b, c)) = (1, (2, 3))
        a + b + c
        ",
        6,
    );
}

// ============================================================
// List / indexing
// ============================================================

#[test]
fn vm_list() {
    assert_int(
        "
        let a = [1, 2, 3]
        a[0]
        ",
        1,
    );

    assert_int(
        "
        let a = [1, 2, 3]
        a[1] = 100
        a[1]
        ",
        100,
    );

    assert_int(
        "
        let a = [1, 2, 3]
        a[0] += 10
        a[0]
        ",
        11,
    );

    assert_list(
        "
        [1, 2, 3]
        ",
        &[1, 2, 3],
    );
}

// ============================================================
// Range
// ============================================================

#[test]
fn vm_range() {
    assert_list(
        "
        (1..5).collect()
        ",
        &[1, 2, 3, 4],
    );

    assert_list(
        "
        (1..=5).collect()
        ",
        &[1, 2, 3, 4, 5],
    );

    assert_list(
        "
        (1..1_000_001)
            .take(3)
            .collect()
        ",
        &[1, 2, 3],
    );
}

// ============================================================
// If / while / for
// ============================================================

#[test]
fn vm_control_flow() {
    assert_int(
        "
        if true {
            10
        } else {
            20
        }
        ",
        10,
    );

    assert_int(
        "
        let i = 0
        while i < 5 {
            i += 1
            i
        }
        ",
        5,
    );

    assert_int(
        "
        let sum = 0

        for x in [1, 2, 3, 4] {
            sum += x
        }

        sum
        ",
        10,
    );
}

// ============================================================
// Closure / capture
// ============================================================

#[test]
fn vm_closure() {
    assert_int(
        "
        let f = |x| x * 2
        f(5)
        ",
        10,
    );

    assert_int(
        "
        let i = 0

        let f = || {
            i += 1
            i
        }

        f()
        f()
        ",
        2,
    );
}

// ============================================================
// Assignment expression
// ============================================================

#[test]
fn vm_assignment_expression() {
    assert_int(
        "
        let a = 0
        let x = (a = 5)
        x
        ",
        5,
    );

    assert_int(
        "
        let a = [1, 2, 3]
        let x = (a[1] = 99)
        x
        ",
        99,
    );
}

// ============================================================
// Iterator
// ============================================================

#[test]
fn vm_iterator_pipeline() {
    assert_list(
        "
        (1..5)
            .map(|x| x * 2)
            .collect()
        ",
        &[2, 4, 6, 8],
    );

    assert_list(
        "
        (1..10)
            .filter(|x| x % 2 == 0)
            .collect()
        ",
        &[2, 4, 6, 8],
    );

    assert_list(
        "
        (1..10)
            .filter(|x| x % 2 == 0)
            .skip(1)
            .take(2)
            .collect()
        ",
        &[4, 6],
    );

    assert_list(
        "
        (1..10)
            .map(|x| x * 2)
            .filter(|x| x % 3 == 0)
            .take(4)
            .collect()
        ",
        &[6, 12, 18],
    );
}

// ============================================================
// String methods
// ============================================================

#[test]
fn vm_strings() {
    assert_int(
        "\"hello\".len()",
        5,
    );

    assert_string(
        "\"hello\".to_upper()",
        "HELLO",
    );

    assert_string(
        "\" hello \".trim()",
        "hello",
    );

    assert_bool(
        "\"hello\".contains(\"ell\")",
        true,
    );

    assert_bool(
        "\"hello\".starts_with(\"he\")",
        true,
    );

    assert_bool(
        "\"hello\".ends_with(\"lo\")",
        true,
    );

    assert_string(
        "\"ab\".repeat(3)",
        "ababab",
    );
}

// ============================================================
// Match
// ============================================================

#[test]
fn vm_match() {
    assert_int(
        "
        let x = 2

        match x {
            1 => 10
            2 => 20
            _ => 30
        }
        ",
        20,
    );

    assert_int(
        "
        let y = (1, 2)

        match y {
            (1, b) => b
            (a, b) => a + b
        }
        ",
        2,
    );

    assert_int(
        "
        let y = (3, 4)

        match y {
            (1, b) => b
            (a, b) => a + b
        }
        ",
        7,
    );

    assert_int(
        "
        let y = [1, 2]

        match y {
            [1, b] => b
            [a, b] => a + b
        }
        ",
        2,
    );
}

// ============================================================
// Combined stress test
// ============================================================

#[test]
fn vm_combined_pipeline() {
    assert_list(
        "
        let offset = 10

        (1..1_000_000)
            .map(|x| x + offset)
            .filter(|x| x % 3 == 0)
            .skip(5)
            .take(5)
            .collect()
        ",
        &[
            27,
            30,
            33,
            36,
            39,
        ],
    );
}

// ============================================================
// Pattern / destructuring
// ============================================================

#[test]
fn vm_let_tuple_pattern() {
    assert_int(
        "
        let (a, b) = (1, 2)
        a + b
        ",
        3,
    );
}

#[test]
fn vm_let_nested_tuple_pattern() {
    assert_int(
        "
        let (a, (b, c)) = (1, (2, 3))
        a + b + c
        ",
        6,
    );
}

#[test]
fn vm_let_list_pattern() {
    assert_int(
        "
        let [a, b, c] = [10, 20, 30]
        a + b + c
        ",
        60,
    );
}

#[test]
fn vm_let_nested_pattern() {
    assert_int(
        "
        let (a, [b, c]) = (1, [2, 3])
        a + b + c
        ",
        6,
    );
}

#[test]
fn vm_let_wildcard_pattern() {
    assert_int(
        "
        let (_, b) = (100, 5)
        b
        ",
        5,
    );
}

// ============================================================
// Match
// ============================================================

#[test]
fn vm_match_integer_literal() {
    assert_int(
        "
        let x = 2

        match x {
            1 => 10
            2 => 20
            _ => 30
        }
        ",
        20,
    );
}

#[test]
fn vm_match_wildcard() {
    assert_int(
        "
        match 999 {
            1 => 10
            _ => 42
        }
        ",
        42,
    );
}

#[test]
fn vm_match_tuple_first_arm() {
    assert_int(
        "
        let y = (1, 2)

        match y {
            (1, b) => b
            (a, b) => a + b
        }
        ",
        2,
    );
}

#[test]
fn vm_match_tuple_second_arm() {
    assert_int(
        "
        let y = (3, 4)

        match y {
            (1, b) => b
            (a, b) => a + b
        }
        ",
        7,
    );
}

#[test]
fn vm_match_tuple_nested_pattern() {
    assert_int(
        "
        let y = (1, (2, 3))

        match y {
            (1, (b, c)) => b + c
            _ => 0
        }
        ",
        5,
    );
}

#[test]
fn vm_match_list_pattern() {
    assert_int(
        "
        let y = [1, 2]

        match y {
            [1, b] => b
            [a, b] => a + b
        }
        ",
        2,
    );
}

#[test]
fn vm_match_list_second_arm() {
    assert_int(
        "
        let y = [3, 4]

        match y {
            [1, b] => b
            [a, b] => a + b
        }
        ",
        7,
    );
}

#[test]
fn vm_match_with_expression_body() {
    assert_int(
        "
        let x = 3

        match x {
            1 => 10 + 1
            2 => 20 + 2
            3 => 30 + 3
        }
        ",
        33,
    );
}

#[test]
fn vm_match_without_commas() {
    assert_int(
        "
        let y = (3, 4)

        match y {
            (1, b) => b
            (a, b) => a + b
        }
        ",
        7,
    );
}

#[test]
fn vm_match_with_commas() {
    assert_int(
        "
        let y = (3, 4)

        match y {
            (1, b) => b,
            (a, b) => a + b,
        }
        ",
        7,
    );
}

// ============================================================
// Match arm scope
// ============================================================

#[test]
fn vm_match_arm_bindings_are_local() {
    assert_int(
        "
        let x = 2

        let result =
            match x {
                1 => {
                    let a = 10
                    a
                }

                2 => {
                    let b = 20
                    b
                }

                _ => 0
            }

        result
        ",
        20,
    );
}

// ============================================================
// Pattern + closure
// ============================================================

#[test]
fn vm_pattern_binding_used_by_closure() {
    assert_int(
        "
        let (a, b) = (10, 20)

        let f = || a + b

        f()
        ",
        30,
    );
}

// ============================================================
// Pattern + mutation
// ============================================================

#[test]
fn vm_pattern_binding_can_be_reassigned() {
    assert_int(
        "
        let (a, b) = (1, 2)

        a = 10

        a + b
        ",
        12,
    );
}