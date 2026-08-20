mod common;

use common::{
    run,
    assert_float_close,
};
use novum::{Interpreter, Lexer, Parser};
use novum::runtime::{Value, Object, Matrix};
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
