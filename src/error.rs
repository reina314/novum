use crate::syntax::Span;
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Lex,
    Parse,
    Name,
    Type,
    Value,
    Arity,
    Index,
    DivisionByZero,
    Overflow,
    Runtime,
    Control,
    Import,
    Shape,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    pub stack: Vec<StackFrame>,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            stack: Vec::new(),
        }
    }

    pub fn lex(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Lex, message, Some(span))
    }

    pub fn parse(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Parse, message, Some(span))
    }

    pub fn with_stack(mut self, stack: &[StackFrame]) -> Self {
        self.stack = stack.to_vec();
        self
    }

    pub fn display(&self, src: &str) {
        eprintln!("{} error: {}", self.kind, self.message);
        if let Some(span) = self.span {
            render_span(src, span);
        }
        if !self.stack.is_empty() {
            eprintln!("Call stack:");
            for frame in self.stack.iter().rev() {
                match frame.span {
                    Some(span) => eprintln!("  in {} at {}..{}", frame.function, span.start, span.end),
                    None => eprintln!("  in {}", frame.function),
                }
            }
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ErrorKind::Lex => "Lex",
            ErrorKind::Parse => "Syntax",
            ErrorKind::Name => "Name",
            ErrorKind::Type => "Type",
            ErrorKind::Value => "Value",
            ErrorKind::Arity => "Arity",
            ErrorKind::Index => "Index",
            ErrorKind::DivisionByZero => "Runtime",
            ErrorKind::Overflow => "Runtime",
            ErrorKind::Runtime => "Runtime",
            ErrorKind::Control => "Control",
            ErrorKind::Import => "Import",
            ErrorKind::Shape => "shape",
        };
        write!(f, "{s}")
    }
}

fn render_span(src: &str, span: Span) {
    let mut line_start = 0usize;
    let mut line_no = 1usize;
    for (idx, ch) in src.char_indices() {
        if idx >= span.start {
            break;
        }
        if ch == '\n' {
            line_no += 1;
            line_start = idx + ch.len_utf8();
        }
    }

    let line_end = src[line_start..]
        .find('\n')
        .map(|n| line_start + n)
        .unwrap_or(src.len());
    let line = &src[line_start..line_end];
    let col = span.start.saturating_sub(line_start);
    let width = span.end.saturating_sub(span.start).max(1);

    eprintln!("  --> line {}, column {}", line_no, col + 1);
    eprintln!("  | {}", line);
    eprintln!("  | {}{}", " ".repeat(col), "^".repeat(width.min(line.len().saturating_sub(col).max(1))));
}
