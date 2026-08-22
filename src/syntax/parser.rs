use crate::error::{Error, Result};
use super::{ast::*, Span, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self { Self { tokens, pos: 0 } }

    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        while !self.at_kind(TokenKind::Eof) {
            if self.at_kind(TokenKind::Semicolon) { continue; }
            statements.push(self.parse_expr()?);
            self.at_kind(TokenKind::Semicolon);
        }
        Ok(Program { statements })
    }

    fn peek(&self) -> &Token { &self.tokens[self.pos] }

    fn peek_n(&self, n: usize) -> &Token {
        self.tokens
            .get(self.pos + n)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn eat(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

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

        if !token.kind.is_ident(expected) {
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

        self.eat();

        Ok(token.span)
    }

    fn eat_ident_name(&mut self) -> Result<(String, Span)> {
        let token = self.peek().clone();

        match token.kind {
            TokenKind::Ident(name) => {
                self.eat();

                Ok((name, token.span))
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
                self.eat();
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

    /// DEPRECATED
    /// 
    /// Kept only for backward compatibility.
    /// Use `check()` or `eat_if()` for future development.
    fn at_kind(&mut self, kind: TokenKind) -> bool {
        if self.peek().kind == kind { self.eat(); true } else { false }
    }

    /// Checks the next token without consuming.
    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    /// Consumes the next token if applicable.
    fn eat_if(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.eat();
            true
        } else {
            false
        }
    }

    /// Forces token cunsumption and otherwise raises an error.
    fn expect(&mut self, kind: TokenKind) -> Result<Token> {
        if self.peek().kind == kind {
            Ok(self.eat())
        } else {
            let t = self.peek().clone();
            Err(Error::parse(format!("expected {:?}, found {:?}", kind, t.kind), t.span))
        }
    }

    fn is_expr_start(&self) -> bool {
        matches!(self.peek().kind,
            TokenKind::Int(_) | TokenKind::Float(_) | TokenKind::Str(_) |
            TokenKind::Bool(_) | TokenKind::Ident(_) | TokenKind::LParen |
            TokenKind::LBrace | TokenKind::LBracket | TokenKind::Pipe |
            TokenKind::Minus | TokenKind::Not | TokenKind::If | TokenKind::While
        )
    }

    fn parse_expr(&mut self) -> Result<Expr> { self.parse_assignment() }

    fn parse_assignment(&mut self) -> Result<Expr> {
        let left =
            self.parse_range()?;

        if self.peek().kind
            == TokenKind::Equals
        {
            let eq =
                self.eat();

            let right =
                self.parse_assignment()?;

            let span =
                left.span.join(
                    right.span
                );

            return match left.kind {
                ExprKind::Ident(name) => {
                    Ok(
                        Expr::new(
                            ExprKind::Assign(
                                name,
                                Box::new(right),
                            ),
                            span,
                        )
                    )
                }

                ExprKind::Index(obj, index) => {
                    Ok(
                        Expr::new(
                            ExprKind::AssignIndex(
                                obj,
                                index,
                                Box::new(right),
                            ),
                            span,
                        )
                    )
                }

                ExprKind::Field(obj, name) => {
                    Ok(
                        Expr::new(
                            ExprKind::AssignField(
                                obj,
                                name,
                                Box::new(right),
                            ),
                            span,
                        )
                    )
                }

                _ => {
                    Err(
                        Error::parse(
                            "invalid assignment target",
                            eq.span,
                        )
                    )
                }
            };
        }

        Ok(left)
    }

    fn parse_range(&mut self) -> Result<Expr> {
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
                    start: Some(
                        Box::new(left)
                    ),
                    end: Some(
                        Box::new(end)
                    ),
                    inclusive,
                },
                span,
            )
        )
    }

    fn parse_range_tail(
        &mut self,
    ) -> Result<(bool, Span, Option<Expr>)> {
        let operator =
            self.peek().clone();

        let inclusive =
            match operator.kind {
                TokenKind::DoubleDot => {
                    self.eat();
                    false
                }

                TokenKind::DoubleDotEq => {
                    self.eat();
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

    /// Helper for `parse_range()`
    fn is_range_operator(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::DoubleDot
                | TokenKind::DoubleDotEq
        )
    }

    /// Helper for `parse_control()`
    fn is_drop_statement(&self) -> bool {
        self.at_ident("drop")
            && matches!(
                self.peek_n(1).kind,
                TokenKind::Ident(_)
            )
    }

    fn parse_control(&mut self) -> Result<Expr> {
        if self.is_drop_statement() {
            return self.parse_drop();
        }

        match self.peek().kind {
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),

            TokenKind::Break => {
                let t = self.eat();
                Ok(Expr::new(ExprKind::Break, t.span))
            }
            TokenKind::Continue => {
                let t = self.eat();
                Ok(Expr::new(ExprKind::Continue, t.span))
            }

            TokenKind::Return => self.parse_return(),
            TokenKind::Let => self.parse_let(),
            TokenKind::Struct => self.parse_struct(), 
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Import => self.parse_import(),

            _ => self.parse_or(),
        }
    }

    fn parse_return(&mut self) -> Result<Expr> {
        let start = self.expect(TokenKind::Return)?.span;
        let value = if self.is_expr_start() {
            Some(Box::new(self.parse_expr()?))
        } else { None };
        let span = value.as_ref().map(|x| start.join(x.span)).unwrap_or(start);
        Ok(Expr::new(ExprKind::Return(value), span))
    }

    fn parse_drop(
        &mut self,
    ) -> Result<Expr> {
        let start =
            self.eat_ident("drop")?;

        let (name, end) =
            self.eat_ident_name()?;

        Ok(
            Expr::new(
                ExprKind::Drop(name),
                start.join(end),
            )
        )
    }

    fn parse_let(&mut self) -> Result<Expr> {
        let start =
            self.expect(TokenKind::Let)?.span;

        let pattern =
            self.parse_pattern()?;

        self.expect(TokenKind::Equals)?;

        let value =
            self.parse_expr()?;

        Ok(
            Expr::new(
                ExprKind::Let(
                    pattern,
                    Box::new(value.clone()),
                ),
                start.join(
                    value.span
                ),
            )
        )
    }

    fn parse_struct(&mut self) -> Result<Expr> {
        let start = self.expect(TokenKind::Struct)?.span;

        // struct name
        let name_token = self.peek().clone();

        let name = match name_token.kind {
            TokenKind::Ident(name) => {
                self.eat();
                name
            }

            _ => {
                return Err(Error::parse(
                    "struct expects an identifier",
                    name_token.span,
                ));
            }
        };

        self.expect(TokenKind::LBrace)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        let mut member_names =
            std::collections::HashSet::new();

        while !self.check(TokenKind::RBrace) {
            if self.check(TokenKind::Eof) {
                return Err(Error::parse(
                    "unterminated struct declaration",
                    start,
                ));
            }

            let member_token = self.peek().clone();

            let member_name = match member_token.kind {
                TokenKind::Ident(name) => {
                    self.eat();
                    name
                }

                _ => {
                    return Err(Error::parse(
                        "expected field or method name",
                        member_token.span,
                    ));
                }
            };

            // duplicate member check
            if !member_names.insert(member_name.clone()) {
                return Err(Error::parse(
                    format!(
                        "duplicate struct member '{}'",
                        member_name
                    ),
                    member_token.span,
                ));
            }

            if self.eat_if(TokenKind::Colon) {
                // method
                let method_expr = self.parse_lambda()?;

                let lambda = match method_expr.kind {
                    ExprKind::Lambda(params, body) => {
                        if params.first().map(String::as_str)
                            != Some("self")
                        {
                            return Err(Error::parse(
                                format!(
                                    "method '{}' must have 'self' as its first parameter",
                                    member_name
                                ),
                                method_expr.span,
                            ));
                        }

                        Expr::new(
                            ExprKind::Lambda(params, body),
                            method_expr.span,
                        )
                    }

                    _ => unreachable!(
                        "parse_lambda() must return ExprKind::Lambda"
                    ),
                };

                methods.push((
                    member_name,
                    Box::new(lambda),
                ));
            } else {
                // field
                fields.push(member_name);
            }

            // optional comma
            self.eat_if(TokenKind::Comma);
        }

        let close = self.expect(TokenKind::RBrace)?.span;

        Ok(Expr::new(
            ExprKind::StructDecl {
                name,
                fields,
                methods,
            },
            start.join(close),
        ))
    }

    fn parse_enum(&mut self) -> Result<Expr> {
        let start =
            self.expect(TokenKind::Enum)?
                .span;

        let name =
            self.expect_ident()?;

        self.expect(
            TokenKind::LBrace
        )?;

        let mut variants =
            Vec::new();

        while !self.check(TokenKind::RBrace) {
            let variant_name =
                self.expect_ident()?;

            let mut fields =
                Vec::new();

            // Looking for EnumValue
            if self.check(TokenKind::LParen) {
                self.eat();

                if !self.check(TokenKind::RParen) {
                    loop {
                        fields.push(
                            self.expect_ident()?
                        );

                        if self.check(TokenKind::Comma) {
                            self.eat();

                            if self.check(TokenKind::RParen) {
                                break;
                            }

                            continue;
                        }

                        break;
                    }
                }

                self.expect(TokenKind::RParen)?;
            }

            variants.push(
                EnumVariant {
                    name: variant_name,
                    fields,
                }
            );

            self.eat_if(TokenKind::Comma);
        }

        let end =
            self.expect(
                TokenKind::RBrace
            )?.span;

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

    fn parse_import(&mut self) -> Result<Expr> {
        let start =
            self.expect(TokenKind::Import)?.span;

        let first = self.peek().clone();

        let first_name =
            match first.kind {
                TokenKind::Ident(name) => {
                    self.eat();
                    name
                }

                _ => {
                    return Err(
                        Error::parse(
                            "import expects a module name",
                            first.span,
                        )
                    );
                }
            };

        let mut parts =
            vec![first_name];

        let mut end_span =
            first.span;

        while self.eat_if(TokenKind::Dot) {
            let token =
                self.peek().clone();

            let name =
                match token.kind {
                    TokenKind::Ident(name) => {
                        self.eat();
                        name
                    }

                    _ => {
                        return Err(
                            Error::parse(
                                "expected module name after '.'",
                                token.span,
                            )
                        );
                    }
                };

            end_span =
                end_span.join(token.span);

            parts.push(name);
        }

        Ok(Expr::new(
            ExprKind::Import(parts),
            start.join(end_span),
        ))
    }

    fn parse_if(&mut self) -> Result<Expr> {
        let start = self.expect(TokenKind::If)?.span;
        let cond = self.parse_expr()?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.at_kind(TokenKind::Else) {
            Some(Box::new(if self.peek().kind == TokenKind::If {
                self.parse_if()?
            } else {
                self.parse_block()?
            }))
        } else { None };
        let end = else_branch.as_ref().map(|x| x.span).unwrap_or(then_branch.span);
        Ok(Expr::new(ExprKind::If(Box::new(cond), Box::new(then_branch), else_branch), start.join(end)))
    }

    fn parse_block(&mut self) -> Result<Expr> {
        if self.at_kind(TokenKind::LBrace) {
            let block = self.parse_block_contents()?;
            self.expect(TokenKind::RBrace)?;
            Ok(block)
        } else {
            self.parse_expr()
        }
    }

    fn parse_block_contents(&mut self) -> Result<Expr> {
        let open_span = if self.pos == 0 { Span::EMPTY } else { self.tokens[self.pos - 1].span };
        let mut exprs = Vec::new();
        while self.peek().kind != TokenKind::RBrace {
            if self.peek().kind == TokenKind::Eof {
                return Err(Error::parse("unterminated block", open_span));
            }
            if self.at_kind(TokenKind::Semicolon) { continue; }
            exprs.push(self.parse_expr()?);
            self.at_kind(TokenKind::Semicolon);
        }
        let end = self.peek().span;
        Ok(Expr::new(ExprKind::Block(exprs), open_span.join(end)))
    }

    fn parse_while(&mut self) -> Result<Expr> {
        let start = self.expect(TokenKind::While)?.span;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Expr::new(ExprKind::While(Box::new(condition), Box::new(body.clone())), start.join(body.span)))
    }

    fn parse_for(&mut self) -> Result<Expr> {
        let start =
            self.expect(TokenKind::For)?.span;

        let pattern =
            self.parse_pattern()?;

        self.expect(TokenKind::In)?;

        let iterable =
            self.parse_expr()?;

        let body =
            self.parse_block()?;

        let end =
            body.span;

        Ok(
            Expr::new(
                ExprKind::For {
                    pattern,
                    iterable: Box::new(iterable),
                    body: Box::new(body),
                },
                start.join(end),
            )
        )
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.peek().kind == TokenKind::Or {
            self.eat();
            let right = self.parse_and()?;
            let span = left.span.join(right.span);
            left = Expr::new(ExprKind::Binary(BinOp::Or, Box::new(left), Box::new(right)), span);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        while self.peek().kind == TokenKind::And {
            self.eat();
            let right = self.parse_comparison()?;
            let span = left.span.join(right.span);
            left = Expr::new(ExprKind::Binary(BinOp::And, Box::new(left), Box::new(right)), span);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_add()?;
        if let Some(op) = self.comparison_op() {
            self.eat();
            let right = self.parse_add()?;
            let span = left.span.join(right.span);
            left = Expr::new(ExprKind::Binary(op, Box::new(left), Box::new(right)), span);
            if self.comparison_op().is_some() {
                let t = self.peek().clone();
                return Err(Error::parse("comparison chaining is not supported", t.span));
            }
        }
        Ok(left)
    }

    fn comparison_op(&self) -> Option<BinOp> {
        match self.peek().kind {
            TokenKind::DoubleEq => Some(BinOp::Eq),
            TokenKind::NotEq => Some(BinOp::Neq),
            TokenKind::Less => Some(BinOp::Lt),
            TokenKind::LessEq => Some(BinOp::Leq),
            TokenKind::Greater => Some(BinOp::Gt),
            TokenKind::GreaterEq => Some(BinOp::Geq),
            _ => None,
        }
    }

    fn parse_add(&mut self) -> Result<Expr> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => Some(BinOp::Add),
                TokenKind::Minus => Some(BinOp::Sub),
                _ => None,
            };
            let Some(op) = op else { break };
            self.eat();
            let right = self.parse_term()?;
            let span = left.span.join(right.span);
            left = Expr::new(ExprKind::Binary(op, Box::new(left), Box::new(right)), span);
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek().kind {
                TokenKind::Star => Some(BinOp::Mul),
                TokenKind::Slash => Some(BinOp::Div),
                TokenKind::Percent => Some(BinOp::Mod),
                TokenKind::At => Some(BinOp::MatMul),
                _ => None,
            };
            let Some(op) = op else { break };
            self.eat();

            let right = self.parse_unary()?;
            let span = left.span.join(right.span);
            left = Expr::new(
                ExprKind::Binary(
                    op,
                    Box::new(left),
                    Box::new(right)
                ),
                span
            );
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        match self.peek().kind {
            TokenKind::Minus => {
                let start = self.eat().span;
                let expr = self.parse_unary()?;
                Ok(Expr::new(ExprKind::Neg(Box::new(expr.clone())), start.join(expr.span)))
            }
            TokenKind::Not => {
                let start = self.eat().span;
                let expr = self.parse_unary()?;
                Ok(Expr::new(ExprKind::Not(Box::new(expr.clone())), start.join(expr.span)))
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Expr> {
        let left = self.parse_postfix()?;
        if self.at_kind(TokenKind::DoubleStar) {
            let right = self.parse_unary()?;
            let span = left.span.join(right.span);
            Ok(Expr::new(ExprKind::Binary(BinOp::Pow, Box::new(left), Box::new(right)), span))
        } else {
            Ok(left)
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr =
            self.parse_primary()?;

        loop {
            match self.peek().kind.clone() {
                TokenKind::LParen => {
                    self.eat();

                    let args =
                        self.parse_args()?;

                    let end =
                        self.expect(
                            TokenKind::RParen
                        )?.span;

                    let start =
                        expr.span;

                    expr = Expr::new(
                        ExprKind::Call(
                            Box::new(expr),
                            args,
                        ),
                        start.join(end),
                    );
                }

                TokenKind::LBracket => {
                    self.eat();

                    let index =
                        self.parse_index_expr()?;

                    let end =
                        self.expect(
                            TokenKind::RBracket
                        )?.span;

                    let start =
                        expr.span;

                    expr = Expr::new(
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

                            expr = Expr::new(
                                ExprKind::Field(
                                    Box::new(expr),
                                    name,
                                ),
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

                            expr = Expr::new(
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
                    let start =
                        expr.span;

                    let question =
                        self.peek().clone();

                    self.eat();

                    expr = Expr::new(
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

    fn parse_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if self.peek().kind == TokenKind::RParen { return Ok(args); }
        loop {
            args.push(self.parse_expr()?);
            if !self.at_kind(TokenKind::Comma) { break; }
            if self.peek().kind == TokenKind::RParen { break; }
        }
        Ok(args)
    }

    fn parse_index_component(
        &mut self,
    ) -> Result<IndexExpr> {
        // ---------------------------------------------------------
        // Start of range may be omitted:
        //
        // ..5
        // ..=5
        // ---------------------------------------------------------
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

        // ---------------------------------------------------------
        // Range
        // ---------------------------------------------------------
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
                    end: end.map(Box::new),
                    inclusive,
                }
            );
        }

        // ---------------------------------------------------------
        // Single index
        // ---------------------------------------------------------
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

        if !self.eat_if(TokenKind::Comma) {
            return Ok(first);
        }

        let mut indices =
            vec![first];

        loop {
            indices.push(
                self.parse_index_component()?
            );

            if !self.eat_if(TokenKind::Comma) {
                break;
            }
        }

        Ok(IndexExpr::Tuple(indices))
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Int(n) => { 
                self.eat(); 
                Ok(Expr::new(ExprKind::Int(n), tok.span)) 
            }
            TokenKind::Float(n) => { 
                self.eat(); 
                Ok(Expr::new(ExprKind::Float(n), tok.span)) 
            }
            TokenKind::Str(s) => { 
                self.eat(); 
                Ok(Expr::new(ExprKind::Str(s), tok.span)) 
            }
            TokenKind::Bool(b) => { 
                self.eat(); 
                Ok(Expr::new(ExprKind::Bool(b), tok.span)) 
            }
            TokenKind::Ident(s) => { 
                self.eat(); 
                Ok(Expr::new(ExprKind::Ident(s), tok.span)) 
            }
            TokenKind::Null => { 
                self.eat(); 
                Ok(Expr::new(
                    ExprKind::Null,
                    tok.span,
                ))
            }
            
            TokenKind::Match 
                => self.parse_match_expr(),
            
            TokenKind::LParen => self.parse_paren_expr(),
            TokenKind::LBrace => self.parse_brace_expr(),
            TokenKind::LBracket => self.parse_list(),
            TokenKind::Pipe => self.parse_lambda(),

            _ => Err(Error::parse(format!("unexpected token {:?}", tok.kind), tok.span)),
        }
    }

    fn looks_like_dict(&self) -> bool {
        matches!(
            (
                self.peek_n(1).kind.clone(),
                self.peek_n(2).kind.clone(),
            ),
            (
                TokenKind::Str(_) | TokenKind::Ident(_),
                TokenKind::Colon
            )
        )
    }

    fn parse_dict(&mut self) -> Result<Expr> {
        let open = self.expect(TokenKind::LBrace)?.span;

        let mut entries = Vec::new();

        // treat {} as block
        if self.peek().kind == TokenKind::RBrace {
            let span = self.peek().span;

            return Err(Error::parse(
                "empty dictionary is not supported; use a non-empty dictionary",
                span,
            ));
        }

        loop {
            let key_token = self.peek().clone();

            let key = match key_token.kind {
                TokenKind::Str(key) => {
                    self.eat();
                    key
                }

                TokenKind::Ident(key) => {
                    self.eat();
                    key
                }

                _ => {
                    return Err(Error::parse(
                        "dictionary key must be a string or identifier",
                        key_token.span,
                    ));
                }
            };

            self.expect(TokenKind::Colon)?;

            let value = self.parse_expr()?;

            entries.push((key, value));

            if !self.at_kind(TokenKind::Comma) {
                break;
            }

            if self.peek().kind == TokenKind::RBrace {
                break;
            }
        }

        let close = self.expect(TokenKind::RBrace)?.span;

        Ok(Expr::new(
            ExprKind::Dict(entries),
            open.join(close),
        ))
    }

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

            // comma is optional
            self.eat_if(TokenKind::Comma);
        }

        let end =
            self.expect(
                TokenKind::RBrace
            )?
            .span;

        Ok(
            Expr::new(
                ExprKind::Match {
                    value: Box::new(value),
                    arms,
                },
                start.join(end),
            )
        )
    }

    fn parse_paren_expr(&mut self) -> Result<Expr> {
        let start = self.expect(TokenKind::LParen)?.span;

        if self.check(TokenKind::RParen) {
            let end = self.eat().span;

            return Ok(
                Expr::new(
                    ExprKind::Unit,
                    start.join(end),
                )
            );
        }

        let first =
            self.parse_expr()?;

        if !self.check(TokenKind::Comma) {
            self.expect(TokenKind::RParen)?;

            return Ok(first);
        }

        let mut elements =
            vec![first];

        while self.check(TokenKind::Comma) {
            self.eat();

            if self.check(TokenKind::RParen) {
                break;
            }

            elements.push(
                self.parse_expr()?
            );
        }

        let end =
            self.expect(TokenKind::RParen)?.span;

        Ok(
            Expr::new(
                ExprKind::Tuple(elements),
                start.join(end),
            )
        )
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        match self.peek().kind.clone() {
            // =====================================================
            // Tuple / grouping pattern
            // =====================================================
            TokenKind::LParen => {
                self.parse_tuple_pattern()
            }

            // =====================================================
            // Integer literal
            // =====================================================
            TokenKind::Int(value) => {
                self.eat();

                Ok(Pattern::Int(value))
            }

            // =====================================================
            // Float literal
            // =====================================================
            TokenKind::Float(value) => {
                self.eat();

                Ok(Pattern::Float(value))
            }

            // =====================================================
            // Bool
            // =====================================================
            TokenKind::Bool(true) => {
                self.eat();

                Ok(Pattern::Bool(true))
            }

            TokenKind::Bool(false) => {
                self.eat();

                Ok(Pattern::Bool(false))
            }

            // =====================================================
            // String
            // =====================================================
            TokenKind::Str(value) => {
                self.eat();

                Ok(Pattern::Str(value))
            }

            // =====================================================
            // Identifier / enum pattern
            // =====================================================
            TokenKind::Ident(name) => {
                self.eat();


                self.parse_ident_pattern(name)
            }

            TokenKind::Underscore => {
                self.eat();

                return Ok(
                    Pattern::Wildcard
                );
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

    /// Helper for `parse_pattern()`
    fn parse_ident_pattern(
        &mut self,
        first: String,
    ) -> Result<Pattern> {
        let mut path = vec![first];

        while self.check(TokenKind::Dot) {
            self.eat();

            path.push(
                self.expect_ident()?
            );
        }

        if path.len() == 1
            && !self.check(TokenKind::LParen)
        {
            return Ok(
                Pattern::Ident(
                    path.remove(0)
                )
            );
        }

        if !self.check(TokenKind::LParen) {
            return Ok(
                Pattern::Enum {
                    path,
                    fields: Vec::new(),
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

                if self.check(
                    TokenKind::Comma
                ) {
                    self.eat();

                    if self.check(
                        TokenKind::RParen
                    ) {
                        break;
                    }

                    continue;
                }

                break;
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

    /// Helper for `parse_tuple_pattern()`
    fn parse_tuple_pattern(
        &mut self,
    ) -> Result<Pattern> {
        self.expect(TokenKind::LParen)?;

        let first =
            self.parse_pattern()?;

        // `(x)` = grouping
        if !self.check(TokenKind::Comma) {
            self.expect(
                TokenKind::RParen
            )?;

            return Ok(first);
        }

        // `(x, ...)` = tuple pattern
        let mut patterns =
            vec![first];

        while self.check(
            TokenKind::Comma
        ) {
            self.eat();

            // trailing comma
            if self.check(
                TokenKind::RParen
            ) {
                break;
            }

            patterns.push(
                self.parse_pattern()?
            );
        }

        self.expect(
            TokenKind::RParen
        )?;

        Ok(
            Pattern::Tuple(patterns)
        )
    }

    fn parse_brace_expr(&mut self) -> Result<Expr> {
        if self.looks_like_dict() {
            self.parse_dict()
        } else {
            self.eat();

            let block = self.parse_block_contents()?;

            self.expect(TokenKind::RBrace)?;

            Ok(block)
        }
    }

    fn parse_list(
        &mut self,
    ) -> Result<Expr> {
        let open =
            self.expect(
                TokenKind::LBracket
            )?.span;

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
            // -----------------------------------------------------
            // Parse the first part without consuming `..`.
            // -----------------------------------------------------

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
                            start: Some(
                                Box::new(first)
                            ),
                            end: Some(
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
            )?.span;

        Ok(
            Expr::new(
                ExprKind::List(items),
                open.join(end),
            )
        )
    }

    fn parse_lambda(&mut self) -> Result<Expr> {
        let start = self.expect(TokenKind::Pipe)?.span;
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::Pipe {
            loop {
                let tok = self.peek().clone();
                let name = match tok.kind {
                    TokenKind::Ident(name) => { self.eat(); name }
                    _ => return Err(Error::parse("lambda parameters must be identifiers", tok.span)),
                };
                params.push(name);
                if !self.at_kind(TokenKind::Comma) { break; }
            }
        }
        self.expect(TokenKind::Pipe)?;
        let body = if self.peek().kind == TokenKind::LBrace {
            self.eat();
            let b = self.parse_block_contents()?;
            self.expect(TokenKind::RBrace)?;
            b
        } else {
            self.parse_expr()?
        };
        Ok(Expr::new(ExprKind::Lambda(params, Box::new(body.clone())), start.join(body.span)))
    }
}
