//! Novum - a small expression-oriented programming language interpreter.
//!
//! The public pipeline is intentionally explicit:
//! source -> lexer -> parser -> AST -> interpreter.

#[cfg(feature = "legacy-interpreter")]
pub mod interpreter;

#[cfg(feature = "legacy-interpreter")]
pub mod stdlib;


pub mod error;
pub mod runtime;
pub mod syntax;
pub mod vm;

pub use error::{Error, ErrorKind, Result};
pub use syntax::{Lexer, Parser};
