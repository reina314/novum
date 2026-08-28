use std::rc::Rc;

use novum::{
    Lexer,
    Parser,
    runtime::Value,
    error::{
        Error,
        ErrorKind,
    },
};
use novum::vm::{
    Compiler,
    Vm,
};

fn run(
    source: &str,
) -> Result<Value, Error> {
    let tokens =
        Lexer::new(source)
            .lex()?;

    let mut parser =
        Parser::new(tokens);

    let program =
        parser.parse()?;

    let chunk =
        Compiler::new()
            .compile(&program)?;

    let mut vm =
        Vm::new();

    vm.run(
        Rc::new(chunk)
    )
}

fn unwrap_value(
    source: &str,
) -> Value {
    match run(source) {
        Ok(value) =>
            value,

        Err(error) => {
            panic!(
                "unexpected runtime error:\n{error:?}\nsource:\n{source}"
            );
        }
    }
}

fn assert_int(
    source: &str,
    expected: i64,
) {
    match unwrap_value(source) {
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
    match unwrap_value(source) {
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
    match unwrap_value(source) {
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
    match unwrap_value(source) {
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
    match unwrap_value(source) {
        Value::List(list) => {
            assert_eq!(
                list.len(),
                expected.len(),
                "\nsource:\n{source}"
            );

            for (
                index,
                expected_value,
            ) in expected.iter().enumerate()
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

fn assert_error_kind(
    source: &str,
    expected: ErrorKind,
) {
    match run(source) {
        Ok(value) => {
            panic!(
                "expected {expected:?} error, got {value:?}\nsource:\n{source}"
            );
        }

        Err(error) => {
            assert_eq!(
                error.kind,
                expected,
                "expected error kind {expected:?}, got {error:?}\nsource:\n{source}"
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
// List / Dict
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

#[test]
fn list_assignment() {
    assert_int(
        r#"
        let xs = [1, 2, 3]

        xs[1] = 42

        xs[1]
        "#,
        42,
    );
}

#[test]
fn dict_literal() {
    assert_int(
        r#"
        let d = {
            "value": 42
        }

        d["value"]
        "#,
        42,
    );
}

#[test]
fn empty_dict() {
    match run(
        r#"
        {}
        "#
    ) {
        Ok(Value::Dict(dict)) => {
            assert!(
                dict.borrow().is_empty()
            );
        }

        other => {
            panic!(
                "expected Dict, got {other:?}"
            );
        }
    }
}

#[test]
fn dict_multiple_entries() {
    assert_int(
        r#"
        let d = {
            "a": 10,
            "b": 20,
        }

        d["a"] + d["b"]
        "#,
        30,
    );
}

#[test]
fn dict_expression_values() {
    assert_int(
        r#"
        let d = {
            "value": 10 + 20
        }

        d["value"]
        "#,
        30,
    );
}

#[test]
fn nested_dict() {
    assert_int(
        r#"
        let d = {
            "inner": {
                "value": 42
            }
        }

        d["inner"]["value"]
        "#,
        42,
    );
}

#[test]
fn dict_with_list() {
    assert_int(
        r#"
        let d = {
            "values": [10, 20, 30]
        }

        d["values"][1]
        "#,
        20,
    );
}

#[test]
fn list_with_dict() {
    assert_int(
        r#"
        let xs = [
            {
                "value": 42
            }
        ]

        xs[0]["value"]
        "#,
        42,
    );
}

#[test]
fn dict_uses_local_values() {
    assert_int(
        r#"
        let x = 10

        let d = {
            "x": x + 5
        }

        d["x"]
        "#,
        15,
    );
}

#[test]
fn dict_add_new_key() {
    assert_int(
        r#"
        let d = {}

        d["x"] = 42

        d["x"]
        "#,
        42,
    );
}

#[test]
fn dict_missing_key() {
    assert_error_kind(
        r#"
        let d = {
            "x": 1
        }

        d["y"]
        "#,
        ErrorKind::Index,
    );
}

#[test]
fn duplicate_dict_key() {
    assert_error_kind(
        r#"
        {
            "x": 1,
            "x": 2,
        }
        "#,
        ErrorKind::Runtime,
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

#[test]
fn vm_range_for_empty() {
    let source =
        r#"
        for i in 10..10 {
            123
        }
        "#;

    assert_eq!(
        unwrap_value(source),
        Value::Unit
    );
}

#[test]
fn vm_range_for_inclusive() {
    assert_int(
        r#"
        let sum = 0

        for i in 0..=100 {
            sum += i
        }

        sum
        "#,
        5050
    )
}

#[test]
fn vm_range_for_break() {
    assert_int(
        r#"
        let sum = 0

        for i in 0..100 {
            if i == 10 {
                break
            }

            sum += i
        }

        sum
        "#, 
        45
    );
}

#[test]
fn vm_range_for_continue() {
    assert_int(
        r#"
        let sum = 0

        for i in 0..10 {
            if i % 2 == 0 {
                continue
            }

            sum += i
        }

        sum
        "#,
        25,
    );
}

#[test]
fn vm_nested_range_for() {
    assert_int(
        r#"
        let sum = 0

        for i in 0..10 {
            for j in 0..10 {
                sum += i * j
            }
        }

        sum
        "#,
        2025
    );
}

#[test]
fn vm_range_for_dynamic_bounds() {
    assert_int(
        r#"
        let a = 2
        let b = 5
        let sum = 0

        for i in a..b {
            sum += i
        }

        sum
        "#,
        9
    )
}

#[test]
fn vm_range_for_negative() {
    assert_int(
        r#"
        let sum = 0

        for i in -3..2 {
            sum += i
        }

        sum
        "#,
        -5
    )
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

#[test]
fn match_wildcard() {
    assert_int(
        r#"
        match 99 {
            1 => 10,
            _ => 42,
        }
        "#,
        42,
    );
}

#[test]
fn match_tuple_length_mismatch() {
    assert_int(
        r#"
        match (1,) {
            (1, 2) => 10,
            _ => 20,
        }
        "#,
        20,
    );
}

#[test]
fn match_list_length_mismatch() {
    assert_int(
        r#"
        match [1] {
            [1, 2] => 10,
            _ => 20,
        }
        "#,
        20,
    );
}

#[test]
fn match_nested_list() {
    assert_int(
        r#"
        match [[1, 2]] {
            [[1, 3]] => 10,
            [[1, 2]] => 20,
            _ => 30,
        }
        "#,
        20,
    );
}

#[test]
fn match_binding() {
    assert_int(
        r#"
        match 42 {
            x => x + 1,
        }
        "#,
        43,
    );
}

#[test]
fn match_tuple_binding() {
    assert_int(
        r#"
        match (10, 20) {
            (x, y) => x + y,
        }
        "#,
        30,
    );
}

#[test]
fn match_option_some() {
    assert_int(
        r#"
        match Option.Some(42) {
            Option.Some(x) => x,
            Option.None => 0,
        }
        "#,
        42,
    );
}

#[test]
fn match_option_none() {
    assert_int(
        r#"
        match Option.None {
            Option.Some(x) => x,
            Option.None => 42,
        }
        "#,
        42,
    );
}

#[test]
fn match_result_ok() {
    assert_int(
        r#"
        match Result.Ok(42) {
            Result.Ok(x) => x,
            Result.Err(_) => 0,
        }
        "#,
        42,
    );
}

#[test]
fn match_result_then_try() {
    assert_int(
        r#"
        let f = || {
            let value =
                match Result.Ok(41) {
                    Result.Ok(x) => x,
                    Result.Err(_) => 0,
                }

            value + 1
        }

        f()
        "#,
        42,
    );
}

#[test]
fn match_no_arm() {
    assert_error_kind(
        r#"
        match 3 {
            1 => 10,
            2 => 20,
        }
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn match_enum_wrong_arity() {
    assert_int(
        r#"
        match Result.Ok(42) {
            Result.Ok(x, y) => 10,
            _ => 20,
        }
        "#,
        20,
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

#[test]
fn vm_fused_closure_capture() {
    assert_list(
        r#"
        let offset = 10

        (1..4)
            .map(|x| x + offset)
            .collect()
        "#,
        &[
            11,
            12,
            13,
        ],
    );
}

#[test]
fn vm_pipeline_falls_back_for_nested_closure() {
    assert_list(
        r#"
        (1..4)
            .map(|x| {
                let f = |y| y + x
                f(10)
            })
            .collect()
        "#,
        &[
            11,
            12,
            13,
        ],
    );
}

#[test]
fn vm_fused_take_before_filter() {
    assert_list(
        r#"
        (0..10)
            .take(5)
            .filter(|x| x % 2 == 0)
            .collect()
        "#,
        &[
            0,
            2,
            4,
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

// ============================================================
// Enum
// ============================================================

#[test]
fn vm_enum_zero_arity_variant() {
    assert_int(
        r#"
        enum Option {
            Some
            None
        }

        let x =
            Option.None

        match x {
            Option.Some(v) => v
            Option.None => 123
        }
        "#,
        123,
    );
}

#[test]
fn vm_enum_variant_with_value() {
    assert_int(
        r#"
        enum Result {
            Ok(value)
            Err

        }

        let x =
            Result.Ok(42)

        match x {
            Result.Ok(value) => value
            Result.Err => 0
        }
        "#,
        42,
    );
}

#[test]
fn vm_enum_second_arm() {
    assert_int(
        r#"
        enum Result {
            Ok(value)
            Err
        }

        let x =
            Result.Err

        match x {
            Result.Ok(value) => value
            Result.Err => 99
        }
        "#,
        99,
    );
}

#[test]
fn vm_enum_nested_tuple_pattern() {
    assert_int(
        r#"
        enum Result {
            Ok(value)
            Err
        }

        let x =
            Result.Ok(
                (10, 20)
            )

        match x {
            Result.Ok((a, b)) =>
                a + b

            Result.Err =>
                0
        }
        "#,
        30,
    );
}

#[test]
fn vm_enum_nested_list_pattern() {
    assert_int(
        r#"
        enum Result {
            Ok(value)
            Err
        }

        let x =
            Result.Ok(
                [10, 20]
            )

        match x {
            Result.Ok([a, b]) =>
                a + b

            Result.Err =>
                0
        }
        "#,
        30,
    );
}

// ============================================================
// Struct
// ============================================================

#[test]
fn vm_struct_constructor() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        let p =
            Point(10, 20)

        p.x
        "#,
        10,
    );
}

#[test]
fn vm_struct_second_field() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        let p =
            Point(10, 20)

        p.y
        "#,
        20,
    );
}

#[test]
fn vm_struct_value_semantics() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        let a =
            Point(10, 20)

        let b =
            a

        b.x
        "#,
        10,
    );
}

#[test]
fn vm_struct_nested_value() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        struct Line {
            start
            end
        }

        let line =
            Line(
                Point(1, 2),
                Point(3, 4),
            )

        line.end.x
        "#,
        3,
    );
}

#[test]
fn vm_struct_pattern() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        let p =
            Point(10, 20)

        match p {
            Point { x, y } =>
                x + y
        }
        "#,
        30,
    );
}

#[test]
fn vm_struct_pattern_renaming() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        let p =
            Point(10, 20)

        match p {
            Point {
                x: a,
                y: b,
            } =>
                a + b
        }
        "#,
        30,
    );
}

#[test]
fn vm_nested_struct_pattern() {
    assert_int(
        r#"
        struct Point {
            x
            y
        }

        struct Line {
            start
            end
        }

        let line =
            Line(
                Point(1, 2),
                Point(3, 4),
            )

        match line {
            Line {
                start: Point { x: a, y: b },
                end: Point { x: c, y: d },
            } =>
                a + b + c + d
        }
        "#,
        10,
    );
}

// ============================================================
// Class
// ============================================================

#[test]
fn vm_class_default_field() {
    assert_int(
        r#"
        class Counter {
            value = 10
        }

        let c =
            Counter()

        c.value
        "#,
        10,
    );
}

#[test]
fn vm_class_field_assignment() {
    assert_int(
        r#"
        class Counter {
            value = 10
        }

        let c =
            Counter()

        c.value = 50

        c.value
        "#,
        50,
    );
}

#[test]
fn vm_class_reference_semantics() {
    assert_int(
        r#"
        class Counter {
            value = 0
        }

        let a =
            Counter()

        let b =
            a

        a.value = 10

        b.value
        "#,
        10,
    );
}

#[test]
fn vm_class_method() {
    assert_int(
        r#"
        class Counter {
            value = 0

            inc = |self| {
                self.value += 1
                self.value
            }
        }

        let c =
            Counter()

        c.inc()
        "#,
        1,
    );
}

#[test]
fn vm_class_method_multiple_calls() {
    assert_int(
        r#"
        class Counter {
            value = 0

            inc = |self| {
                self.value += 1
                self.value
            }
        }

        let c =
            Counter()

        c.inc()
        c.inc()
        c.inc()
        "#,
        3,
    );
}

#[test]
fn vm_class_default_is_per_instance() {
    assert_int(
        r#"
        class Counter {
            value = 0
        }

        let a =
            Counter()

        let b =
            Counter()

        a.value = 10

        b.value
        "#,
        0,
    );
}

#[test]
fn vm_class_constructor() {
    assert_int(
        r#"
        class Counter {
            value = 0

            init = |self, initial| {
                self.value = initial
            }
        }

        let c =
            Counter(10)

        c.value
        "#,
        10,
    );
}

#[test]
fn vm_class_constructor_and_method() {
    assert_int(
        r#"
        class Counter {
            value = 0

            init = |self, initial| {
                self.value = initial
            }

            inc = |self| {
                self.value += 1
                self.value
            }
        }

        let c =
            Counter(10)

        c.inc()
        "#,
        11,
    );
}

#[test]
fn vm_class_constructor_multiple_args() {
    assert_int(
        r#"
        class Point {
            x = 0
            y = 0

            init = |self, x, y| {
                self.x = x
                self.y = y
            }
        }

        let p =
            Point(10, 20)

        p.x + p.y
        "#,
        30,
    );
}

#[test]
fn vm_class_defaults_before_constructor() {
    assert_int(
        r#"
        class Counter {
            value = 10

            init = |self| {
                self.value += 5
            }
        }

        let c =
            Counter()

        c.value
        "#,
        15,
    );
}

#[test]
fn vm_class_constructor_return_value_is_ignored() {
    assert_int(
        r#"
        class Counter {
            value = 0

            init = |self| {
                self.value = 42
                999
            }
        }

        let c =
            Counter()

        c.value
        "#,
        42,
    );
}

// ============================================================
// Named arguments
// ============================================================

#[test]
fn vm_named_function_arguments() {
    assert_int(
        r#"
        let add =
            |a, b| a + b

        add(
            a = 10,
            b = 20,
        )
        "#,
        30,
    );
}

#[test]
fn vm_named_arguments_out_of_order() {
    assert_int(
        r#"
        let sub =
            |a, b| a - b

        sub(
            b = 3,
            a = 10,
        )
        "#,
        7,
    );
}

#[test]
fn vm_named_constructor_arguments() {
    assert_int(
        r#"
        class Point {
            x = 0
            y = 0

            init = |self, x, y| {
                self.x = x
                self.y = y
            }
        }

        let p =
            Point(
                y = 20,
                x = 10,
            )

        p.x + p.y
        "#,
        30,
    );
}

#[test]
fn vm_mixed_method_arguments() {
    assert_int(
        r#"
        class Calculator {
            calc = |self, a, b, c| {
                a + b * c
            }
        }

        let c =
            Calculator()

        c.calc(
            10,
            c = 2,
            b = 5,
        )
        "#,
        20,
    );
}

// ============================================================
// Import
// ============================================================

#[test]
fn vm_import_unknown_module_is_error() {
    assert_error_kind(
        "import does_not_exist",
        ErrorKind::Import,
    );
}

#[test]
fn vm_cyclic_import_is_error() {
    assert_error_kind(
        "import tests.modules.c",
        ErrorKind::Import,
    );
}

#[test]
fn vm_import_user_module_alias() {
    assert_int(
        r#"
        import tests.modules.counter as mod

        mod.value
        "#,
        1
    );
}

#[test]
fn vm_import_with_and_without_alias() {
    assert_bool(
        r#"
        import tests.modules.counter
        import tests.modules.counter as c
        
        tests.modules.counter.value
            == c.value
        "#, 
        true
    );
}

#[test]
fn vm_import_without_alias_keeps_namespace() {
    assert_int(
        r#"
        import tests.modules.counter

        tests.modules.counter.value
        "#,
        1,
    );
}

#[test]
fn vm_public_let_is_exported() {
    // fixture:
    //
    // pub let answer = 42
    //
    assert_int(
        r#"
        import tests.modules.visibility
        tests.modules.visibility.answer
        "#,
        42
    );
}

#[test]
fn vm_private_let_is_hidden() {
    assert_error_kind(
        r#"
        import tests.modules.visibility
        tests.modules.visibility.secret
        "#,
        ErrorKind::Name
    );
}

#[test]
fn vm_public_lambda_is_exported() {
    assert_int(
        r#"
        import tests.modules.visibility
        tests.modules.visibility.add(2, 3)
        "#,
        5
    );
}

#[test]
fn vm_private_lambda_is_hidden() {
    assert_error_kind(
        r#"
        import tests.modules.visibility
        tests.modules.visibility.helper(10)
        "#,
        ErrorKind::Name,
    );
}

#[test]
fn vm_pub_local_is_error() {
    assert_error_kind(
        r#"
        {
            pub let x = 10
            x
        }
        "#,
        ErrorKind::Name,
    );
}

#[test]
fn imported_class_supports_named_arguments() {
    assert_int(
        r#"
        import tests.modules.test1

        let test =
            tests.modules.test1.Test(
                x = 1,
                y = 3,
            )

        test.x
        "#,
        4,
    );
}

#[test]
fn imported_class_named_arguments_with_alias() {
    assert_int(
        r#"
        import tests.modules.test1 as m

        let test =
            m.Test(
                x = 1,
                y = 3,
            )

        test.x
        "#,
        4,
    );
}

// ============================================================
// Stdlib
// ============================================================

#[test]
fn builtin_len() {
    assert_int(
        "len([1, 2, 3])",
        3,
    );
}

#[test]
fn builtin_typeof() {
    assert_string(
        "typeof(42)",
        "Int",
    );
}

#[test]
fn builtin_str() {
    assert_string(
        "str(42)",
        "42",
    );
}

#[test]
fn stdlib_math_sqrt() {
    assert_float(
        r#"
        import math as m
        m.sqrt(16)
        "#,
        4.0,
    );
}

#[test]
fn stdlib_math_sin() {
    assert_float(
        r#"
        import math
        math.sin(math.pi())
        "#,
        0.0,
    );
}

#[test]
fn stdlib_fs_exists() {
    match run(
        r#"
        import fs
        fs.exists("__novum_file_that_does_not_exist__")
        "#
    ) {
        Ok(Value::Bool(false)) => {}

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn stdlib_process_cwd() {
    match run(
        r#"
        import process as p
        p.cwd()?
        "#
    ) {
        Ok(Value::Path(_)) => {}

        other => {
            panic!(
                "expected Str, got {other:?}"
            );
        }
    }
}

// ============================================================
// Option and Result
// ============================================================

#[test]
fn option_some() {
    match run(
        r#"
        Option.Some(42)
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Option"
            );

            assert_eq!(
                value.variant(),
                "Some"
            );

            assert_eq!(
                value.fields(),
                &[Value::Int(42)]
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn option_none() {
    match run(
        r#"
        Option.None
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Option"
            );

            assert_eq!(
                value.variant(),
                "None"
            );

            assert!(
                value.fields().is_empty()
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn result_ok() {
    match run(
        r#"
        Result.Ok(42)
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Ok"
            );

            assert_eq!(
                value.field(0),
                Some(Value::Int(42))
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn result_err() {
    match run(
        r#"
        Result.Err("failed")
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Err"
            );

            assert_eq!(
                value.field(0),
                Some(
                    Value::Str(
                        Rc::new(
                            "failed"
                                .to_string()
                        )
                    )
                )
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn try_option_some() {
    assert_int(
        r#"
        Option.Some(42)?
        "#,
        42,
    );
}

#[test]
fn try_option_none() {
    match run(
        r#"
        Option.None?
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Option"
            );

            assert_eq!(
                value.variant(),
                "None"
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn try_result_ok() {
    assert_int(
        r#"
        Result.Ok(42)?
        "#,
        42,
    );
}

#[test]
fn try_result_err() {
    match run(
        r#"
        Result.Err("failed")?
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Err"
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn try_propagates_from_function() {
    match run(
        r#"
        let f = || {
            Option.None?
            42
        }

        f()
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Option"
            );

            assert_eq!(
                value.variant(),
                "None"
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn try_unwraps_inside_function() {
    assert_int(
        r#"
        let f = || {
            Option.Some(41)?
                + 1
        }

        f()
        "#,
        42,
    );
}

#[test]
fn try_result_propagates_from_function() {
    match run(
        r#"
        let f = || {
            Result.Err("failed")?
            42
        }

        f()
        "#
    ) {
        Ok(
            Value::EnumValue(value)
        ) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Err"
            );
        }

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn fs_read_try_propagates_from_function() {
    match run(
        r#"
        import fs

        let read_file =
            |path| {
                fs.read(path)?
            }

        read_file(
            "__novum_missing_file__"
        )
        "#
    ) {
        Ok(Value::EnumValue(value)) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Err"
            );
        }

        other => {
            panic!(
                "expected Result::Err, got {other:?}"
            );
        }
    }
}

// ============================================================
// Parameter pattern
// ============================================================

#[test]
fn destructuring_list_parameter() {
    assert_int(
        r#"
        let f =
            |[x, y]| {
                x + y
            }

        f([10, 20])
        "#,
        30,
    );
}

#[test]
fn destructuring_tuple_parameter() {
    assert_int(
        r#"
        let f =
            |(x, y)| {
                x * y
            }

        f((6, 7))
        "#,
        42,
    );
}

#[test]
fn nested_destructuring_parameter() {
    assert_int(
        r#"
        let f =
            |(x, [y, z])| {
                x + y + z
            }

        f(
            (10, [20, 30])
        )
        "#,
        60,
    );
}

#[test]
fn literal_parameter_pattern() {
    assert_int(
        r#"
        let f =
            |42| {
                100
            }

        f(42)
        "#,
        100,
    );
}

#[test]
fn literal_parameter_pattern_fails() {
    assert_error_kind(
        r#"
        let f =
            |42| {
                100
            }

        f(41)
        "#,
        ErrorKind::Runtime,
    );
}

#[test]
fn enum_parameter_pattern() {
    assert_int(
        r#"
        let f =
            |Option.Some(x)| {
                x + 1
            }

        f(
            Option.Some(41)
        )
        "#,
        42,
    );
}

#[test]
fn result_parameter_pattern() {
    assert_int(
        r#"
        let f =
            |Result.Ok(x)| {
                x
            }

        f(
            Result.Ok(42)
        )
        "#,
        42,
    );
}

#[test]
fn named_function_argument() {
    assert_int(
        r#"
        let f =
            |x, y| {
                x + y
            }

        f(
            y = 20,
            x = 22
        )
        "#,
        42,
    );
}

// ============================================================
// Method call
// ============================================================

#[test]
fn nested_method_call_argument() {
    assert_int(
        r#"
        class Test {
            value = 42

            get = |self| {
                self.value
            }
        }

        let t =
            Test()

        let f =
            |x| x

        f(
            t.get()
        )
        "#,
        42,
    );
}

#[test]
fn nested_method_call_in_builtin() {
    match run(
        r#"
        class Test {
            value = 42

            get = |self| {
                self.value
            }
        }

        let t =
            Test()

        print(
            t.get()
        )
        "#
    ) {
        Ok(Value::Unit) => {}

        other => {
            panic!(
                "unexpected result: {other:?}"
            );
        }
    }
}

#[test]
fn tuple_pattern_result_can_be_nested_in_call() {
    assert_int(
        r#"
        let f =
            |(x, y)| {
                x + y
            }

        let identity =
            |value| {
                value
            }

        identity(
            f((10, 20))
        )
        "#,
        30,
    );
}

#[test]
fn method_result_can_be_nested_in_call() {
    assert_int(
        r#"
        class Test {
            weights = []
            
            predict =
                |self, x| {
                    zip(
                        self.weights,
                        x
                    )
                    .map(
                        |(w, xi)| {
                            w * xi
                        }
                    )
                    .sum()
                }
        }

        let model =
            Test()

        model.weights =
            [1, 1]

        let x =
            [10, 20]

        let identity =
            |value| {
                value
            }

        identity(
            model.predict(x)
        )
        "#,
        30,
    );
}
