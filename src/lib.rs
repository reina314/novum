//! Novum - a small expression-oriented programming language interpreter.

pub mod error;
pub mod runtime;
pub mod syntax;
pub mod vm;
pub mod stdlib;

pub use error::{
    Error,
    ErrorKind,
    Result,
};
pub use syntax::{
    Lexer,
    Parser,
};
