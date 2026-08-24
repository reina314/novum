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

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Clone)]
pub struct CallArg {
    pub name: Option<String>,
    pub value: Box<Expr>,
}

impl CallArg {
    pub fn positional(
        value: Expr,
    ) -> Self {
        Self {
            name: None,
            value: Box::new(value),
        }
    }

    pub fn named(
        name: String,
        value: Expr,
    ) -> Self {
        Self {
            name: Some(name),
            value: Box::new(value),
        }
    }
}

impl fmt::Debug for CallArg {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match &self.name {
            Some(name) => f
                .debug_struct("CallArg")
                .field("name", name)
                .field("value", &self.value)
                .finish(),

            None => f
                .debug_struct("CallArg")
                .field("value", &self.value)
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Ident(String),

    Tuple(Vec<Expr>),
    TupleIndex {
        object: Box<Expr>,
        index: usize,
    },

    List(Vec<ListItem>),
    Dict(Vec<(String, Expr)>),

    StructDecl {
        visibility: Visibility,
        name: String,
        fields: Vec<(String, Option<Box<Expr>>)>,
        methods: Vec<(String, Box<Expr>)>,
    },
    ClassDecl {
        visibility: Visibility,
        name: String,
        fields: Vec<(String, Option<Box<Expr>>)>,
        methods: Vec<(String, Box<Expr>)>,
    },
    EnumDecl(EnumDef),

    Import {
        path: Vec<String>,
        alias: Option<String>,
    },

    // Assignment & Deassignment
    Let {
        visibility: Visibility,
        pattern: Pattern, 
        value: Box<Expr>,
    },

    Assign {
        target: Box<Expr>, 
        value: Box<Expr>,
    },
    AssignOp {
        target: Box<Expr>,
        op: BinOp,
        value: Box<Expr>,
    },
    Drop(String),

    Binary(BinOp, Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),

    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    Break,
    Continue,
    Return(Option<Box<Expr>>),
    For {
        pattern: Pattern,
        iterable: Box<Expr>, 
        body: Box<Expr>,
    },
    Try(Box<Expr>),

    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    
    Block(Vec<Expr>),
    Lambda(Vec<Pattern>, Box<Expr>),
    Call(Box<Expr>, Vec<CallArg>),
    Index(
        Box<Expr>,
        IndexExpr,
    ),
    Field {
        object: Box<Expr>, 
        name: String,
    },

    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },

    Null,
    Unit,
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

impl IndexExpr {
    pub fn as_single(&self) -> Option<&Expr> {
        match self {
            IndexExpr::Single(expr) => Some(expr),
            _ => None,
        }
    }
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

#[derive(Clone, Debug)]
pub enum Pattern {
    Wildcard,

    Ident(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),

    Tuple(Vec<Pattern>),
    List(Vec<Pattern>),

    Enum {
        path: Vec<String>,
        fields: Vec<Pattern>,
    },
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
