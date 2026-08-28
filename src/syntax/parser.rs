use crate::{
    error::{Error, Result},
    stdlib::is_self_pattern,
};
use super::{
    ast::*,
    Span,
    Token,
    TokenKind,
};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(
        tokens: Vec<Token>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
        }
    }

    pub fn parse(
        &mut self,
    ) -> Result<Program> {
        let mut statements =
            Vec::new();

        while !self.check(TokenKind::Eof) {
            if self.eat_if(TokenKind::Semicolon) {
                continue;
            }

            statements.push(
                self.parse_expr()?
            );

            self.eat_if(
                TokenKind::Semicolon
            );
        }

        Ok(
            Program {
                statements,
            }
        )
    }

    #[inline]
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    #[inline]
    fn peek_n(
        &self,
        n: usize,
    ) -> &Token {
        self.tokens
            .get(self.pos + n)
            .unwrap_or_else(|| {
                self.tokens
                    .last()
                    .expect("parser token stream is empty")
            })
    }

    #[inline]
    fn eat(&mut self) -> Token {
        let token =
            self.tokens[self.pos]
                .clone();

        self.pos += 1;

        token
    }

    #[inline]
    fn check(
        &self,
        kind: TokenKind,
    ) -> bool {
        self.peek().kind == kind
    }

    #[inline]
    fn eat_if(
        &mut self,
        kind: TokenKind,
    ) -> bool {
        if self.check(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(
        &mut self,
        kind: TokenKind,
    ) -> Result<Token> {
        if self.check(kind.clone()) {
            return Ok(
                self.eat()
            );
        }

        let token =
            self.peek().clone();

        Err(
            Error::parse(
                format!(
                    "expected {:?}, found {:?}",
                    kind,
                    token.kind,
                ),
                token.span,
            )
        )
    }

    #[inline]
    fn at_ident(
        &self,
        expected: &str,
    ) -> bool {
        self.peek()
            .kind
            .is_ident(expected)
    }

    fn eat_ident(
        &mut self,
        expected: &str,
    ) -> Result<Span> {
        let token =
            self.peek().clone();

        if !token.kind.is_ident(
            expected
        ) {
            return Err(
                Error::parse(
                    format!(
                        "expected '{}'",
                        expected
                    ),
                    token.span,
                )
            );
        }

        self.pos += 1;

        Ok(token.span)
    }

    fn eat_ident_name(
        &mut self,
    ) -> Result<(String, Span)> {
        let token =
            self.peek().clone();

        match token.kind {
            TokenKind::Ident(name) => {
                self.pos += 1;

                Ok((
                    name,
                    token.span,
                ))
            }

            _ => {
                Err(
                    Error::parse(
                        "expected identifier",
                        token.span,
                    )
                )
            }
        }
    }

    fn expect_ident(
        &mut self,
    ) -> Result<String> {
        let token =
            self.peek().clone();

        match token.kind {
            TokenKind::Ident(name) => {
                self.pos += 1;
                Ok(name)
            }

            _ => {
                Err(
                    Error::parse(
                        "expected identifier",
                        token.span,
                    )
                )
            }
        }
    }

    fn is_expr_start(
        &self,
    ) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Int(_)
                | TokenKind::Float(_)
                | TokenKind::Str(_)
                | TokenKind::Bool(_)
                | TokenKind::Ident(_)
                | TokenKind::LParen
                | TokenKind::LBrace
                | TokenKind::LBracket
                | TokenKind::Pipe
                | TokenKind::Minus
                | TokenKind::Not
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Match
                | TokenKind::Null
        )
    }

    // ============================================================
    // Expressions
    // ============================================================

    fn parse_expr(
        &mut self,
    ) -> Result<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(
        &mut self,
    ) -> Result<Expr> {
        let lhs =
            self.parse_range()?;

        if self.eat_if(
            TokenKind::Equals
        ) {
            let rhs =
                self.parse_assignment()?;

            self.validate_assignment_target(
                &lhs
            )?;

            let span =
                lhs.span.join(
                    rhs.span
                );

            return Ok(
                Expr::new(
                    ExprKind::Assign {
                        target:
                            Box::new(lhs),
                        value:
                            Box::new(rhs),
                    },
                    span,
                )
            );
        }

        if let Some(op) =
            assignment_binop(
                &self.peek().kind
            )
        {
            self.eat();

            let rhs =
                self.parse_assignment()?;

            self.validate_assignment_target(
                &lhs
            )?;

            let span =
                lhs.span.join(
                    rhs.span
                );

            return Ok(
                Expr::new(
                    ExprKind::AssignOp {
                        target:
                            Box::new(lhs),
                        op,
                        value:
                            Box::new(rhs),
                    },
                    span,
                )
            );
        }

        Ok(lhs)
    }

    fn validate_assignment_target(
        &self,
        expr: &Expr,
    ) -> Result<()> {
        match expr.kind {
            ExprKind::Ident(_)
            | ExprKind::Index(_, _)
            | ExprKind::Field { .. } => {
                Ok(())
            }

            _ => {
                Err(
                    Error::parse(
                        "invalid assignment target",
                        expr.span,
                    )
                )
            }
        }
    }

    fn parse_range(
        &mut self,
    ) -> Result<Expr> {
        let left =
            self.parse_control()?;

        if !self.is_range_operator() {
            return Ok(left);
        }

        let (
            inclusive,
            operator_span,
            end,
        ) =
            self.parse_range_tail()?;

        let end =
            end.ok_or_else(|| {
                Error::parse(
                    "range expression requires an end value",
                    operator_span,
                )
            })?;

        let span =
            left.span.join(
                end.span
            );

        Ok(
            Expr::new(
                ExprKind::Range {
                    start:
                        Some(Box::new(left)),
                    end:
                        Some(Box::new(end)),
                    inclusive,
                },
                span,
            )
        )
    }

    fn parse_range_tail(
        &mut self,
    ) -> Result<(
        bool,
        Span,
        Option<Expr>,
    )> {
        let operator =
            self.peek().clone();

        let inclusive =
            match operator.kind {
                TokenKind::DoubleDot => {
                    self.pos += 1;
                    false
                }

                TokenKind::DoubleDotEq => {
                    self.pos += 1;
                    true
                }

                _ => {
                    return Err(
                        Error::parse(
                            "expected range operator",
                            operator.span,
                        )
                    );
                }
            };

        let end =
            if self.is_expr_start() {
                Some(
                    self.parse_control()?
                )
            } else {
                None
            };

        Ok((
            inclusive,
            operator.span,
            end,
        ))
    }

    #[inline]
    fn is_range_operator(
        &self,
    ) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::DoubleDot
                | TokenKind::DoubleDotEq
        )
    }

    fn parse_control(
        &mut self,
    ) -> Result<Expr> {
        if self.is_drop_statement() {
            return self.parse_drop();
        }

        match self.peek().kind {
            TokenKind::If =>
                self.parse_if(),

            TokenKind::While =>
                self.parse_while(),

            TokenKind::For =>
                self.parse_for(),

            TokenKind::Break => {
                let token =
                    self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Break,
                        token.span,
                    )
                )
            }

            TokenKind::Continue => {
                let token =
                    self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Continue,
                        token.span,
                    )
                )
            }

            TokenKind::Return =>
                self.parse_return(),

            TokenKind::Let =>
                self.parse_let(),

            TokenKind::Struct =>
                self.parse_struct_or_class(
                    false
                ),

            TokenKind::Class =>
                self.parse_struct_or_class(
                    true
                ),

            TokenKind::Enum =>
                self.parse_enum(),

            TokenKind::Import =>
                self.parse_import(),

            TokenKind::Pub =>
                self.parse_public_declaration(),

            _ =>
                self.parse_or(),
        }
    }

    fn is_drop_statement(
        &self,
    ) -> bool {
        self.at_ident("drop")
            && matches!(
                self.peek_n(1).kind,
                TokenKind::Ident(_)
            )
    }

    fn parse_return(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::Return
            )?
            .span;

        let value =
            if self.is_expr_start() {
                Some(
                    Box::new(
                        self.parse_expr()?
                    )
                )
            } else {
                None
            };

        let span =
            value
                .as_ref()
                .map(
                    |value|
                        start.join(
                            value.span
                        )
                )
                .unwrap_or(start);

        Ok(
            Expr::new(
                ExprKind::Return(value),
                span,
            )
        )
    }

    fn parse_drop(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.eat_ident("drop")?;

        let (
            name,
            end,
        ) =
            self.eat_ident_name()?;

        Ok(
            Expr::new(
                ExprKind::Drop(name),
                start.join(end),
            )
        )
    }

    // ============================================================
    // Declarations
    // ============================================================

    fn parse_let(
        &mut self,
    ) -> Result<Expr> {
        self.parse_let_with_visibility(
            Visibility::Private
        )
    }

    fn parse_let_with_visibility(
        &mut self,
        visibility: Visibility,
    ) -> Result<Expr> {
        let start =
            self.peek().span;

        self.expect(
            TokenKind::Let
        )?;

        let pattern =
            self.parse_pattern()?;

        self.expect(
            TokenKind::Equals
        )?;

        let value =
            self.parse_expr()?;

        Ok(
            Expr::new(
                ExprKind::Let {
                    visibility,
                    pattern,
                    value:
                        Box::new(
                            value.clone()
                        ),
                },
                start.join(
                    value.span
                ),
            )
        )
    }

    fn parse_public_declaration(
        &mut self,
    ) -> Result<Expr> {
        self.expect(
            TokenKind::Pub
        )?;

        match self.peek().kind {
            TokenKind::Let =>
                self.parse_let_with_visibility(
                    Visibility::Public
                ),

            TokenKind::Struct =>
                self.parse_struct_or_class_with_visibility(
                    Visibility::Public,
                    false,
                ),

            TokenKind::Class =>
                self.parse_struct_or_class_with_visibility(
                    Visibility::Public,
                    true,
                ),

            _ => {
                Err(
                    Error::parse(
                        "expected 'let', 'struct', or 'class' after 'pub'",
                        self.peek().span,
                    )
                )
            }
        }
    }

    fn parse_struct_or_class(
        &mut self,
        is_class: bool,
    ) -> Result<Expr> {
        self.parse_struct_or_class_with_visibility(
            Visibility::Private,
            is_class,
        )
    }

    fn parse_struct_or_class_with_visibility(
        &mut self,
        visibility: Visibility,
        is_class: bool,
    ) -> Result<Expr> {
        let start =
            if is_class {
                self.expect(
                    TokenKind::Class
                )?
                .span
            } else {
                self.expect(
                    TokenKind::Struct
                )?
                .span
            };

        let name_token =
            self.peek().clone();

        let name =
            match name_token.kind {
                TokenKind::Ident(name) => {
                    self.pos += 1;
                    name
                }

                _ => {
                    return Err(
                        Error::parse(
                            if is_class {
                                "class expects an identifier"
                            } else {
                                "struct expects an identifier"
                            },
                            name_token.span,
                        )
                    )
                }
            };

        self.expect(
            TokenKind::LBrace
        )?;

        let mut fields =
            Vec::new();

        let mut methods =
            Vec::new();

        let mut member_names =
            std::collections::HashSet::new();

        while !self.check(
            TokenKind::RBrace
        ) {
            if self.check(
                TokenKind::Eof
            ) {
                return Err(
                    Error::parse(
                        if is_class {
                            "unterminated class declaration"
                        } else {
                            "unterminated struct declaration"
                        },
                        start,
                    )
                )
            }

            let member_token =
                self.peek().clone();

            let member_name =
                match member_token.kind {
                    TokenKind::Ident(name) => {
                        self.pos += 1;
                        name
                    }

                    _ => {
                        return Err(
                            Error::parse(
                                "expected field or method name",
                                member_token.span,
                            )
                        )
                    }
                };

            if !member_names.insert(
                member_name.clone()
            ) {
                return Err(
                    Error::parse(
                        format!(
                            "duplicate {} member '{}'",
                            if is_class {
                                "class"
                            } else {
                                "struct"
                            },
                            member_name,
                        ),
                        member_token.span,
                    )
                );
            }

            if self.eat_if(
                TokenKind::Equals
            ) {
                let value =
                    self.parse_expr()?;

                match &value.kind {
                    ExprKind::Lambda(
                        params,
                        _,
                    ) => {
                        if params.first()
                            .map(is_self_pattern)
                            != Some(true)
                        {
                            return Err(
                                Error::parse(
                                    format!(
                                        "method '{}' must have 'self' as its first parameter",
                                        member_name,
                                    ),
                                    value.span,
                                )
                            );
                        }

                        methods.push((
                            member_name,
                            Box::new(value),
                        ));
                    }

                    _ => {
                        fields.push((
                            member_name,
                            Some(
                                Box::new(value)
                            ),
                        ));
                    }
                }
            } else {
                fields.push((
                    member_name,
                    None,
                ));
            }

            self.eat_if(
                TokenKind::Comma
            );
        }

        let close =
            self.expect(
                TokenKind::RBrace
            )?
            .span;

        let kind =
            if is_class {
                ExprKind::ClassDecl {
                    visibility,
                    name,
                    fields,
                    methods,
                }
            } else {
                ExprKind::StructDecl {
                    visibility,
                    name,
                    fields,
                    methods,
                }
            };

        Ok(
            Expr::new(
                kind,
                start.join(close),
            )
        )
    }

    fn parse_enum(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::Enum
            )?
            .span;

        let name =
            self.expect_ident()?;

        self.expect(
            TokenKind::LBrace
        )?;

        let mut variants =
            Vec::new();

        while !self.check(
            TokenKind::RBrace
        ) {
            let variant_name =
                self.expect_ident()?;

            let mut fields =
                Vec::new();

            if self.eat_if(
                TokenKind::LParen
            ) {
                if !self.check(
                    TokenKind::RParen
                ) {
                    loop {
                        fields.push(
                            self.expect_ident()?
                        );

                        if !self.eat_if(
                            TokenKind::Comma
                        ) {
                            break;
                        }

                        if self.check(
                            TokenKind::RParen
                        ) {
                            break;
                        }
                    }
                }

                self.expect(
                    TokenKind::RParen
                )?;
            }

            variants.push(
                EnumVariant {
                    name:
                        variant_name,
                    fields,
                }
            );

            self.eat_if(
                TokenKind::Comma
            );
        }

        let end =
            self.expect(
                TokenKind::RBrace
            )?
            .span;

        Ok(
            Expr::new(
                ExprKind::EnumDecl(
                    EnumDef {
                        name,
                        variants,
                    }
                ),
                start.join(end),
            )
        )
    }

    fn parse_import(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::Import
            )?
            .span;

        let first =
            self.peek().clone();

        let first_name =
            match first.kind {
                TokenKind::Ident(name) => {
                    self.pos += 1;
                    name
                }

                _ => {
                    return Err(
                        Error::parse(
                            "import expects a module name",
                            first.span,
                        )
                    )
                }
            };

        let mut parts =
            vec![first_name];

        let mut end_span =
            first.span;

        while self.eat_if(
            TokenKind::Dot
        ) {
            let token =
                self.peek().clone();

            let name =
                match token.kind {
                    TokenKind::Ident(name) => {
                        self.pos += 1;
                        name
                    }

                    _ => {
                        return Err(
                            Error::parse(
                                "expected module name after '.'",
                                token.span,
                            )
                        )
                    }
                };

            end_span =
                token.span;

            parts.push(name);
        }

        let alias =
            if self.eat_if(
                TokenKind::As
            ) {
                let token =
                    self.peek().clone();

                let name =
                    match token.kind {
                        TokenKind::Ident(name) => {
                            self.pos += 1;
                            name
                        }

                        _ => {
                            return Err(
                                Error::parse(
                                    "expected identifier after 'as'",
                                    token.span,
                                )
                            )
                        }
                    };

                end_span =
                    token.span;

                Some(name)
            } else {
                None
            };

        Ok(
            Expr::new(
                ExprKind::Import {
                    path: parts,
                    alias,
                },
                start.join(end_span),
            )
        )
    }

    // ============================================================
    // Control flow
    // ============================================================

    fn parse_if(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::If
            )?
            .span;

        let cond =
            self.parse_expr()?;

        let then_branch =
            self.parse_block()?;

        let else_branch =
            if self.eat_if(TokenKind::Else) {
                Some(
                    Box::new(
                        if self.check(TokenKind::If) {
                            self.parse_if()?
                        } else {
                            self.parse_block()?
                        }
                    )
                )
            } else {
                None
            };

        let end =
            else_branch
                .as_ref()
                .map(|expr| expr.span)
                .unwrap_or(
                    then_branch.span
                );

        Ok(
            Expr::new(
                ExprKind::If(
                    Box::new(cond),
                    Box::new(then_branch),
                    else_branch,
                ),
                start.join(end),
            )
        )
    }

    fn parse_block(
        &mut self,
    ) -> Result<Expr> {
        if self.eat_if(
            TokenKind::LBrace
        ) {
            let block =
                self.parse_block_contents()?;

            self.expect(
                TokenKind::RBrace
            )?;

            Ok(block)
        } else {
            self.parse_expr()
        }
    }

    fn parse_block_contents(
        &mut self,
    ) -> Result<Expr> {
        let open_span =
            if self.pos == 0 {
                Span::EMPTY
            } else {
                self.tokens[
                    self.pos - 1
                ].span
            };

        let mut exprs =
            Vec::new();

        while !self.check(
            TokenKind::RBrace
        ) {
            if self.check(
                TokenKind::Eof
            ) {
                return Err(
                    Error::parse(
                        "unterminated block",
                        open_span,
                    )
                );
            }

            if self.eat_if(
                TokenKind::Semicolon
            ) {
                continue;
            }

            exprs.push(
                self.parse_expr()?
            );

            self.eat_if(
                TokenKind::Semicolon
            );
        }

        let end =
            self.peek().span;

        Ok(
            Expr::new(
                ExprKind::Block(exprs),
                open_span.join(end),
            )
        )
    }

    fn parse_while(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::While
            )?
            .span;

        let condition =
            self.parse_expr()?;

        let body =
            self.parse_block()?;

        Ok(
            Expr::new(
                ExprKind::While(
                    Box::new(condition),
                    Box::new(body.clone()),
                ),
                start.join(body.span),
            )
        )
    }

    fn parse_for(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::For
            )?
            .span;

        let pattern =
            self.parse_pattern()?;

        self.expect(
            TokenKind::In
        )?;

        let iterable =
            self.parse_expr()?;

        let body =
            self.parse_block()?;

        Ok(
            Expr::new(
                ExprKind::For {
                    pattern,
                    iterable:
                        Box::new(iterable),
                    body:
                        Box::new(body.clone()),
                },
                start.join(body.span),
            )
        )
    }

    // ============================================================
    // Binary expressions
    // ============================================================

    fn parse_or(
        &mut self,
    ) -> Result<Expr> {
        let mut left =
            self.parse_and()?;

        while self.check(
            TokenKind::Or
        ) {
            self.pos += 1;

            let right =
                self.parse_and()?;

            let span =
                left.span.join(
                    right.span
                );

            left =
                Expr::new(
                    ExprKind::Binary(
                        BinOp::Or,
                        Box::new(left),
                        Box::new(right),
                    ),
                    span,
                );
        }

        Ok(left)
    }

    fn parse_and(
        &mut self,
    ) -> Result<Expr> {
        let mut left =
            self.parse_comparison()?;

        while self.check(
            TokenKind::And
        ) {
            self.pos += 1;

            let right =
                self.parse_comparison()?;

            let span =
                left.span.join(
                    right.span
                );

            left =
                Expr::new(
                    ExprKind::Binary(
                        BinOp::And,
                        Box::new(left),
                        Box::new(right),
                    ),
                    span,
                );
        }

        Ok(left)
    }

    fn parse_comparison(
        &mut self,
    ) -> Result<Expr> {
        let mut left =
            self.parse_add()?;

        if let Some(op) =
            self.comparison_op()
        {
            self.eat();

            let right =
                self.parse_add()?;

            let span =
                left.span.join(
                    right.span
                );

            left =
                Expr::new(
                    ExprKind::Binary(
                        op,
                        Box::new(left),
                        Box::new(right),
                    ),
                    span,
                );

            if self.comparison_op()
                .is_some()
            {
                return Err(
                    Error::parse(
                        "comparison chaining is not supported",
                        self.peek().span,
                    )
                );
            }
        }

        Ok(left)
    }

    fn comparison_op(
        &self,
    ) -> Option<BinOp> {
        match self.peek().kind {
            TokenKind::DoubleEq =>
                Some(BinOp::Eq),

            TokenKind::NotEq =>
                Some(BinOp::Neq),

            TokenKind::Less =>
                Some(BinOp::Lt),

            TokenKind::LessEq =>
                Some(BinOp::Leq),

            TokenKind::Greater =>
                Some(BinOp::Gt),

            TokenKind::GreaterEq =>
                Some(BinOp::Geq),

            _ =>
                None,
        }
    }

    fn parse_add(
        &mut self,
    ) -> Result<Expr> {
        let mut left =
            self.parse_term()?;

        loop {
            let op =
                match self.peek().kind {
                    TokenKind::Plus =>
                        Some(BinOp::Add),

                    TokenKind::Minus =>
                        Some(BinOp::Sub),

                    _ =>
                        None,
                };

            let Some(op) =
                op
            else {
                break;
            };

            self.eat();

            let right =
                self.parse_term()?;

            let span =
                left.span.join(
                    right.span
                );

            left =
                Expr::new(
                    ExprKind::Binary(
                        op,
                        Box::new(left),
                        Box::new(right),
                    ),
                    span,
                );
        }

        Ok(left)
    }

    fn parse_term(
        &mut self,
    ) -> Result<Expr> {
        let mut left =
            self.parse_unary()?;

        loop {
            let op =
                match self.peek().kind {
                    TokenKind::Star =>
                        Some(BinOp::Mul),

                    TokenKind::Slash =>
                        Some(BinOp::Div),

                    TokenKind::Percent =>
                        Some(BinOp::Mod),

                    TokenKind::At =>
                        Some(BinOp::MatMul),

                    _ =>
                        None,
                };

            let Some(op) =
                op
            else {
                break;
            };

            self.eat();

            let right =
                self.parse_unary()?;

            let span =
                left.span.join(
                    right.span
                );

            left =
                Expr::new(
                    ExprKind::Binary(
                        op,
                        Box::new(left),
                        Box::new(right),
                    ),
                    span,
                );
        }

        Ok(left)
    }

    fn parse_unary(
        &mut self,
    ) -> Result<Expr> {
        match self.peek().kind {
            TokenKind::Minus => {
                let start =
                    self.eat().span;

                let expr =
                    self.parse_unary()?;

                Ok(
                    Expr::new(
                        ExprKind::Neg(
                            Box::new(
                                expr.clone()
                            )
                        ),
                        start.join(
                            expr.span
                        ),
                    )
                )
            }

            TokenKind::Not => {
                let start =
                    self.eat().span;

                let expr =
                    self.parse_unary()?;

                Ok(
                    Expr::new(
                        ExprKind::Not(
                            Box::new(
                                expr.clone()
                            )
                        ),
                        start.join(
                            expr.span
                        ),
                    )
                )
            }

            _ =>
                self.parse_power(),
        }
    }

    fn parse_power(
        &mut self,
    ) -> Result<Expr> {
        let left =
            self.parse_postfix()?;

        if self.eat_if(
            TokenKind::DoubleStar
        ) {
            let right =
                self.parse_unary()?;

            let span =
                left.span.join(
                    right.span
                );

            Ok(
                Expr::new(
                    ExprKind::Binary(
                        BinOp::Pow,
                        Box::new(left),
                        Box::new(right),
                    ),
                    span,
                )
            )
        } else {
            Ok(left)
        }
    }

    // ============================================================
    // Postfix
    // ============================================================

    fn parse_postfix(
        &mut self,
    ) -> Result<Expr> {
        let mut expr =
            self.parse_primary()?;

        loop {
            match self.peek().kind.clone() {
                TokenKind::LParen => {
                    let next =
                        self.peek().clone();

                    if self.is_match_arm_start() {
                        break;
                    }

                    if !self.is_adjacent(
                        expr.span,
                        next.span,
                    ) {
                        break;
                    }

                    self.eat();

                    let args =
                        self.parse_args()?;

                    let end =
                        self.expect(
                            TokenKind::RParen
                        )?
                        .span;

                    let start =
                        expr.span;

                    expr =
                        Expr::new(
                            ExprKind::Call(
                                Box::new(expr),
                                args,
                            ),
                            start.join(end),
                        );
                }

                TokenKind::LBracket => {
                    let next =
                        self.peek().clone();

                    if !self.is_adjacent(
                        expr.span,
                        next.span,
                    ) {
                        break;
                    }

                    self.eat();

                    let index =
                        self.parse_index_expr()?;

                    let end =
                        self.expect(
                            TokenKind::RBracket
                        )?
                        .span;

                    let start =
                        expr.span;

                    expr =
                        Expr::new(
                            ExprKind::Index(
                                Box::new(expr),
                                index,
                            ),
                            start.join(end),
                        );
                }

                TokenKind::Dot => {
                    self.eat();

                    let token =
                        self.peek().clone();

                    match token.kind {
                        TokenKind::Ident(name) => {
                            self.eat();

                            let start =
                                expr.span;

                            expr =
                                Expr::new(
                                    ExprKind::Field {
                                        object:
                                            Box::new(expr),
                                        name,
                                    },
                                    start.join(
                                        token.span
                                    ),
                                );
                        }

                        TokenKind::Int(index)
                            if index >= 0 =>
                        {
                            self.eat();

                            let start =
                                expr.span;

                            expr =
                                Expr::new(
                                    ExprKind::TupleIndex {
                                        object:
                                            Box::new(expr),
                                        index:
                                            index as usize,
                                    },
                                    start.join(
                                        token.span
                                    ),
                                );
                        }

                        _ => {
                            return Err(
                                Error::parse(
                                    "expected field name or tuple index after '.'",
                                    token.span,
                                )
                            );
                        }
                    }
                }

                TokenKind::Question => {
                    let token =
                        self.peek().clone();

                    if !self.is_adjacent(
                        expr.span,
                        token.span,
                    ) {
                        break;
                    }

                    let start =
                        expr.span;

                    let question =
                        self.eat();

                    expr =
                        Expr::new(
                            ExprKind::Try(
                                Box::new(expr)
                            ),
                            start.join(
                                question.span
                            ),
                        );
                }

                _ => break,
            }
        }

        Ok(expr)
    }

    /// Returns true when the upcoming `(...) =>` sequence is a
    /// match-arm pattern rather than a function-call argument list.
    fn is_match_arm_start(
        &self,
    ) -> bool {
        let opening =
            match self.peek().kind {
                TokenKind::LParen =>
                    Some(TokenKind::RParen),

                TokenKind::LBracket =>
                    Some(TokenKind::RBracket),

                _ =>
                    None,
            };

        let Some(closing) =
            opening
        else {
            return false;
        };

        let mut stack =
            vec![closing];

        let mut offset =
            1usize;

        while !stack.is_empty() {
            match self.peek_n(offset).kind {
                TokenKind::LParen => {
                    stack.push(
                        TokenKind::RParen
                    );
                }

                TokenKind::LBracket => {
                    stack.push(
                        TokenKind::RBracket
                    );
                }

                TokenKind::RParen
                | TokenKind::RBracket => {
                    let current =
                        self.peek_n(offset)
                            .kind
                            .clone();

                    if stack.last()
                        != Some(&current)
                    {
                        return false;
                    }

                    stack.pop();
                }

                TokenKind::Eof => {
                    return false;
                }

                _ => {}
            }

            offset += 1;
        }

        self.peek_n(offset)
            .kind
            == TokenKind::FatArrow
    }

    #[inline]
    fn is_adjacent(
        &self,
        left: Span,
        right: Span,
    ) -> bool {
        left.end == right.start
    }

    // ============================================================
    // Call arguments
    // ============================================================

    fn parse_args(
        &mut self,
    ) -> Result<Vec<CallArg>> {
        let mut args =
            Vec::new();

        let mut seen_named =
            false;

        let mut named =
            std::collections::HashSet::new();

        if self.check(
            TokenKind::RParen
        ) {
            return Ok(args);
        }

        loop {
            let is_named =
                matches!(
                    self.peek().kind,
                    TokenKind::Ident(_)
                )
                && self.peek_n(1).kind
                    == TokenKind::Equals;

            if is_named {
                seen_named = true;

                let token =
                    self.peek().clone();

                let name =
                    match token.kind {
                        TokenKind::Ident(name) => {
                            self.eat();
                            name
                        }

                        _ =>
                            unreachable!(),
                    };

                if !named.insert(
                    name.clone()
                ) {
                    return Err(
                        Error::parse(
                            format!(
                                "duplicate named argument '{}'",
                                name
                            ),
                            token.span,
                        )
                    );
                }

                self.expect(
                    TokenKind::Equals
                )?;

                let value =
                    self.parse_assignment()?;

                args.push(
                    CallArg::named(
                        name,
                        value,
                    )
                );
            } else {
                if seen_named {
                    return Err(
                        Error::parse(
                            "positional argument cannot appear after named argument",
                            self.peek().span,
                        )
                    );
                }

                let value =
                    self.parse_assignment()?;

                args.push(
                    CallArg::positional(
                        value
                    )
                );
            }

            if !self.eat_if(
                TokenKind::Comma
            ) {
                break;
            }

            if self.check(
                TokenKind::RParen
            ) {
                break;
            }
        }

        Ok(args)
    }

    // ============================================================
    // Indexing
    // ============================================================

    fn parse_index_component(
        &mut self,
    ) -> Result<IndexExpr> {
        let start =
            if self.is_range_operator() {
                None
            } else {
                Some(
                    Box::new(
                        self.parse_control()?
                    )
                )
            };

        if self.is_range_operator() {
            let (
                inclusive,
                _operator_span,
                end,
            ) =
                self.parse_range_tail()?;

            return Ok(
                IndexExpr::Range {
                    start,
                    end:
                        end.map(Box::new),
                    inclusive,
                }
            );
        }

        match start {
            Some(expr) =>
                Ok(
                    IndexExpr::Single(expr)
                ),

            None =>
                Err(
                    Error::parse(
                        "expected index expression",
                        self.peek().span,
                    )
                ),
        }
    }

    fn parse_index_expr(
        &mut self,
    ) -> Result<IndexExpr> {
        let first =
            self.parse_index_component()?;

        if !self.eat_if(
            TokenKind::Comma
        ) {
            return Ok(first);
        }

        let mut indices =
            vec![first];

        loop {
            indices.push(
                self.parse_index_component()?
            );

            if !self.eat_if(
                TokenKind::Comma
            ) {
                break;
            }
        }

        Ok(
            IndexExpr::Tuple(indices)
        )
    }

    // ============================================================
    // Primary expressions
    // ============================================================

    fn parse_primary(
        &mut self,
    ) -> Result<Expr> {
        let token =
            self.peek().clone();

        match token.kind {
            TokenKind::Int(value) => {
                self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Int(value),
                        token.span,
                    )
                )
            }

            TokenKind::Float(value) => {
                self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Float(value),
                        token.span,
                    )
                )
            }

            TokenKind::Str(value) => {
                self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Str(value),
                        token.span,
                    )
                )
            }

            TokenKind::Bool(value) => {
                self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Bool(value),
                        token.span,
                    )
                )
            }

            TokenKind::Ident(value) => {
                self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Ident(value),
                        token.span,
                    )
                )
            }

            TokenKind::Null => {
                self.eat();

                Ok(
                    Expr::new(
                        ExprKind::Null,
                        token.span,
                    )
                )
            }

            TokenKind::Match =>
                self.parse_match_expr(),

            TokenKind::LParen =>
                self.parse_paren_expr(),

            TokenKind::LBrace =>
                self.parse_brace_expr(),

            TokenKind::LBracket =>
                self.parse_list(),

            TokenKind::Pipe =>
                self.parse_lambda(),

            _ => {
                Err(
                    Error::parse(
                        format!(
                            "unexpected token {:?}",
                            token.kind
                        ),
                        token.span,
                    )
                )
            }
        }
    }

    // ============================================================
    // Dictionary / list / lambda
    // ============================================================

    fn parse_dict(
        &mut self,
    ) -> Result<Expr> {
        let open =
            self.expect(
                TokenKind::LBrace
            )?
            .span;

        let mut entries =
            Vec::new();

        if self.check(
            TokenKind::RBrace
        ) {
            let close =
                self.eat()
                    .span;

            return Ok(
                Expr::new(
                    ExprKind::Dict(entries),
                    open.join(close),
                )
            );
        }

        loop {
            let key_token =
                self.peek().clone();

            let key =
                match key_token.kind {
                    TokenKind::Str(key) => {
                        self.eat();
                        key
                    }

                    TokenKind::Ident(key) => {
                        self.eat();
                        key
                    }

                    _ => {
                        return Err(
                            Error::parse(
                                "dictionary key must be a string or identifier",
                                key_token.span,
                            )
                        )
                    }
                };

            self.expect(
                TokenKind::Colon
            )?;

            let value =
                self.parse_expr()?;

            entries.push((
                key,
                value,
            ));

            if !self.eat_if(
                TokenKind::Comma
            ) {
                break;
            }

            if self.check(
                TokenKind::RBrace
            ) {
                break;
            }
        }

        let close =
            self.expect(
                TokenKind::RBrace
            )?
            .span;

        Ok(
            Expr::new(
                ExprKind::Dict(entries),
                open.join(close),
            )
        )
    }

    fn parse_list(
        &mut self,
    ) -> Result<Expr> {
        let open =
            self.expect(
                TokenKind::LBracket
            )?
            .span;

        let mut items =
            Vec::new();

        if self.check(
            TokenKind::RBracket
        ) {
            let end =
                self.eat().span;

            return Ok(
                Expr::new(
                    ExprKind::List(items),
                    open.join(end),
                )
            );
        }

        loop {
            let first =
                self.parse_control()?;

            if self.is_range_operator() {
                let (
                    inclusive,
                    operator_span,
                    end,
                ) =
                    self.parse_range_tail()?;

                let end =
                    end.ok_or_else(|| {
                        Error::parse(
                            "list range requires an end value",
                            operator_span,
                        )
                    })?;

                items.push(
                    ListItem::Range(
                        IndexExpr::Range {
                            start:
                                Some(
                                    Box::new(first)
                                ),
                            end:
                                Some(
                                    Box::new(end)
                                ),
                            inclusive,
                        }
                    )
                );
            } else {
                items.push(
                    ListItem::Expr(first)
                );
            }

            if !self.eat_if(
                TokenKind::Comma
            ) {
                break;
            }

            if self.check(
                TokenKind::RBracket
            ) {
                break;
            }
        }

        let end =
            self.expect(
                TokenKind::RBracket
            )?
            .span;

        Ok(
            Expr::new(
                ExprKind::List(items),
                open.join(end),
            )
        )
    }

    fn parse_lambda(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::Pipe
            )?
            .span;

        let mut params =
            Vec::new();

        if !self.check(
            TokenKind::Pipe
        ) {
            loop {
                params.push(
                    self.parse_pattern()?
                );

                if !self.eat_if(
                    TokenKind::Comma
                ) {
                    break;
                }

                if self.check(
                    TokenKind::Pipe
                ) {
                    break;
                }
            }
        }

        self.expect(
            TokenKind::Pipe
        )?;

        let body =
            if self.eat_if(
                TokenKind::LBrace
            ) {
                let block =
                    self.parse_block_contents()?;

                self.expect(
                    TokenKind::RBrace
                )?;

                block
            } else {
                self.parse_expr()?
            };

        let span =
            start.join(
                body.span
            );

        Ok(
            Expr::new(
                ExprKind::Lambda(
                    params,
                    Box::new(body),
                ),
                span,
            )
        )
    }

    // ============================================================
    // Match
    // ============================================================

    fn parse_match_expr(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::Match
            )?
            .span;

        let value =
            self.parse_expr()?;

        self.expect(
            TokenKind::LBrace
        )?;

        let mut arms =
            Vec::new();

        while !self.check(
            TokenKind::RBrace
        ) {
            let pattern =
                self.parse_pattern()?;

            self.expect(
                TokenKind::FatArrow
            )?;

            let body =
                self.parse_expr()?;

            arms.push(
                MatchArm {
                    pattern,
                    body,
                }
            );

            // Comma is deliberately optional.
            self.eat_if(
                TokenKind::Comma
            );
        }

        let end =
            self.expect(
                TokenKind::RBrace
            )?
            .span;

        Ok(
            Expr::new(
                ExprKind::Match {
                    value:
                        Box::new(value),
                    arms,
                },
                start.join(end),
            )
        )
    }

    // ============================================================
    // Braces / parentheses
    // ============================================================

    fn parse_brace_expr(
        &mut self,
    ) -> Result<Expr> {
        if self.looks_like_dict() {
            self.parse_dict()
        } else {
            self.expect(
                TokenKind::LBrace
            )?;

            let block =
                self.parse_block_contents()?;

            self.expect(
                TokenKind::RBrace
            )?;

            Ok(block)
        }
    }

    fn looks_like_dict(
        &self,
    ) -> bool {
        if self.peek_n(1).kind
            == TokenKind::RBrace
        {
            return true;
        }

        matches!(
            (
                self.peek_n(1).kind.clone(),
                self.peek_n(2).kind.clone(),
            ),
            (
                TokenKind::Str(_)
                    | TokenKind::Ident(_),
                TokenKind::Colon
            )
        )
    }

    fn parse_paren_expr(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.expect(
                TokenKind::LParen
            )?
            .span;

        if self.check(
            TokenKind::RParen
        ) {
            let end =
                self.eat().span;

            return Ok(
                Expr::new(
                    ExprKind::Unit,
                    start.join(end),
                )
            );
        }

        let first =
            self.parse_expr()?;

        if !self.eat_if(
            TokenKind::Comma
        ) {
            let end =
                self.expect(
                    TokenKind::RParen
                )?
                .span;

            return Ok(
                Expr::new(
                    first.kind,
                    start.join(end),
                )
            );
        }

        let mut elements =
            vec![first];

        while !self.check(
            TokenKind::RParen
        ) {
            elements.push(
                self.parse_expr()?
            );

            if !self.eat_if(
                TokenKind::Comma
            ) {
                break;
            }
        }

        let end =
            self.expect(
                TokenKind::RParen
            )?
            .span;

        Ok(
            Expr::new(
                ExprKind::Tuple(elements),
                start.join(end),
            )
        )
    }

    // ============================================================
    // Patterns
    // ============================================================

    fn parse_pattern(
        &mut self,
    ) -> Result<Pattern> {
        match self.peek().kind.clone() {
            TokenKind::LParen =>
                self.parse_tuple_pattern(),

            TokenKind::LBracket =>
                self.parse_list_pattern(),

            TokenKind::Int(value) => {
                self.eat();

                Ok(
                    Pattern::Int(value)
                )
            }

            TokenKind::Float(value) => {
                self.eat();

                Ok(
                    Pattern::Float(value)
                )
            }

            TokenKind::Bool(value) => {
                self.eat();

                Ok(
                    Pattern::Bool(value)
                )
            }

            TokenKind::Str(value) => {
                self.eat();

                Ok(
                    Pattern::Str(value)
                )
            }

            TokenKind::Ident(name) => {
                self.eat();

                self.parse_ident_pattern(
                    name
                )
            }

            TokenKind::Underscore => {
                self.eat();

                Ok(
                    Pattern::Wildcard
                )
            }

            _ => {
                Err(
                    Error::parse(
                        "expected pattern",
                        self.peek().span,
                    )
                )
            }
        }
    }

    fn parse_ident_pattern(
        &mut self,
        first: String,
    ) -> Result<Pattern> {
        let mut path =
            vec![first];

        while self.eat_if(
            TokenKind::Dot
        ) {
            path.push(
                self.expect_ident()?
            );
        }

        if self.check(
            TokenKind::LBrace
        ) {
            return self.parse_struct_pattern(
                path
            );
        }

        if path.len() == 1
            && !self.check(
                TokenKind::LParen
            )
        {
            return Ok(
                Pattern::Ident(
                    path.into_iter()
                        .next()
                        .unwrap()
                )
            );
        }

        if !self.check(
            TokenKind::LParen
        ) {
            return Ok(
                Pattern::Enum {
                    path,
                    fields:
                        Vec::new(),
                }
            );
        }

        self.eat();

        let mut fields =
            Vec::new();

        if !self.check(
            TokenKind::RParen
        ) {
            loop {
                fields.push(
                    self.parse_pattern()?
                );

                if !self.eat_if(
                    TokenKind::Comma
                ) {
                    break;
                }

                if self.check(
                    TokenKind::RParen
                ) {
                    break;
                }
            }
        }

        self.expect(
            TokenKind::RParen
        )?;

        Ok(
            Pattern::Enum {
                path,
                fields,
            }
        )
    }

    fn parse_tuple_pattern(
        &mut self,
    ) -> Result<Pattern> {
        self.expect(
            TokenKind::LParen
        )?;

        let first =
            self.parse_pattern()?;

        if !self.eat_if(
            TokenKind::Comma
        ) {
            self.expect(
                TokenKind::RParen
            )?;

            return Ok(first);
        }

        let mut patterns =
            vec![first];

        while !self.check(
            TokenKind::RParen
        ) {
            patterns.push(
                self.parse_pattern()?
            );

            if !self.eat_if(
                TokenKind::Comma
            ) {
                break;
            }
        }

        self.expect(
            TokenKind::RParen
        )?;

        Ok(
            Pattern::Tuple(patterns)
        )
    }

    fn parse_list_pattern(
        &mut self,
    ) -> Result<Pattern> {
        self.expect(
            TokenKind::LBracket
        )?;

        let mut patterns =
            Vec::new();

        if !self.check(
            TokenKind::RBracket
        ) {
            loop {
                patterns.push(
                    self.parse_pattern()?
                );

                if !self.eat_if(
                    TokenKind::Comma
                ) {
                    break;
                }

                if self.check(
                    TokenKind::RBracket
                ) {
                    break;
                }
            }
        }

        self.expect(
            TokenKind::RBracket
        )?;

        Ok(
            Pattern::List(patterns)
        )
    }

    fn parse_struct_pattern(
        &mut self,
        path: Vec<String>,
    ) -> Result<Pattern> {
        self.expect(
            TokenKind::LBrace
        )?;

        let mut fields =
            Vec::new();

        while !self.check(
            TokenKind::RBrace
        ) {
            let name =
                self.expect_ident()?;

            let pattern =
                if self.eat_if(
                    TokenKind::Colon
                ) {
                    self.parse_pattern()?
                } else {
                    Pattern::Ident(
                        name.clone()
                    )
                };

            fields.push(
                (name, pattern)
            );

            self.eat_if(
                TokenKind::Comma
            );
        }

        self.expect(
            TokenKind::RBrace
        )?;

        Ok(
            Pattern::Struct {
                path,
                fields,
            }
        )
    }
}

fn assignment_binop(
    token: &TokenKind,
) -> Option<BinOp> {
    match token {
        TokenKind::PlusEq =>
            Some(BinOp::Add),

        TokenKind::MinusEq =>
            Some(BinOp::Sub),

        TokenKind::StarEq =>
            Some(BinOp::Mul),

        TokenKind::SlashEq =>
            Some(BinOp::Div),

        TokenKind::PercentEq =>
            Some(BinOp::Mod),

        _ =>
            None,
    }
}