//! Novum - a small expression-oriented programming language interpreter.
//!
//! The public pipeline is intentionally explicit:
//! source -> lexer -> parser -> AST -> interpreter.

pub mod error;
pub mod interpreter;
pub mod runtime;
pub mod stdlib;
pub mod syntax;

pub use error::{Error, ErrorKind, Result};
pub use interpreter::Interpreter;
pub use syntax::{Lexer, Parser};
