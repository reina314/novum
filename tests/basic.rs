mod common;

use common::{
    run, run_result
};
use novum::{Interpreter, Lexer, Parser};
use novum::runtime::{BoundMethod, Class, FuncRef, Function, MethodReceiver, Object, Series, Value};
use std::{cell::RefCell, rc::Rc};


#[test]
fn compound_assignment_add() {
    let result = run(
        r#"
        x = 10
        x += 5
        x
        "#
    );

    assert_eq!(
        result,
        Value::Int(15)
    );
}

#[test]
fn compound_assignment_sub() {
    let result = run(
        r#"
        x = 10
        x -= 3
        x
        "#
    );

    assert_eq!(
        result,
        Value::Int(7)
    );
}

#[test]
fn compound_assignment_mul() {
    let result = run(
        r#"
        x = 10
        x *= 3
        x
        "#
    );

    assert_eq!(
        result,
        Value::Int(30)
    );
}

#[test]
fn compound_assignment_div() {
    let result = run(
        r#"
        x = 10
        x /= 4
        x
        "#
    );

    // 現在の division semantics に合わせる
}

#[test]
fn compound_assign_list_index() {
    let result =
        run(
            r#"
            xs = [1, 2, 3]
            xs[0] += 10
            xs[0]
            "#
        );

    assert_eq!(
        result,
        Value::Int(11)
    );
}

#[test]
fn compound_assign_dict_index() {
    let result =
        run(
            r#"
            d = {"count": 1}
            d["count"] += 4
            d["count"]
            "#
        );

    assert_eq!(
        result,
        Value::Int(5)
    );
}

#[test]
fn compound_assign_matrix_index() {
    let result =
        run(
            r#"
            A = matrix([[1, 2], [3, 4]])
            A[0, 1] += 10
            A[0, 1]
            "#
        );

    assert_eq!(
        result,
        Value::Float(12.0)
    );
}

#[test]
fn compound_assign_object_field() {
    let result =
        run(
            r#"
            struct Point {
                x,
            }

            let p = Point(10)
            p.x += 5
            p.x
            "#
        );

    assert_eq!(
        result,
        Value::Int(15)
    );
}

#[test]
fn mathematical_precedence() {
    assert_eq!(run("2 + 3 * 4"), Value::Int(14));
    assert_eq!(run("2 * 3 ** 2"), Value::Int(18));
    assert_eq!(run("-2 ** 2"),Value::Int(-4));
    assert_eq!(run("(-2) ** 2"),Value::Int(4));
    assert_eq!(run("2 ** 3 ** 2"),Value::Int(512));
    assert_eq!(run("2 ** -2"),Value::Float(0.25));
}

#[test]
fn drop_statement_still_works() {
    let result =
        run(
            r#"
            let xs = [1, 2, 3]
            drop xs
            "#
        );

    assert_eq!(
        result,
        Value::Unit
    );
}

