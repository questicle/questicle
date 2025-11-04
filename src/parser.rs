use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::{Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected end of input")]
    Eof,
    #[error("unexpected token at line {line}, col {col}")]
    Unexpected { line: usize, col: usize },
    #[error("expected {expected} at line {line}, col {col}")]
    Expected {
        expected: &'static str,
        line: usize,
        col: usize,
    },
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(src: &str) -> Self {
        let tokens = Lexer::new(src).lex();
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        Ok(Program { statements })
    }

    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        if self.matches(&[TokenKind::Let]) {
            return self.let_decl();
        }
        if self.matches(&[TokenKind::Fn]) {
            return self.fn_decl();
        }
        self.statement()
    }

    fn let_decl(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume_ident("identifier")?;
        self.consume(TokenKind::Assign, "=")?;
        let init = self.expression()?;
        self.optional(TokenKind::Semicolon);
        Ok(Stmt::Let { name, init })
    }

    fn fn_decl(&mut self) -> Result<Stmt, ParseError> {
        // function statement as: fn name(params){ body }
        let name = self.consume_ident("function name")?;
        let func = self.function_literal()?;
        Ok(Stmt::Let {
            name,
            init: Expr::Fn {
                params: func.0,
                body: func.1,
            },
        })
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.matches(&[TokenKind::If]) {
            return self.if_stmt();
        }
        if self.matches(&[TokenKind::While]) {
            return self.while_stmt();
        }
        if self.matches(&[TokenKind::For]) {
            return self.for_stmt();
        }
        if self.matches(&[TokenKind::LeftBrace]) {
            return Ok(Stmt::Block(self.block()?));
        }
        if self.matches(&[TokenKind::Return]) {
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                return Ok(Stmt::Return(None));
            }
            let v = self.expression()?;
            self.optional(TokenKind::Semicolon);
            return Ok(Stmt::Return(Some(v)));
        }
        if self.matches(&[TokenKind::Break]) {
            self.optional(TokenKind::Semicolon);
            return Ok(Stmt::Break);
        }
        if self.matches(&[TokenKind::Continue]) {
            self.optional(TokenKind::Semicolon);
            return Ok(Stmt::Continue);
        }
        let expr = self.expression()?;
        self.optional(TokenKind::Semicolon);
        Ok(Stmt::Expr(expr))
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            stmts.push(self.declaration()?);
        }
        self.consume(TokenKind::RightBrace, "}")?;
        Ok(stmts)
    }

    fn if_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenKind::LeftParen, "(")?;
        let cond = self.expression()?;
        self.consume(TokenKind::RightParen, ")")?;
        let then_branch = Box::new(self.statement()?);
        let else_branch = if self.matches(&[TokenKind::Else]) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenKind::LeftParen, "(")?;
        let cond = self.expression()?;
        self.consume(TokenKind::RightParen, ")")?;
        let body = Box::new(self.statement()?);
        Ok(Stmt::While { cond, body })
    }

    fn for_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenKind::LeftParen, "(")?;
        let name = self.consume_ident("loop variable")?;
        self.consume(TokenKind::In, "in")?;
        let iter = self.expression()?;
        self.consume(TokenKind::RightParen, ")")?;
        let body = Box::new(self.statement()?);
        Ok(Stmt::For { name, iter, body })
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.or()?;
        if self.matches(&[TokenKind::Assign]) {
            let value = self.assignment()?;
            if let Expr::Var(name) = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                });
            }
            return Err(self.error_expected("assignable expression"));
        }
        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.and()?;
        while self.matches(&[TokenKind::OrOr]) {
            let right = self.and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;
        while self.matches(&[TokenKind::AndAnd]) {
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;
        while self.matches(&[TokenKind::EqualEqual, TokenKind::BangEqual]) {
            let op = if self.prev_is(&TokenKind::EqualEqual) {
                BinOp::Eq
            } else {
                BinOp::Ne
            };
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;
        while self.matches(&[
            TokenKind::Less,
            TokenKind::LessEqual,
            TokenKind::Greater,
            TokenKind::GreaterEqual,
        ]) {
            let op = match self.previous().unwrap().kind {
                TokenKind::Less => BinOp::Lt,
                TokenKind::LessEqual => BinOp::Le,
                TokenKind::Greater => BinOp::Gt,
                TokenKind::GreaterEqual => BinOp::Ge,
                _ => unreachable!(),
            };
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;
        while self.matches(&[TokenKind::Plus, TokenKind::Minus]) {
            let op = if self.prev_is(&TokenKind::Plus) {
                BinOp::Add
            } else {
                BinOp::Sub
            };
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;
        while self.matches(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent]) {
            let op = match self.previous().unwrap().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.matches(&[TokenKind::Bang]) {
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.matches(&[TokenKind::Minus]) {
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        loop {
            if self.matches(&[TokenKind::LeftParen]) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.matches(&[TokenKind::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RightParen, ")")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.matches(&[TokenKind::LeftBracket]) {
                let idx = self.expression()?;
                self.consume(TokenKind::RightBracket, "]")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(idx),
                };
            } else if self.matches(&[TokenKind::Dot]) {
                let name = self.consume_ident("field name")?;
                expr = Expr::Field {
                    target: Box::new(expr),
                    name,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.matches(&[TokenKind::True]) {
            return Ok(Expr::Literal(Lit::Bool(true)));
        }
        if self.matches(&[TokenKind::False]) {
            return Ok(Expr::Literal(Lit::Bool(false)));
        }
        if self.matches(&[TokenKind::Null]) {
            return Ok(Expr::Literal(Lit::Null));
        }
        if self.matches_numbers() {
            return Ok(Expr::Literal(Lit::Number(self.take_number().unwrap())));
        }
        if self.matches_strings() {
            return Ok(Expr::Literal(Lit::String(self.take_string().unwrap())));
        }
        if let Some(Token {
            kind: TokenKind::Identifier(_),
            ..
        }) = self.peek()
        {
            let name = self.consume_ident("identifier")?;
            return Ok(Expr::Var(name));
        }
        if self.matches(&[TokenKind::LeftParen]) {
            let e = self.expression()?;
            self.consume(TokenKind::RightParen, ")")?;
            return Ok(e);
        }
        if self.matches(&[TokenKind::LeftBracket]) {
            let mut items = Vec::new();
            if !self.check(&TokenKind::RightBracket) {
                loop {
                    items.push(self.expression()?);
                    if !self.matches(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RightBracket, "]")?;
            return Ok(Expr::List(items));
        }
        if self.matches(&[TokenKind::LeftBrace]) {
            let mut props = Vec::new();
            if !self.check(&TokenKind::RightBrace) {
                loop {
                    let key = self.consume_ident("map key")?;
                    self.consume(TokenKind::Colon, ":")?;
                    let val = self.expression()?;
                    props.push((key, val));
                    if !self.matches(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RightBrace, "}")?;
            return Ok(Expr::Map(props));
        }
        if self.matches(&[TokenKind::Fn]) {
            let (params, body) = self.function_literal()?;
            return Ok(Expr::Fn { params, body });
        }
        Err(self.error_unexpected())
    }

    fn function_literal(&mut self) -> Result<(Vec<String>, Vec<Stmt>), ParseError> {
        self.consume(TokenKind::LeftParen, "(")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                params.push(self.consume_ident("parameter")?);
                if !self.matches(&[TokenKind::Comma]) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, ")")?;
        self.consume(TokenKind::LeftBrace, "{")?;
        let body = self.block()?;
        Ok((params, body))
    }

    // Utilities
    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    fn previous(&self) -> Option<&Token> {
        self.tokens.get(self.pos - 1)
    }
    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.previous()
    }

    fn matches(&mut self, kinds: &[TokenKind]) -> bool {
        for k in kinds {
            if self.match_kind(k) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn match_kind(&self, kind: &TokenKind) -> bool {
        self.peek().map(|t| kind_eq(&t.kind, kind)).unwrap_or(false)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek().map(|t| kind_eq(&t.kind, kind)).unwrap_or(false)
    }

    fn prev_is(&self, kind: &TokenKind) -> bool {
        self.previous()
            .map(|t| kind_eq(&t.kind, kind))
            .unwrap_or(false)
    }

    fn consume(&mut self, kind: TokenKind, expected: &'static str) -> Result<&Token, ParseError> {
        if self.check(&kind) {
            return Ok(self.advance().unwrap());
        }
        Err(self.error_expected(expected))
    }

    fn consume_ident(&mut self, expected: &'static str) -> Result<String, ParseError> {
        if let Some(Token {
            kind: TokenKind::Identifier(name),
            ..
        }) = self.peek()
        {
            let n = name.clone();
            self.advance();
            return Ok(n);
        }
        Err(self.error_expected(expected))
    }

    fn optional(&mut self, kind: TokenKind) {
        if self.check(&kind) {
            self.advance();
        }
    }

    fn matches_numbers(&self) -> bool {
        self.peek()
            .map(|t| matches!(t.kind, TokenKind::Number(_)))
            .unwrap_or(false)
    }
    fn matches_strings(&self) -> bool {
        self.peek()
            .map(|t| matches!(t.kind, TokenKind::String(_)))
            .unwrap_or(false)
    }
    fn take_number(&mut self) -> Option<f64> {
        if let Some(Token {
            kind: TokenKind::Number(n),
            ..
        }) = self.peek()
        {
            let v = *n;
            self.advance();
            Some(v)
        } else {
            None
        }
    }
    fn take_string(&mut self) -> Option<String> {
        if let Some(Token {
            kind: TokenKind::String(s),
            ..
        }) = self.peek()
        {
            let v = s.clone();
            self.advance();
            Some(v)
        } else {
            None
        }
    }
    fn take_ident(&mut self) -> Option<String> {
        if let Some(Token {
            kind: TokenKind::Identifier(s),
            ..
        }) = self.peek()
        {
            let v = s.clone();
            self.advance();
            Some(v)
        } else {
            None
        }
    }

    fn error_unexpected(&self) -> ParseError {
        if let Some(t) = self.peek() {
            ParseError::Unexpected {
                line: t.line,
                col: t.col,
            }
        } else {
            ParseError::Eof
        }
    }
    fn error_expected(&self, expected: &'static str) -> ParseError {
        if let Some(t) = self.peek() {
            ParseError::Expected {
                expected,
                line: t.line,
                col: t.col,
            }
        } else {
            ParseError::Eof
        }
    }
}

fn kind_eq(a: &TokenKind, b: &TokenKind) -> bool {
    use TokenKind::*;
    match (a, b) {
        (Identifier(_), Identifier(_)) => true,
        (String(_), String(_)) => true,
        (Number(_), Number(_)) => true,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}
