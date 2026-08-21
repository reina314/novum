use super::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Ident(String),

    List(Vec<ListItem>),
    Dict(Vec<(String, Expr)>),

    StructDecl {
        name: String,
        fields: Vec<String>,
        methods: Vec<(String, Box<Expr>)>,
    },
    EnumDecl(EnumDef),

    Import(Vec<String>),

    // Assignment & Deassignment
    Let(String, Box<Expr>),
    Assign(String, Box<Expr>),
    AssignIndex(Box<Expr>, IndexExpr, Box<Expr>),
    AssignField(Box<Expr>, String, Box<Expr>),
    Drop(String),

    Binary(BinOp, Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),

    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    Break,
    Return(Option<Box<Expr>>),
    For(String, IndexExpr, Box<Expr>),

    Block(Vec<Expr>),
    Lambda(Vec<String>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Field(Box<Expr>, String),
    Index(Box<Expr>, IndexExpr),

    Null,
}

#[derive(Debug, Clone)]
pub enum IndexExpr {
    Single(Box<Expr>),
    
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },

    Tuple(Vec<IndexExpr>),
}

#[derive(Debug, Clone)]
pub enum ListItem {
    Expr(Expr),
    Range(IndexExpr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    
    MatMul,

    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,

    And,
    Or,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "**",
            Self::Mod => "%",

            Self::MatMul => "@",

            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Leq => "<=",
            Self::Gt => ">",
            Self::Geq => ">=",

            Self::And => "and",
            Self::Or => "or",
        };
        write!(f, "{s}")
    }
}

pub fn span_of_index(
    index: &IndexExpr,
) -> Span {
    match index {
        IndexExpr::Single(expr) => {
            expr.span
        }

        IndexExpr::Range {
            start,
            end,
            ..
        } => {
            match (start, end) {
                (Some(a), Some(b)) =>
                    a.span.join(b.span),

                (Some(a), None) =>
                    a.span,

                (None, Some(b)) =>
                    b.span,

                (None, None) =>
                    Span::EMPTY,
            }
        }

        IndexExpr::Tuple(indices) => {
            match (
                indices.first(),
                indices.last(),
            ) {
                (
                    Some(first),
                    Some(last),
                ) => {
                    span_of_index(first)
                        .join(
                            span_of_index(last)
                        )
                }

                _ => Span::EMPTY,
            }
        }
    }
}
