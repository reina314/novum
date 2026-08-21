mod common;

use common::{
    run, run_result
};
use novum::{Interpreter, Lexer, Parser};
use novum::runtime::{Value, Object, Series, BoundMethod, MethodReceiver};
use std::{cell::RefCell, rc::Rc};

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