#[test]
fn drop_can_be_used_as_identifier() {
    let result =
        run(
            r#"
            let drop = 42
            drop
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn short_circuit() {
    assert_eq!(run("false and missing"), Value::Bool(false));
    assert_eq!(run("true or missing"), Value::Bool(true));
}

#[test]
fn function_arity_is_checked() {
    let mut lexer = Lexer::new("f = |x| x; f()" );
    let tokens = lexer.lex().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new();
    assert!(interpreter.eval_program(&program).is_err());
}

#[test]
fn list_push_method() {
    let result = run(
        r#"
        let xs = [1, 2]
        xs.push(3)
        xs
        "#
    );

    match result {
        Value::List(list) => {
            let list =
                list.borrow();

            assert_eq!(
                list.as_slice(),
                &[
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                ]
            );
        }

        other => {
            panic!(
                "expected List, got {:?}",
                other
            );
        }
    }
}

#[test]
fn list_pop_method() {
    let result = run(
        r#"
        let xs = [1, 2, 3]
        xs.pop()
        "#
    );

    assert_eq!(
        result,
        Value::Int(3)
    );
}

#[test]
fn list_remove_method() {
    let result = run(
        r#"
        let xs = [10, 20, 30]
        xs.remove(1)
        "#
    );

    assert_eq!(
        result,
        Value::Int(20)
    );
}

#[test]
fn list_len_method() {
    let result = run(
        r#"
        let xs = [10, 20, 30]
        xs.len()
        "#
    );

    assert_eq!(
        result,
        Value::Int(3)
    );
}

#[test]
fn list_methods_are_safe() {
    assert_eq!(run("x = [1, 2]; x.push(3); x.len()"), Value::Int(3));
}

#[test]
fn let_declaration() {
    assert_eq!(
        run("let x = 10; x"),
        Value::Int(10)
    );
}

#[test]
fn let_then_assign() {
    assert_eq!(
        run("let x = 10; x = 20; x"),
        Value::Int(20)
    );
}

#[test]
fn duplicate_let_is_error() {
    let source = "let x = 10; let x = 20";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.lex().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();
    let result = interpreter.eval_program(&program);

    assert!(result.is_err());
}

#[test]
fn nested_scope_shadowing_is_allowed() {
    let result =
        run(
            r#"
            let x = 10

            {
                let x = 20
                x
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(20)
    );
}

#[test]
fn outer_binding_survives_inner_shadowing() {
    let result =
        run(
            r#"
            let x = 10

            {
                let x = 20
                x
            }

            x
            "#
        );

    assert_eq!(
        result,
        Value::Int(10)
    );
}

#[test]
fn let_allows_shadowing_in_child_scope() {
    assert_eq!(
        run(
            r#"
            let x = 10;

            if true {
                let x = 20;
                x
            }
            "#
        ),
        Value::Int(20)
    );
}

#[test]
fn assignment_updates_outer_binding() {
    assert_eq!(
        run(
            r#"
            let x = 10;

            if true {
                x = 20;
            }

            x
            "#
        ),
        Value::Int(20)
    );
}

#[test]
fn while_continue() {
    let result =
        run(
            r#"
            let x = 0

            while x < 5 {
                x = x + 1

                if x < 5 {
                    continue
                }

                x
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(5)
    );
}

#[test]
fn while_continue_skips_remaining_body() {
    let result =
        run(
            r#"
            let x = 0
            let y = 0

            while x < 5 {
                x = x + 1

                if x < 5 {
                    continue
                }

                y = 100
            }

            y
            "#
        );

    assert_eq!(
        result,
        Value::Int(100)
    );
}

#[test]
fn continue_outside_loop_is_error() {
    let result =
        run_result(
            r#"
            continue
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn function_cannot_continue_caller_loop() {
    let result =
        run_result(
            r#"
            foo = || {
                continue
            }

            while true {
                foo()
            }
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn function_local_continue() {
    let result =
        run(
            r#"
            foo = || {
                let x = 0

                while x < 3 {
                    x = x + 1

                    if x < 3 {
                        continue
                    }
                }

                x
            }

            foo()
            "#
        );

    assert_eq!(
        result,
        Value::Int(3)
    );
}

#[test]
fn dict_literal() {
    assert_eq!(
        run(
            r#"
            let user = {
                "name": "Alice",
                "age": 20
            };

            user["age"]
            "#
        ),
        Value::Int(20)
    );
}

#[test]
fn dict_string_value() {
    assert_eq!(
        run(
            r#"
            let user = {
                "name": "Alice"
            };

            user["name"]
            "#
        ),
        Value::Str(
            std::rc::Rc::new("Alice".to_string())
        )
    );
}

#[test]
fn dict_assignment() {
    assert_eq!(
        run(
            r#"
            let user = {
                "name": "Alice"
            };

            user["age"] = 20;

            user["age"]
            "#
        ),
        Value::Int(20)
    );
}

#[test]
fn dict_overwrite() {
    assert_eq!(
        run(
            r#"
            let user = {
                "age": 20
            };

            user["age"] = 21;

            user["age"]
            "#
        ),
        Value::Int(21)
    );
}

#[test]
fn dict_missing_key_is_error() {
    let mut lexer = Lexer::new(
        r#"
        let user = {
            "name": "Alice"
        };

        user["age"]
        "#
    );

    let tokens = lexer.lex().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();

    assert!(
        interpreter.eval_program(&program).is_err()
    );
}

#[test]
fn dict_index_requires_string() {
    let mut lexer = Lexer::new(
        r#"
        let user = {
            "name": "Alice"
        };

        user[0]
        "#
    );

    let tokens = lexer.lex().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();

    assert!(
        interpreter.eval_program(&program).is_err()
    );
}

#[test]
fn duplicate_dict_key_is_error() {
    let mut lexer = Lexer::new(
        r#"
        {
            "x": 1,
            "x": 2
        }
        "#
    );

    let tokens = lexer.lex().unwrap();

    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let mut interpreter = Interpreter::new();

    assert!(
        interpreter.eval_program(&program).is_err()
    );
}

#[test]
fn object_field_get_set() {
    let object = Rc::new(
        RefCell::new(Object::new())
    );

    object.borrow_mut().set_field(
        "x",
        Value::Int(10),
    );

    assert_eq!(
        object.borrow().get_field("x"),
        Some(Value::Int(10))
    );
}

#[test]
fn struct_constructor() {
    assert_eq!(
        run(
            r#"
            struct Point {
                x,
                y,
            }

            let p = Point(10, 20);

            p.x
            "#
        ),
        Value::Int(10)
    );
}

#[test]
fn struct_multiple_fields() {
    assert_eq!(
        run(
            r#"
            struct Point {
                x,
                y,
            }

            let p = Point(10, 20);

            p.y
            "#
        ),
        Value::Int(20)
    );
}

#[test]
fn struct_field_assignment() {
    assert_eq!(
        run(
            r#"
            struct Point {
                x,
            }

            let p = Point(10);

            p.x = 20;

            p.x
            "#
        ),
        Value::Int(20)
    );
}

#[test]
fn struct_method() {
    assert_eq!(
        run(
            r#"
            struct Point {
                x,
                y,

                move: |self, dx, dy| {
                    self.x = self.x + dx;
                    self.y = self.y + dy;
                }
            }

            let p = Point(10, 20);

            p.move(5, 3);

            p.x
            "#
        ),
        Value::Int(15)
    );
}

#[test]
fn struct_method_updates_multiple_fields() {
    assert_eq!(
        run(
            r#"
            struct Point {
                x,
                y,

                move: |self, dx, dy| {
                    self.x = self.x + dx;
                    self.y = self.y + dy;
                }
            }

            let p = Point(10, 20);

            p.move(5, 3);

            p.y
            "#
        ),
        Value::Int(23)
    );
}

#[test]
fn bound_method_keeps_receiver() {
    let series =
        Rc::new(
            Series::new(
                "score",
                vec![
                    Value::Int(1),
                    Value::Int(2),
                ],
            )
        );

    let method =
        BoundMethod::new(
            MethodReceiver::Series(
                series.clone()
            ),
            "to_list",
        );

    assert_eq!(
        method.name(),
        "to_list"
    );

    match method.receiver() {
        MethodReceiver::Series(value) => {
            assert!(
                Rc::ptr_eq(
                    value,
                    &series
                )
            );
        }

        _ => panic!(
            "unexpected receiver"
        ),
    }
}

#[test]
fn enum_definition() {
    let result =
        run(
            r#"
            enum Color {
                Red,
                Green,
                Blue,
            }

            Color
            "#
        );

    match result {
        Value::Enum(definition) => {
            assert_eq!(
                definition.name(),
                "Color"
            );

            assert!(
                definition
                    .variant("Red")
                    .is_some()
            );

            assert!(
                definition
                    .variant("Green")
                    .is_some()
            );
        }

        other => {
            panic!(
                "expected Enum, got {:?}",
                other
            );
        }
    }
}

#[test]
fn enum_variant_constructor() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value),
                Err(error),
            }

            Result.Ok(42)
            "#
        );

    match result {
        Value::EnumValue(value) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Ok"
            );

            assert_eq!(
                value.fields(),
                &[
                    Value::Int(42)
                ]
            );
        }

        other => {
            panic!(
                "expected EnumValue, got {:?}",
                other
            );
        }
    }
}

#[test]
fn enum_variant_arity_error() {
    let result =
        run_result(
            r#"
            enum Result {
                Ok(value),
            }

            Result.Ok()
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn enum_value_equality() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value),
                Err(error),
            }

            Result.Ok(42)
            ==
            Result.Ok(42)
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

#[test]
fn enum_value_display() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value),
            }

            Result.Ok(42)
            "#
        );

    assert_eq!(
        format!("{}", result),
        "Result.Ok(42)"
    );
}

