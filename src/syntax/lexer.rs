use crate::error::{Error, Result};
use super::{Span, Token, TokenKind};

pub struct Lexer<'a> {
    src: &'a str,
    chars: std::str::Chars<'a>,
    pos: usize,
    lookahead: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let mut chars = src.chars();
        let lookahead = chars.next();

        Self {
            src,
            chars,
            pos: 0,
            lookahead,
        }
    }

    pub fn source(&self) -> &'a str {
        self.src
    }

    fn peek(&self) -> Option<char> {
        self.lookahead
    }

    fn skip_trivia(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.consume();
            }

            if self.peek() == Some('/') {
                let mut look = self.chars.clone();

                if look.next() == Some('/') {
                    self.consume();
                    self.consume();

                    while !matches!(self.peek(), None | Some('\n')) {
                        self.consume();
                    }

                    continue;
                }
            }

            break;
        }
    }

    fn consume(&mut self) -> Option<char> {
        let ch = self.lookahead?;

        self.pos += ch.len_utf8();
        self.lookahead = self.chars.next();

        Some(ch)
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.consume();
            true
        } else {
            false
        }
    }

    pub fn lex(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_trivia();

            let start = self.pos;

            let Some(ch) = self.consume() else {
                break;
            };

            let kind = match ch {
                c if c.is_ascii_alphabetic() || c == '_' => {
                    self.lex_ident(c)
                }

                c if c.is_ascii_digit() => {
                    self.lex_number(c)?
                }

                '\'' | '"' => {
                    self.lex_string(ch)?
                }

                '+' => {
                    if self.consume_if('=') {
                        TokenKind::PlusEq
                    } else {
                        TokenKind::Plus
                    }
                }

                '-' => {
                    if self.consume_if('=') {
                        TokenKind::MinusEq
                    } else {
                        TokenKind::Minus
                    }
                }

                '*' => {
                    if self.consume_if('*') {
                        TokenKind::DoubleStar
                    } else if self.consume_if('=') {
                        TokenKind::StarEq
                    } else {
                        TokenKind::Star
                    }
                }

                '/' => {
                    if self.consume_if('=') {
                        TokenKind::SlashEq
                    } else {
                        TokenKind::Slash
                    }
                }

                '%' => {
                    if self.consume_if('=') {
                        TokenKind::PercentEq
                    } else {
                        TokenKind::Percent
                    }
                }

                '@' => TokenKind::At,

                '?' => TokenKind::Question,

                '=' => {
                    if self.consume_if('=') {
                        TokenKind::DoubleEq
                    } else 
                    if self.consume_if('>') {
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Equals
                    }
                }

                '!' => {
                    if self.consume_if('=') {
                        TokenKind::NotEq
                    } else {
                        TokenKind::Not
                    }
                }

                '<' => {
                    if self.consume_if('=') {
                        TokenKind::LessEq
                    } else {
                        TokenKind::Less
                    }
                }

                '>' => {
                    if self.consume_if('=') {
                        TokenKind::GreaterEq
                    } else {
                        TokenKind::Greater
                    }
                }

                '.' => {
                    if self.consume_if('.') {
                        if self.consume_if('=') {
                            TokenKind::DoubleDotEq
                        } else {
                            TokenKind::DoubleDot
                        }
                    } else {
                        TokenKind::Dot
                    }
                }

                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,

                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,

                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,

                '|' => TokenKind::Pipe,
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                ';' => TokenKind::Semicolon,

                _ => {
                    return Err(Error::lex(
                        format!("unexpected character {:?}", ch),
                        Span::new(start, self.pos),
                    ));
                }
            };

            tokens.push(Token {
                kind,
                span: Span::new(start, self.pos),
            });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.pos, self.pos),
        });

        Ok(tokens)
    }

    fn lex_ident(&mut self, first: char) -> TokenKind {
        let mut s = String::new();
        s.push(first);

        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(self.consume().unwrap());
            } else {
                break;
            }
        }

        match s.as_str() {
            "_" => TokenKind::Underscore,

            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),

            // syntax aliases
            "is" => TokenKind::DoubleEq,
            "not" => TokenKind::Not,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,

            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "continue" => TokenKind::Continue,
            "break" => TokenKind::Break,
            "return" => TokenKind::Return,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "let" => TokenKind::Let,
            "match" => TokenKind::Match,

            "import" => TokenKind::Import,

            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,

            "pub" => TokenKind::Pub,

            "null" => TokenKind::Null,

            _ => TokenKind::Ident(s),
        }
    }

    fn lex_number(&mut self, first: char) -> Result<TokenKind> {
        let mut digits = String::new();
        digits.push(first);

        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            digits.push(self.consume().unwrap());
        }

        // float
        if self.peek() == Some('.') {
            let mut look = self.chars.clone();

            if look
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                self.consume();
                digits.push('.');

                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    digits.push(self.consume().unwrap());
                }

                let value = digits.parse::<f64>().map_err(|_| {
                    Error::lex(
                        "invalid float literal",
                        Span::new(self.pos - digits.len(), self.pos),
                    )
                })?;

                return Ok(TokenKind::Float(value));
            }
        }

        // integer
        let value = digits.parse::<i64>().map_err(|_| {
            Error::lex(
                "integer literal overflow",
                Span::new(self.pos - digits.len(), self.pos),
            )
        })?;

        Ok(TokenKind::Int(value))
    }

    fn lex_string(&mut self, quote: char) -> Result<TokenKind> {
        let mut out = String::new();

        loop {
            let Some(c) = self.peek() else {
                return Err(Error::lex(
                    "unterminated string literal",
                    Span::new(self.pos, self.pos),
                ));
            };

            self.consume();

            match c {
                c if c == quote => break,

                '\\' => {
                    let Some(esc) = self.consume() else {
                        return Err(Error::lex(
                            "unterminated escape sequence",
                            Span::new(self.pos, self.pos),
                        ));
                    };

                    let decoded = match esc {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        '\'' => '\'',

                        other => other,
                    };

                    out.push(decoded);
                }

                _ => out.push(c),
            }
        }

        Ok(TokenKind::Str(out))
    }
}