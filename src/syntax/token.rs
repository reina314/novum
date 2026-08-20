use super::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Ident(String),

    Equals,
    Drop,

    Plus,
    Minus,
    Star,
    Slash,
    DoubleStar,
    Percent,
    At,

    DoubleEq,
    NotEq,
    Not,
    And,
    Or,
    Less,
    LessEq,
    Greater,
    GreaterEq,

    If,
    Else,
    While,
    Break,
    Return,
    For,
    In,
    Let,

    Struct,

    Import,

    LParen,
    LBrace,
    LBracket,
    RParen,
    RBrace,
    RBracket,

    Pipe,
    Dot,
    Comma,
    Colon,
    Semicolon,

    DoubleDot,
    DoubleDotEq,

    Null,
    Eof,
}

#[derive(Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{}..{}", self.kind, self.span.start, self.span.end)
    }
}