#[test]
fn match_literal() {
    let result =
        run(
            r#"
            let x = 2

            match x {
                0 => "zero"
                1 => "one"
                2 => "two"
                _ => "other"
            }
            "#
        );

    assert_eq!(
        result,
        Value::Str(
            Rc::new("two".into())
        )
    );
}

#[test]
fn match_wildcard() {
    let result =
        run(
            r#"
            let x = 42

            match x {
                0 => "zero"
                _ => "other"
            }
            "#
        );

    assert_eq!(
        result,
        Value::Str(
            Rc::new("other".into())
        )
    );
}

#[test]
fn match_enum_unit_variant() {
    let result =
        run(
            r#"
            enum Color {
                Red
                Green
                Blue
            }

            let color =
                Color.Green

            match color {
                Color.Red => 1
                Color.Green => 2
                Color.Blue => 3
                _ => 0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(2)
    );
}

#[test]
fn match_enum_mixed_variants() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value)
                Err(error)
                Empty
            }

            let result =
                Result.Empty

            match result {
                Result.Ok(value) => 1
                Result.Err(error) => 2
                Result.Empty => 3
                _ => 4
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(3)
    );
}

#[test]
fn match_enum_payload() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value)
                Err(error)
            }

            let result =
                Result.Ok(42)

            match result {
                Result.Ok(value) =>
                    value + 1

                Result.Err(error) =>
                    0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(43)
    );
}

