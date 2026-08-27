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

