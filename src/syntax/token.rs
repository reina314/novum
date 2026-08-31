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
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    FatArrow,

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
    Continue,
    Return,
    For,
    In,
    As,
    Let,
    Match,
    Pub,

    Struct,
    Class,
    Enum,

    Import,
    Use,

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
    Underscore,
    Question,

    DoubleDot,
    DoubleDotEq,

    Null,
    Eof,
}

impl TokenKind {
    pub fn is_ident(
        &self,
        expected: &str,
    ) -> bool {
        matches!(
            self,
            TokenKind::Ident(name)
                if name == expected
        )
    }
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