#[test]
fn match_nested_enum() {
    let result =
        run(
            r#"
            enum Option {
                Some(value)
                None
            }

            enum Result {
                Ok(value)
                Err(error)
            }

            let value =
                Result.Ok(
                    Option.Some(42)
                )

            match value {
                Result.Ok(
                    Option.Some(x)
                ) =>
                    x

                _ =>
                    0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn match_binding_is_local() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value)
                Err(error)
            }

            let result =
                Result.Ok(42)

            let output =
                match result {
                    Result.Ok(value) =>
                        value + 1

                    _ =>
                        0
                }

            output
            "#
        );

    assert_eq!(
        result,
        Value::Int(43)
    );
}

#[test]
fn match_binding_does_not_escape() {
    let result =
        run_result(
            r#"
            enum Result {
                Ok(value)
            }

            let result =
                Result.Ok(42)

            match result {
                Result.Ok(value) =>
                    value
            }

            value
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn match_arm_block() {
    let result =
        run(
            r#"
            let x = 2

            match x {
                2 => {
                    let y = 10
                    y + 1
                }

                _ => 0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(11)
    );
}

#[test]
fn match_arm_return() {
    let result =
        run(
            r#"
            let test = |x| {
                match x {
                    0 => return 10
                    _ => 20
                }
            }

            test(0)
            "#
        );

    assert_eq!(
        result,
        Value::Int(10)
    );
}

#[test]
fn match_arm_break() {
    let result =
        run(
            r#"
            let x = 0

            while true {
                match x {
                    0 => break
                    _ => null
                }
            }

            42
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn option_some() {
    let result =
        run(
            r#"
            Option.Some(42)
            "#
        );

    match result {
        Value::EnumValue(value) => {
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
                "expected EnumValue, got {:?}",
                other
            );
        }
    }
}

#[test]
fn option_none() {
    let result =
        run(
            r#"
            Option.None
            "#
        );

    match result {
        Value::EnumValue(value) => {
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
                "expected EnumValue, got {:?}",
                other
            );
        }
    }
}

#[test]
fn result_ok() {
    let result =
        run(
            r#"
            Result.Ok(42)
            "#
        );

    match result {
        Value::EnumValue(value) => {
            assert_eq!(
                value.enum_name(),
                "Result"
            );

            assert_eq!(
                value.variant(),
                "Ok"
            );

            assert_eq!(
                value.fields(),
                &[Value::Int(42)]
            );
        }

        other => {
            panic!(
                "expected EnumValue, got {:?}",
                other
            );
        }
    }
}

#[test]
fn option_match() {
    let result =
        run(
            r#"
            let x =
                Option.Some(42)

            match x {
                Option.Some(value) =>
                    value + 1

                Option.None =>
                    0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(43)
    );
}

#[test]
fn option_match_none() {
    let result =
        run(
            r#"
            let x =
                Option.None

            match x {
                Option.Some(value) =>
                    value + 1

                Option.None =>
                    0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(0)
    );
}

#[test]
fn result_match() {
    let result =
        run(
            r#"
            let result =
                Result.Ok(42)

            match result {
                Result.Ok(value) =>
                    value

                Result.Err(error) =>
                    0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn return_value() {
    let result =
        run(
            r#"
            get_value = || {
                return 42
            }

            get_value()
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn try_result_err() {
    let result =
        run_result(
            r#"
            get_value = || {
                return Result.Err("failed")?
            }

            get_value()
            "#
        );

    match result {
        Ok(value) => {
            match value {
                Value::EnumValue(value) => {
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
                        "unexpected value: {:?}",
                        other
                    );
                }
            }
        }

        Err(_) => {
            panic!(
                "unexpected error"
            );
        }
    }
}

#[test]
fn try_option_some() {
    let result =
        run(
            r#"
            get_value = || {
                return Option.Some(42)?
            }

            get_value()
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn try_option_none() {
    let result =
        run_result(
            r#"
            get_value = || {
                return Option.None?
            }

            get_value()
            "#
        );

    let value =
        result.expect(
            "function should return Option.None"
        );

    match value {
        Value::EnumValue(value) => {
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
                "unexpected value: {:?}",
                other
            );
        }
    }
}

#[test]
fn tuple_value() {
    let result =
        run(
            "(10, 20)"
        );

    match result {
        Value::Tuple(values) => {
            assert_eq!(
                values.as_ref(),
                &[
                    Value::Int(10),
                    Value::Int(20),
                ]
            );
        }

        other => panic!(
            "expected Tuple, got {:?}",
            other
        ),
    }
}

#[test]
fn tuple_index() {
    let result =
        run(
            r#"
            let p = (10, 20)
            p.1
            "#
        );

    assert_eq!(
        result,
        Value::Int(20)
    );
}

#[test]
fn match_tuple() {
    let result =
        run(
            r#"
            match (10, 20) {
                (x, y) =>
                    x + y
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(30)
    );
}

#[test]
fn match_nested_tuple() {
    let result =
        run(
            r#"
            match ((1, 2), 3) {
                ((a, b), c) =>
                    a + b + c
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(6)
    );
}

#[test]
fn match_tuple_and_enum() {
    let result =
        run(
            r#"
            enum Point {
                Cartesian(x, y)
                Empty
            }

            let p =
                Point.Cartesian(10, 20)

            match p {
                Point.Cartesian(x, y) =>
                    x + y

                Point.Empty =>
                    0
            }
            "#
        );

    assert_eq!(
        result,
        Value::Int(30)
    );
}

#[test]
fn let_tuple_destructuring() {
    let result =
        run(
            r#"
            let (x, y) =
                (10, 20)

            x + y
            "#
        );

    assert_eq!(
        result,
        Value::Int(30)
    );
}

#[test]
fn let_nested_tuple_destructuring() {
    let result =
        run(
            r#"
            let ((a, b), c) =
                ((1, 2), 3)

            a + b + c
            "#
        );

    assert_eq!(
        result,
        Value::Int(6)
    );
}

#[test]
fn list_pattern() {
    let result = run(
        r#"
        let [x, y] = [10, 20]
        x + y
        "#
    );

    assert_eq!(
        result,
        Value::Int(30)
    );
}

#[test]
fn nested_list_pattern() {
    let result = run(
        r#"
        let [[a, b], c] = [[1, 2], 3]
        a + b + c
        "#
    );

    assert_eq!(
        result,
        Value::Int(6)
    );
}

#[test]
fn let_enum_destructuring() {
    let result =
        run(
            r#"
            enum Result {
                Ok(value)
                Err(error)
            }

            let Result.Ok(x) =
                Result.Ok(42)

            x
            "#
        );

    assert_eq!(
        result,
        Value::Int(42)
    );
}

#[test]
fn let_wildcard_pattern() {
    let result =
        run(
            r#"
            let (_, x) =
                (10, 20)

            x
            "#
        );

    assert_eq!(
        result,
        Value::Int(20)
    );
}

#[test]
fn let_pattern_mismatch() {
    let result =
        run_result(
            r#"
            let (x, y) =
                (1, 2, 3)
            "#
        );

    assert!(
        result.is_err()
    );
}

#[test]
fn list_iterator_next() {
    let result =
        run(
            r#"
            let it =
                [1, 2, 3].iter()

            it.next()
            "#
        );

    match result {
        Value::EnumValue(value) => {
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
                &[Value::Int(1)]
            );
        }

        other => panic!(
            "expected Option, got {:?}",
            other
        ),
    }
}

#[test]
fn list_range_expansion() {
    let result =
        run(
            "[1..5]"
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                    Value::Int(4),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other,
        ),
    }
}

#[test]
fn iterator_map() {
    let result =
        run(
            r#"
            [1, 2, 3]
                .iter()
                .map(|x| x * 2)
                .collect()
            "#
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Int(2),
                    Value::Int(4),
                    Value::Int(6),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other
        ),
    }
}

#[test]
fn iterator_map_filter_chain() {
    let result =
        run(
            r#"
            [1, 2, 3, 4, 5]
                .iter()
                .map(|x| x * 2)
                .filter(|x| x > 5)
                .collect()
            "#
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Int(6),
                    Value::Int(8),
                    Value::Int(10),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other
        ),
    }
}

#[test]
fn range_expression_collect() {
    let result =
        run(
            r#"
            (1..5)
                .iter()
                .collect()
            "#
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                    Value::Int(4),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other
        ),
    }
}

#[test]
fn range_iterator_map() {
    let result =
        run(
            r#"
            (1..5)
                .map(|x| x * x)
                .collect()
            "#
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Int(1),
                    Value::Int(4),
                    Value::Int(9),
                    Value::Int(16),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other
        ),
    }
}

#[test]
fn range_map_matches_explicit_iterator() {
    let implicit =
        run(
            r#"
            (1..5)
                .map(|x| x * 2)
                .collect()
            "#
        );

    let explicit =
        run(
            r#"
            (1..5)
                .iter()
                .map(|x| x * 2)
                .collect()
            "#
        );

    assert_eq!(
        implicit,
        explicit,
    );
}

#[test]
fn range_expression() {
    let result =
        run(
            "(1..5)"
        );

    assert_eq!(
        result,
        Value::Range(
            1,
            5,
            false,
        )
    );
}

#[test]
fn list_slice_range() {
    let result =
        run(
            "[1,2,3,4,5][1..4]"
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Int(2),
                    Value::Int(3),
                    Value::Int(4),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other,
        ),
    }
}

#[test]
fn inclusive_range_expression() {
    let result =
        run(
            "(1..=5)"
        );

    assert_eq!(
        result,
        Value::Range(
            1,
            5,
            true,
        )
    );
}

#[test]
fn string_chars_iterator() {
    let result =
        run(
            r#"
            "abc"
                .chars()
                .collect()
            "#
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Str(
                        Rc::new("a".into())
                    ),
                    Value::Str(
                        Rc::new("b".into())
                    ),
                    Value::Str(
                        Rc::new("c".into())
                    ),
                ]
            );
        }

        other => panic!(
            "expected List, got {:?}",
            other
        ),
    }
}

#[test]
fn string_chars_unicode() {
    let result =
        run(
            r#"
            "あいう"
                .chars()
                .collect()
            "#
        );

    match result {
        Value::List(list) => {
            assert_eq!(
                list.borrow().as_slice(),
                &[
                    Value::Str(
                        Rc::new("あ".into())
                    ),
                    Value::Str(
                        Rc::new("い".into())
                    ),
                    Value::Str(
                        Rc::new("う".into())
                    ),
                ]
            );
        }

        other => {
            panic!(
                "expected List, got {:?}",
                other
            );
        }
    }
}

#[test]
fn iterator_reduce() {
    let result =
        run(
            r#"
            (1..5)
                .reduce(|a, b| a + b)
            "#
        );

    match result {
        Value::EnumValue(value) => {
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
                &[Value::Int(10)]
            );
        }

        other => {
            panic!(
                "expected Option.Some, got {:?}",
                other
            );
        }
    }
}

#[test]
fn iterator_reduce_empty() {
    let result =
        run(
            r#"
            [].iter()
                .reduce(|a, b| a + b)
            "#
        );

    match result {
        Value::EnumValue(value) => {
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
                "expected Option.None, got {:?}",
                other
            );
        }
    }
}

