//! Novum - a small expression-oriented programming language interpreter.

pub mod error;
pub mod runtime;
pub mod stdlib;
pub mod syntax;
pub mod vm;

pub use error::{Error, ErrorKind, Result};
pub use syntax::{Lexer, Parser};