#[test]
fn iterator_fold() {
    let result =
        run(
            r#"
            (1..5)
                .fold(
                    10,
                    |acc, x| acc + x
                )
            "#
        );

    assert_eq!(
        result,
        Value::Int(20)
    );
}

#[test]
fn iterator_any() {
    let result =
        run(
            r#"
            (1..5)
                .any(|x| x > 3)
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

#[test]
fn iterator_all() {
    let result =
        run(
            r#"
            (1..5)
                .all(|x| x > 0)
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

#[test]
fn iterator_all_false() {
    let result =
        run(
            r#"
            (1..5)
                .all(|x| x < 4)
            "#
        );

    assert_eq!(
        result,
        Value::Bool(false)
    );
}

#[test]
fn for_tuple_pattern() {
    let result =
        run(
            r#"
            let sum = 0

            for (x, y) in [
                (1, 2),
                (3, 4),
            ] {
                sum = sum + x + y
            }

            sum
            "#
        );

    assert_eq!(
        result,
        Value::Int(10)
    );
}

#[test]
fn for_nested_tuple_pattern() {
    let result =
        run(
            r#"
            let sum = 0

            for ((a, b), c) in [
                ((1, 2), 3),
                ((4, 5), 6),
            ] {
                sum = sum + a + b + c
            }

            sum
            "#
        );

    assert_eq!(
        result,
        Value::Int(21)
    );
}

#[test]
fn for_enum_pattern() {
    let result =
        run(
            r#"
            let sum = 0

            let values = [
                Result.Ok(10),
                Result.Ok(20),
            ]

            for Result.Ok(x) in values {
                sum = sum + x
            }

            sum
            "#
        );

    assert_eq!(
        result,
        Value::Int(30)
    );
}

#[test]
fn iterator_enumerate() {
    let result =
        run(
            r#"
            [10,20,30]
                .iter()
                .enumerate()
                .collect()
            "#
        );

    // expected:
    // [(0,10), (1,20), (2,30)]
}

#[test]
fn iterator_zip() {
    let result =
        run(
            r#"
            [1,2,3]
                .iter()
                .zip(
                    [4,5,6].iter()
                )
                .collect()
            "#
        );

    // [(1,4), (2,5), (3,6)]
}

#[test]
fn iterator_take() {
    let result =
        run(
            r#"
            (1..100)
                .take(3)
                .collect()
            "#
        );

    // [1,2,3]
}

#[test]
fn iterator_skip() {
    let result =
        run(
            r#"
            (1..6)
                .skip(3)
                .collect()
            "#
        );

    // [4,5]
}

#[test]
fn iterator_zip_map() {
    let result =
        run(
            r#"
            [1,2,3]
                .iter()
                .zip(
                    [10,20,30].iter()
                )
                .map(
                    |pair| pair.0 + pair.1
                )
                .collect()
            "#
        );

    // [11,22,33]
}

#[test]
fn dict_len_method() {
    let result =
        run(
            r#"
            {"a": 1, "b": 2}.len()
            "#
        );

    assert_eq!(
        result,
        Value::Int(2)
    );
}

#[test]
fn dict_get_method() {
    let result =
        run(
            r#"
            {"a": 10}.get("a")
            "#
        );

    // 既存の option_some helper / equality test に
    // 合わせて検証
}

#[test]
fn dict_get_missing_returns_none() {
    let result =
        run(
            r#"
            {"a": 10}.get("b")
            "#
        );

    // Option.None
}

#[test]
fn dict_set_method() {
    let result =
        run(
            r#"
            let d = {"a": 1}

            d.set("b", 2)

            d["b"]
            "#
        );

    assert_eq!(
        result,
        Value::Int(2)
    );
}

#[test]
fn dict_remove_method() {
    let result =
        run(
            r#"
            let d = {"a": 1}

            d.remove("a")
            "#
        );

    // Option.Some(1)
}

#[test]
fn dict_remove_missing_returns_none() {
    let result =
        run(
            r#"
            let d = {"a": 1}

            d.remove("b")
            "#
        );

    // Option.None
}

#[test]
fn dict_contains_method() {
    assert_eq!(
        run(
            r#"
            {"a": 1}.contains("a")
            "#
        ),
        Value::Bool(true)
    );

    assert_eq!(
        run(
            r#"
            {"a": 1}.contains("b")
            "#
        ),
        Value::Bool(false)
    );
}

#[test]
fn dict_keys_method() {
    let result =
        run(
            r#"
            {"a": 1, "b": 2}
                .keys()
                .iter()
                .all(|key|
                    key == "a"
                    or key == "b"
                )
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

#[test]
fn set_constructor_removes_duplicates() {
    let result =
        run(
            r#"
            set([1,2,2,3])
                .len()
            "#
        );

    assert_eq!(
        result,
        Value::Int(3)
    );
}

#[test]
fn set_contains() {
    assert_eq!(
        run(
            r#"
            set([1,2,3])
                .contains(2)
            "#
        ),
        Value::Bool(true)
    );
}

#[test]
fn set_add_remove() {
    let result =
        run(
            r#"
            let s = set([1,2])

            s.add(3)
            s.remove(1)

            s.contains(3)
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

#[test]
fn set_union() {
    let result =
        run(
            r#"
            set([1,2])
                .union(
                    set([2,3])
                )
                .len()
            "#
        );

    assert_eq!(
        result,
        Value::Int(3)
    );
}

#[test]
fn set_intersection() {
    let result =
        run(
            r#"
            set([1,2,3])
                .intersection(
                    set([2,3,4])
                )
                .len()
            "#
        );

    assert_eq!(
        result,
        Value::Int(2)
    );
}

#[test]
fn set_difference() {
    let result =
        run(
            r#"
            set([1,2,3])
                .difference(
                    set([2,3])
                )
                .contains(1)
            "#
        );

    assert_eq!(
        result,
        Value::Bool(true)
    );
}

#[test]
fn set_iterator() {
    let result =
        run(
            r#"
            set([1,2,3])
                .iter()
                .map(|x| x * 2)
                .collect()
            "#
        );

    assert_eq!(
        result,
        Value::List(
            Rc::new(
                RefCell::new(vec![
                    Value::Int(2),
                    Value::Int(4),
                    Value::Int(6),
                ])
            )
        )
    );
}

#[test]
fn list_auto_iter_map() {
    let result =
        run(
            r#"
            [1,2,3]
                .map(|x| x * 2)
                .collect()
            "#
        );

    assert_eq!(
        result,
        Value::List(
            Rc::new(
                RefCell::new(vec![
                    Value::Int(2),
                    Value::Int(4),
                    Value::Int(6),
                ])
            )
        )
    );
}

#[test]
fn string_auto_iter_enumerate() {
    let result =
        run(
            r#"
            "abc"
                .enumerate()
                .collect()
            "#
        );

    assert_eq!(
        result,
        Value::List(
            Rc::new(
                RefCell::new(vec![
                    Value::Tuple(Rc::new(vec![
                        Value::Int(0),
                        Value::Str(Rc::new("a".into())),
                    ])),
                    Value::Tuple(Rc::new(vec![
                        Value::Int(1),
                        Value::Str(Rc::new("b".into())),
                    ])),
                    Value::Tuple(Rc::new(vec![
                        Value::Int(2),
                        Value::Str(Rc::new("c".into())),
                    ])),
                ])
            )
        )
    );
}

#[test]
fn int_float_conversion() {
    assert_eq!(
        run("int(3.9)"),
        Value::Int(3)
    );

    assert_eq!(
        run("int(-3.9)"),
        Value::Int(-3)
    );
}

#[test]
fn int_from_invalid_string() {
    assert!(
        run_result(
            r#"int("hello")"#
        ).is_err()
    );
}

#[test]
fn str_conversion() {
    assert_eq!(
        run(r#"str(123)"#),
        Value::Str(
            Rc::new("123".into())
        )
    );

    assert_eq!(
        run(r#"str(true)"#),
        Value::Str(
            Rc::new("true".into())
        )
    );
}

#[test]
fn float_from_int() {
    assert_eq!(
        run("float(10)"),
        Value::Float(10.0)
    );
}

#[test]
fn int_from_string() {
    assert_eq!(
        run(r#"int("42")"#),
        Value::Int(42)
    );
}

#[test]
fn float_from_string() {
    assert_eq!(
        run(r#"float("3.14")"#),
        Value::Float(3.14)
    );
}

#[test]
fn list_repetition() {
    let result =
        run(
            "[0] * 5"
        );

    assert_eq!(
        result,
        Value::List(
            Rc::new(
                RefCell::new(vec![
                    Value::Int(0),
                    Value::Int(0),
                    Value::Int(0),
                    Value::Int(0),
                    Value::Int(0),
                ])
            )
        )
    );
}

#[test]
fn string_repetition() {
    let result =
        run(
            r#""=" * 20"#
        );

    assert_eq!(
        result,
        Value::Str(
            Rc::new(
                "=".repeat(20)
            )
        )
    );
}

#[test]
fn zeros_builtin() {
    let result =
        run(
            "zeros(3)"
        );

    assert_eq!(
        result,
        Value::List(
            Rc::new(
                RefCell::new(vec![
                    Value::Int(0),
                    Value::Int(0),
                    Value::Int(0),
                ])
            )
        )
    );
}

#[test]
fn object_has_class() {
    let class =
        Class::new("Point");

    let class =
        Rc::new(class);

    let object =
        class.instantiate();

    assert_eq!(
        object.borrow().type_name(),
        "Point"
    );
}

#[test]
fn object_gets_method_from_class() {
    let mut class =
        Class::new("Point");

    let function = match run("|| {}") {
        Value::Func(function) => function,
        other => panic!("expected Function, got {:?}", other),
    };

    class.add_method(
        "move",
        function,
    );

    let class =
        Rc::new(class);

    let object =
        class.instantiate();

    assert!(
        object
            .borrow()
            .get_method("move")
            .is_some()
    );
}

