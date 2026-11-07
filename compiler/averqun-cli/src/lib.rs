pub mod ir;
use std::{collections::HashMap, fmt, fs};

pub(crate) const MAX_CALL_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}
fn cover(start: Span, end: Span) -> Span {
    Span {
        start: start.start,
        end: end.end,
        line: start.line,
        column: start.column,
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: error: {}",
            self.span.line, self.span.column, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}
fn diag(_s: &str, m: impl Into<String>) -> Diagnostic {
    Diagnostic {
        message: m.into(),
        span: Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        },
    }
}
pub(crate) fn has_placeholder_span(span: Span) -> bool {
    span == Span {
        start: 0,
        end: 0,
        line: 1,
        column: 1,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Number(f64),
    String(String),
    Fn,
    Weave,
    Let,
    Var,
    Kept,
    Live,
    Return,
    Yield,
    If,
    Else,
    When,
    Otherwise,
    While,
    Again,
    Show,
    True,
    False,
    Nil,
    Print,
    Module,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Bang,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Arrow,
    Gives,
    Colon,
    ColonEq,
    Comma,
    Semi,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Eof,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}
impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            col: 1,
        }
    }
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }
    fn peek2(&self) -> Option<char> {
        self.src[self.pos..].chars().nth(1)
    }
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1
        } else {
            self.col += 1
        }
        Some(c)
    }
    fn span(&self, s: usize, l: usize, c: usize) -> Span {
        Span {
            start: s,
            end: self.pos,
            line: l,
            column: c,
        }
    }
    fn error(&self, m: &str, s: usize, l: usize, c: usize) -> Diagnostic {
        Diagnostic {
            message: m.into(),
            span: self.span(s, l, c),
        }
    }
    pub fn tokenize(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut out = Vec::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
                continue;
            }
            if ch == '/' && self.peek2() == Some('/') {
                while self.peek().is_some_and(|x| x != '\n') {
                    self.advance();
                }
                continue;
            }
            let (s, l, c) = (self.pos, self.line, self.col);
            let kind = if ch.is_ascii_alphabetic() || ch == '_' {
                self.advance();
                while self
                    .peek()
                    .is_some_and(|x| x.is_ascii_alphanumeric() || x == '_')
                {
                    self.advance();
                }
                match &self.src[s..self.pos] {
                    "fn" => TokenKind::Fn,
                    "weave" => TokenKind::Weave,
                    "let" => TokenKind::Let,
                    "var" => TokenKind::Var,
                    "kept" => TokenKind::Kept,
                    "live" => TokenKind::Live,
                    "return" => TokenKind::Return,
                    "yield" => TokenKind::Yield,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "when" => TokenKind::When,
                    "otherwise" => TokenKind::Otherwise,
                    "while" => TokenKind::While,
                    "again" => TokenKind::Again,
                    "show" => TokenKind::Show,
                    "gives" => TokenKind::Gives,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "nil" => TokenKind::Nil,
                    "print" => TokenKind::Print,
                    "module" => TokenKind::Module,
                    w => TokenKind::Ident(w.into()),
                }
            } else if ch.is_ascii_digit() {
                self.advance();
                while self.peek().is_some_and(|x| x.is_ascii_digit()) {
                    self.advance();
                }
                if self.peek() == Some('.') && self.peek2().is_some_and(|x| x.is_ascii_digit()) {
                    self.advance();
                    while self.peek().is_some_and(|x| x.is_ascii_digit()) {
                        self.advance();
                    }
                }
                let value = self.src[s..self.pos]
                    .parse::<f64>()
                    .map_err(|_| self.error("invalid numeric literal", s, l, c))?;
                if !value.is_finite() {
                    return Err(self.error("numeric literal is out of range", s, l, c));
                }
                TokenKind::Number(value)
            } else if ch == '"' {
                self.advance();
                let mut v = String::new();
                while let Some(x) = self.peek() {
                    if x == '"' {
                        break;
                    }
                    if x == '\n' {
                        return Err(self.error("newline in string", s, l, c));
                    }
                    if x == '\\' {
                        self.advance();
                        let e = self
                            .advance()
                            .ok_or_else(|| self.error("unterminated escape", s, l, c))?;
                        v.push(match e {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            _ => return Err(self.error("unsupported string escape", s, l, c)),
                        });
                    } else {
                        v.push(x);
                        self.advance();
                    }
                }
                if self.peek() != Some('"') {
                    return Err(self.error("unterminated string", s, l, c));
                }
                self.advance();
                TokenKind::String(v)
            } else {
                self.advance();
                match ch {
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    '{' => TokenKind::LBrace,
                    '}' => TokenKind::RBrace,
                    ',' => TokenKind::Comma,
                    ';' => TokenKind::Semi,
                    ':' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::ColonEq
                        } else {
                            TokenKind::Colon
                        }
                    }
                    '+' => TokenKind::Plus,
                    '-' => {
                        if self.peek() == Some('>') {
                            self.advance();
                            TokenKind::Arrow
                        } else {
                            TokenKind::Minus
                        }
                    }
                    '*' => TokenKind::Star,
                    '/' => TokenKind::Slash,
                    '%' => TokenKind::Percent,
                    '=' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::EqEq
                        } else {
                            TokenKind::Eq
                        }
                    }
                    '!' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::BangEq
                        } else {
                            TokenKind::Bang
                        }
                    }
                    '<' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::LtEq
                        } else {
                            TokenKind::Lt
                        }
                    }
                    '>' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::GtEq
                        } else {
                            TokenKind::Gt
                        }
                    }
                    '&' => {
                        if self.peek() == Some('&') {
                            self.advance();
                            TokenKind::AndAnd
                        } else {
                            return Err(self.error("expected '&'", s, l, c));
                        }
                    }
                    '|' => {
                        if self.peek() == Some('|') {
                            self.advance();
                            TokenKind::OrOr
                        } else {
                            return Err(self.error("expected '|'", s, l, c));
                        }
                    }
                    _ => return Err(self.error(&format!("unexpected character {ch:?}"), s, l, c)),
                }
            };
            out.push(Token {
                kind,
                span: self.span(s, l, c),
            });
        }
        out.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: self.pos,
                end: self.pos,
                line: self.line,
                column: self.col,
            },
        });
        Ok(out)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Nil,
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{n:.0}")
                } else {
                    write!(f, "{n}")
                }
            }
            Self::String(s) => write!(f, "{s}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Nil => write!(f, "nil"),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lit(Value, Span),
    Var(String, Span),
    Unary(TokenKind, Box<Expr>, Span),
    Binary(Box<Expr>, TokenKind, Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
}
impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::Lit(_, span)
            | Self::Var(_, span)
            | Self::Unary(_, _, span)
            | Self::Binary(_, _, _, span)
            | Self::Call(_, _, span) => *span,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    While(Expr, Vec<Stmt>),
    Let(String, Expr, bool),
    TypedLet(String, String, Expr, bool),
    Assign(String, Expr),
    Print(Expr),
    Expr(Expr),
    Return(Option<Expr>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    Block(Vec<Stmt>),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub param_types: Vec<Option<String>>,
    pub return_type: Option<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}
pub struct Parser {
    t: Vec<Token>,
    i: usize,
    nesting: usize,
}
const MAX_PARSE_NESTING: usize = 128;
impl Parser {
    pub fn new(mut t: Vec<Token>) -> Self {
        if !t.iter().any(|token| matches!(token.kind, TokenKind::Eof)) {
            let span = t.last().map_or(Span::default(), |token| token.span);
            t.push(Token {
                kind: TokenKind::Eof,
                span,
            });
        }
        Self {
            t,
            i: 0,
            nesting: 0,
        }
    }
    fn cur(&self) -> &Token {
        &self.t[self.i]
    }
    fn check(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(&self.cur().kind) == std::mem::discriminant(k)
    }
    fn adv(&mut self) -> Token {
        let t = self.cur().clone();
        if !self.check(&TokenKind::Eof) {
            self.i += 1
        }
        t
    }
    fn eat(&mut self, k: &TokenKind) -> bool {
        if self.check(k) {
            self.adv();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, k: &TokenKind, m: &str) -> Result<Token, Diagnostic> {
        if self.check(k) {
            Ok(self.adv())
        } else {
            Err(Diagnostic {
                message: m.into(),
                span: self.cur().span,
            })
        }
    }
    pub fn parse(&mut self) -> Result<Program, Diagnostic> {
        let mut fs = Vec::new();
        while !self.check(&TokenKind::Eof) {
            if self.eat(&TokenKind::Module) {
                self.expect(&TokenKind::Ident(String::new()), "expected module name")?;
                self.eat(&TokenKind::Semi);
                continue;
            }
            fs.push(self.function()?)
        }
        let mut names = std::collections::HashSet::new();
        for f in &fs {
            if !names.insert(f.name.clone()) {
                return Err(Diagnostic {
                    message: format!("duplicate function '{}'", f.name),
                    span: f.span,
                });
            }
        }
        Ok(Program { functions: fs })
    }
    fn function(&mut self) -> Result<Function, Diagnostic> {
        let woven = if self.eat(&TokenKind::Weave) {
            true
        } else {
            self.expect(&TokenKind::Fn, "expected 'fn' or 'weave'")?;
            false
        };
        let span = self.t[self.i - 1].span;
        let n = self.expect(&TokenKind::Ident(String::new()), "expected function name")?;
        let name = if let TokenKind::Ident(x) = n.kind {
            x
        } else {
            unreachable!()
        };
        let (left, right) = if woven {
            (TokenKind::LBracket, TokenKind::RBracket)
        } else {
            (TokenKind::LParen, TokenKind::RParen)
        };
        self.expect(&left, "expected parameter-list opener")?;
        let mut params = Vec::new();
        let mut param_types = Vec::new();
        if !self.check(&right) {
            loop {
                let p = self.expect(&TokenKind::Ident(String::new()), "expected parameter")?;
                if let TokenKind::Ident(x) = p.kind {
                    if params.contains(&x) {
                        return Err(Diagnostic {
                            message: format!("duplicate parameter '{x}'"),
                            span: p.span,
                        });
                    }
                    params.push(x)
                }
                let ty = if self.eat(&TokenKind::Colon) {
                    let t =
                        self.expect(&TokenKind::Ident(String::new()), "expected parameter type")?;
                    if let TokenKind::Ident(x) = t.kind {
                        Some(x)
                    } else {
                        None
                    }
                } else {
                    None
                };
                param_types.push(ty);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&right, "expected parameter-list closer")?;
        let has_return_type = if woven {
            self.eat(&TokenKind::Gives)
        } else {
            self.eat(&TokenKind::Arrow)
        };
        let return_type = if has_return_type {
            let t = self.expect(&TokenKind::Ident(String::new()), "expected return type")?;
            if let TokenKind::Ident(x) = t.kind {
                Some(x)
            } else {
                None
            }
        } else {
            None
        };
        let body = self.block()?;
        Ok(Function {
            name,
            params,
            param_types,
            return_type,
            body,
            span,
        })
    }
    fn block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.enter_nesting()?;
        let result = self.block_inner();
        self.nesting -= 1;
        result
    }
    fn block_inner(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        let mut b = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            b.push(self.statement()?)
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(b)
    }
    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        let mutable = if self.eat(&TokenKind::Let) || self.eat(&TokenKind::Kept) {
            Some(false)
        } else if self.eat(&TokenKind::Var) || self.eat(&TokenKind::Live) {
            Some(true)
        } else {
            None
        };
        if let Some(m) = mutable {
            let t = self.expect(&TokenKind::Ident(String::new()), "expected variable name")?;
            let n = if let TokenKind::Ident(x) = t.kind {
                x
            } else {
                unreachable!()
            };
            let annotation = if self.eat(&TokenKind::Colon) {
                let t = self.expect(&TokenKind::Ident(String::new()), "expected type name")?;
                if let TokenKind::Ident(x) = t.kind {
                    Some(x)
                } else {
                    None
                }
            } else {
                None
            };
            if !self.eat(&TokenKind::Eq) && !self.eat(&TokenKind::ColonEq) {
                return Err(Diagnostic {
                    message: "expected '=' or ':=' after binding".into(),
                    span: self.cur().span,
                });
            }
            let e = self.expr()?;
            self.eat(&TokenKind::Semi);
            return Ok(if let Some(t) = annotation {
                Stmt::TypedLet(n, t, e, m)
            } else {
                Stmt::Let(n, e, m)
            });
        }
        if self.eat(&TokenKind::While) {
            let c = self.expr()?;
            let b = self.block()?;
            return Ok(Stmt::While(c, b));
        }
        if self.eat(&TokenKind::Again) {
            self.expect(&TokenKind::While, "expected 'while' after 'again'")?;
            let c = self.expr()?;
            let b = self.block()?;
            return Ok(Stmt::While(c, b));
        }
        if self.eat(&TokenKind::Return) || self.eat(&TokenKind::Yield) {
            let x = if self.check(&TokenKind::Semi) || self.check(&TokenKind::RBrace) {
                None
            } else {
                Some(self.expr()?)
            };
            self.eat(&TokenKind::Semi);
            return Ok(Stmt::Return(x));
        }
        if self.eat(&TokenKind::If) || self.eat(&TokenKind::When) {
            let c = self.expr()?;
            let a = self.block()?;
            let b = if self.eat(&TokenKind::Else) || self.eat(&TokenKind::Otherwise) {
                if self.eat(&TokenKind::If) {
                    vec![self.statement()?]
                } else {
                    self.block()?
                }
            } else {
                vec![]
            };
            return Ok(Stmt::If(c, a, b));
        }
        if self.eat(&TokenKind::Show) {
            let value = self.expr()?;
            self.eat(&TokenKind::Semi);
            return Ok(Stmt::Print(value));
        }
        if self.check(&TokenKind::LBrace) {
            return Ok(Stmt::Block(self.block()?));
        }
        let x = self.expr()?;
        if let Expr::Var(ref n, _) = x {
            let name = n.clone();
            if self.eat(&TokenKind::Eq) || self.eat(&TokenKind::ColonEq) {
                let v = self.expr()?;
                self.eat(&TokenKind::Semi);
                return Ok(Stmt::Assign(name, v));
            }
            self.eat(&TokenKind::Semi);
            return Ok(Stmt::Expr(Expr::Var(name, x.span())));
        }
        self.eat(&TokenKind::Semi);
        Ok(Stmt::Expr(x))
    }
    fn expr(&mut self) -> Result<Expr, Diagnostic> {
        self.enter_nesting()?;
        let result = self.or();
        self.nesting -= 1;
        result
    }
    fn enter_nesting(&mut self) -> Result<(), Diagnostic> {
        if self.nesting >= MAX_PARSE_NESTING {
            return Err(Diagnostic {
                message: format!("parser nesting limit ({MAX_PARSE_NESTING}) exceeded"),
                span: self.cur().span,
            });
        }
        self.nesting += 1;
        Ok(())
    }
    fn chain(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, Diagnostic>,
        ops: &[TokenKind],
    ) -> Result<Expr, Diagnostic> {
        let mut e = next(self)?;
        loop {
            let o = ops.iter().find(|k| self.check(k)).cloned();
            if let Some(o) = o {
                self.adv();
                let r = next(self)?;
                let span = cover(e.span(), r.span());
                e = Expr::Binary(Box::new(e), o, Box::new(r), span)
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn or(&mut self) -> Result<Expr, Diagnostic> {
        self.chain(Self::and, &[TokenKind::OrOr])
    }
    fn and(&mut self) -> Result<Expr, Diagnostic> {
        self.chain(Self::eq, &[TokenKind::AndAnd])
    }
    fn eq(&mut self) -> Result<Expr, Diagnostic> {
        self.chain(Self::cmp, &[TokenKind::EqEq, TokenKind::BangEq])
    }
    fn cmp(&mut self) -> Result<Expr, Diagnostic> {
        self.chain(
            Self::term,
            &[
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
            ],
        )
    }
    fn term(&mut self) -> Result<Expr, Diagnostic> {
        self.chain(Self::factor, &[TokenKind::Plus, TokenKind::Minus])
    }
    fn factor(&mut self) -> Result<Expr, Diagnostic> {
        self.chain(
            Self::unary,
            &[TokenKind::Star, TokenKind::Slash, TokenKind::Percent],
        )
    }
    fn unary(&mut self) -> Result<Expr, Diagnostic> {
        self.enter_nesting()?;
        let result = self.unary_inner();
        self.nesting -= 1;
        result
    }
    fn unary_inner(&mut self) -> Result<Expr, Diagnostic> {
        if self.check(&TokenKind::Bang) {
            let operator = self.adv();
            let value = self.unary()?;
            let span = cover(operator.span, value.span());
            return Ok(Expr::Unary(TokenKind::Bang, Box::new(value), span));
        }
        if self.check(&TokenKind::Minus) {
            let operator = self.adv();
            let value = self.unary()?;
            let span = cover(operator.span, value.span());
            return Ok(Expr::Unary(TokenKind::Minus, Box::new(value), span));
        }
        self.primary()
    }
    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        let t = self.adv();
        match t.kind {
            TokenKind::Number(n) => Ok(Expr::Lit(Value::Number(n), t.span)),
            TokenKind::String(s) => Ok(Expr::Lit(Value::String(s), t.span)),
            TokenKind::True => Ok(Expr::Lit(Value::Bool(true), t.span)),
            TokenKind::False => Ok(Expr::Lit(Value::Bool(false), t.span)),
            TokenKind::Nil => Ok(Expr::Lit(Value::Nil, t.span)),
            TokenKind::Print => {
                self.expect(&TokenKind::LParen, "expected '(' after print")?;
                let mut a = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        a.push(self.expr()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                let close = self.expect(&TokenKind::RParen, "expected ')' after print")?;
                Ok(Expr::Call("print".into(), a, cover(t.span, close.span)))
            }
            TokenKind::Ident(n) => {
                let call_open = if self.eat(&TokenKind::LParen) {
                    Some((TokenKind::RParen, "expected ')'"))
                } else if self.eat(&TokenKind::Bang) && self.eat(&TokenKind::LBrace) {
                    Some((TokenKind::RBrace, "expected '}' after woven call arguments"))
                } else {
                    None
                };
                if let Some((close, message)) = call_open {
                    let mut a = Vec::new();
                    if !self.check(&close) {
                        loop {
                            a.push(self.expr()?);
                            if !self.eat(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    let close = self.expect(&close, message)?;
                    Ok(Expr::Call(n, a, cover(t.span, close.span)))
                } else {
                    Ok(Expr::Var(n, t.span))
                }
            }
            TokenKind::LParen => {
                let e = self.expr()?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                Ok(e)
            }
            _ => Err(Diagnostic {
                message: "expected expression".into(),
                span: t.span,
            }),
        }
    }
}
struct Env {
    scopes: Vec<HashMap<String, (Value, bool)>>,
}
impl Env {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }
    fn contains_current(&self, n: &str) -> bool {
        self.scopes.last().is_some_and(|s| s.contains_key(n))
    }
    fn define(&mut self, n: String, v: Value, mutable: bool) {
        self.scopes
            .last_mut()
            .expect("environment always has a scope")
            .insert(n, (v, mutable));
    }
    fn get(&self, n: &str) -> Option<&(Value, bool)> {
        self.scopes.iter().rev().find_map(|s| s.get(n))
    }
    fn assign(&mut self, n: &str, v: Value) -> Result<(), Diagnostic> {
        for s in self.scopes.iter_mut().rev() {
            if let Some(slot) = s.get_mut(n) {
                if !slot.1 {
                    return Err(diag("", format!("cannot assign immutable '{n}'")));
                }
                slot.0 = v;
                return Ok(());
            }
        }
        Err(diag("", format!("undefined variable '{n}'")))
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new())
    }
    fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
}
pub struct Interpreter {
    f: HashMap<String, Function>,
    fuel: u64,
    depth: usize,
    output: Vec<String>,
    capture_output: bool,
}
impl Interpreter {
    pub fn new(p: Program) -> Self {
        Self {
            f: p.functions
                .into_iter()
                .map(|x| (x.name.clone(), x))
                .collect(),
            fuel: 1_000_000,
            depth: 0,
            output: Vec::new(),
            capture_output: false,
        }
    }
    pub fn run(&mut self) -> Result<Value, Diagnostic> {
        self.output.clear();
        self.capture_output = false;
        self.call("main", vec![])
    }
    /// Execute the program and return its printed lines alongside the result.
    /// This is useful for embedding and for checking runtime backend parity.
    pub fn run_captured(&mut self) -> Result<(Value, Vec<String>), Diagnostic> {
        self.output.clear();
        self.capture_output = true;
        let result = self.call("main", vec![]);
        self.capture_output = false;
        result.map(|value| (value, std::mem::take(&mut self.output)))
    }
    fn print_value(&mut self, value: Value) {
        let line = value.to_string();
        if self.capture_output {
            self.output.push(line);
        } else {
            println!("{line}");
        }
    }
    fn call(&mut self, n: &str, a: Vec<Value>) -> Result<Value, Diagnostic> {
        if self.fuel == 0 {
            return Err(diag("", "execution limit reached"));
        }
        if self.depth >= MAX_CALL_DEPTH {
            return Err(diag("", "maximum function call depth exceeded"));
        }
        self.fuel -= 1;
        let f = self
            .f
            .get(n)
            .cloned()
            .ok_or_else(|| diag("", format!("undefined function '{n}'")))?;
        if f.params.len() != a.len() {
            return Err(Diagnostic {
                message: format!("{n} expects {} arguments, got {}", f.params.len(), a.len()),
                span: f.span,
            });
        }
        let mut e = Env::new();
        for (p, v) in f.params.iter().zip(a) {
            e.define(p.clone(), v, true);
        }
        self.depth += 1;
        let result = self.exec(&f.body, &mut e).map(|v| v.unwrap_or(Value::Nil));
        self.depth -= 1;
        result
    }
    fn exec(&mut self, b: &[Stmt], e: &mut Env) -> Result<Option<Value>, Diagnostic> {
        for s in b {
            if self.fuel == 0 {
                return Err(diag("", "execution limit reached"));
            }
            self.fuel -= 1;
            match s {
                Stmt::TypedLet(n, _, x, m) => {
                    if e.contains_current(n) {
                        return Err(diag("", format!("'{n}' already declared")));
                    }
                    let v = self.eval(x, e)?;
                    e.define(n.clone(), v, *m);
                }
                Stmt::While(c, b) => {
                    while truth(&self.eval(c, e)?) {
                        if self.fuel == 0 {
                            return Err(diag("", "execution limit reached"));
                        }
                        self.fuel -= 1;
                        if let Some(v) = self.exec_scoped(b, e)? {
                            return Ok(Some(v));
                        }
                    }
                }
                Stmt::Let(n, x, m) => {
                    if e.contains_current(n) {
                        return Err(diag("", format!("'{n}' already declared")));
                    }
                    let v = self.eval(x, e)?;
                    e.define(n.clone(), v, *m);
                }
                Stmt::Assign(n, x) => {
                    let v = self.eval(x, e)?;
                    e.assign(n, v)?
                }
                Stmt::Print(x) => {
                    let line = self.eval(x, e)?;
                    self.print_value(line);
                }
                Stmt::Expr(x) => {
                    self.eval(x, e)?;
                }
                Stmt::Return(x) => {
                    return Ok(Some(
                        x.as_ref()
                            .map(|x| self.eval(x, e))
                            .transpose()?
                            .unwrap_or(Value::Nil),
                    ))
                }
                Stmt::If(c, a, z) => {
                    if truth(&self.eval(c, e)?) {
                        if let Some(v) = self.exec_scoped(a, e)? {
                            return Ok(Some(v));
                        }
                    } else if let Some(v) = self.exec_scoped(z, e)? {
                        return Ok(Some(v));
                    }
                }
                Stmt::Block(x) => {
                    if let Some(v) = self.exec_scoped(x, e)? {
                        return Ok(Some(v));
                    }
                }
            }
        }
        Ok(None)
    }
    fn exec_scoped(&mut self, b: &[Stmt], e: &mut Env) -> Result<Option<Value>, Diagnostic> {
        e.push();
        let result = self.exec(b, e);
        e.pop();
        result
    }
    fn eval(&mut self, x: &Expr, e: &mut Env) -> Result<Value, Diagnostic> {
        self.eval_inner(x, e).map_err(|mut diagnostic| {
            if has_placeholder_span(diagnostic.span) {
                diagnostic.span = x.span();
            }
            diagnostic
        })
    }
    fn eval_inner(&mut self, x: &Expr, e: &mut Env) -> Result<Value, Diagnostic> {
        match x {
            Expr::Lit(v, _) => Ok(v.clone()),
            Expr::Var(n, _) => e
                .get(n)
                .map(|x| x.0.clone())
                .ok_or_else(|| diag("", format!("undefined variable '{n}'"))),
            Expr::Unary(o, x, _) => {
                let v = self.eval(x, e)?;
                match o {
                    TokenKind::Bang => Ok(Value::Bool(!truth(&v))),
                    TokenKind::Minus => Ok(Value::Number(-num(v)?)),
                    _ => unreachable!(),
                }
            }
            Expr::Call(n, a, _) => {
                let v = a
                    .iter()
                    .map(|x| self.eval(x, e))
                    .collect::<Result<Vec<_>, _>>()?;
                if n == "print" {
                    for x in v {
                        self.print_value(x)
                    }
                    Ok(Value::Nil)
                } else {
                    self.call(n, v)
                }
            }
            Expr::Binary(a, o, b, _) => {
                let l = self.eval(a, e)?;
                if *o == TokenKind::AndAnd && !truth(&l) {
                    return Ok(Value::Bool(false));
                }
                if *o == TokenKind::OrOr && truth(&l) {
                    return Ok(Value::Bool(true));
                }
                let r = self.eval(b, e)?;
                binary(l, o, r)
            }
        }
    }
}
fn truth(v: &Value) -> bool {
    !matches!(v, Value::Nil | Value::Bool(false))
}
fn num(v: Value) -> Result<f64, Diagnostic> {
    if let Value::Number(n) = v {
        Ok(n)
    } else {
        Err(diag("", "expected number"))
    }
}
fn binary(a: Value, o: &TokenKind, b: Value) -> Result<Value, Diagnostic> {
    use TokenKind::*;
    match o {
        Plus => match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(Value::Number(x + y)),
            (Value::String(x), Value::String(y)) => Ok(Value::String(x + &y)),
            _ => Err(diag("", "'+' expects matching numbers or strings")),
        },
        Minus | Star | Slash | Percent => {
            let (x, y) = (num(a)?, num(b)?);
            if (*o == Slash || *o == Percent) && y == 0.0 {
                return Err(diag("", "division by zero"));
            }
            Ok(Value::Number(match o {
                Minus => x - y,
                Star => x * y,
                Slash => x / y,
                Percent => x % y,
                _ => 0.0,
            }))
        }
        EqEq => Ok(Value::Bool(a == b)),
        BangEq => Ok(Value::Bool(a != b)),
        Lt | LtEq | Gt | GtEq => {
            let (x, y) = (num(a)?, num(b)?);
            Ok(Value::Bool(match o {
                Lt => x < y,
                LtEq => x <= y,
                Gt => x > y,
                _ => x >= y,
            }))
        }
        AndAnd => Ok(Value::Bool(truth(&a) && truth(&b))),
        OrOr => Ok(Value::Bool(truth(&a) || truth(&b))),
        _ => Err(diag("", "unsupported operator")),
    }
}
#[derive(Default)]
struct CheckScope {
    layers: Vec<HashMap<String, bool>>,
}
impl CheckScope {
    fn new() -> Self {
        Self {
            layers: vec![HashMap::new()],
        }
    }
    fn get(&self, n: &str) -> Option<bool> {
        self.layers.iter().rev().find_map(|s| s.get(n).copied())
    }
    fn contains_current(&self, n: &str) -> bool {
        self.layers.last().is_some_and(|s| s.contains_key(n))
    }
    fn define(&mut self, n: String, m: bool) {
        self.layers
            .last_mut()
            .expect("scope stack is never empty")
            .insert(n, m);
    }
    fn push(&mut self) {
        self.layers.push(HashMap::new())
    }
    fn pop(&mut self) {
        if self.layers.len() > 1 {
            self.layers.pop();
        }
    }
}
fn check_expr(x: &Expr, env: &CheckScope, fns: &HashMap<String, usize>) -> Result<(), Diagnostic> {
    match x {
        Expr::Lit(_, _) => Ok(()),
        Expr::Var(n, span) => {
            if env.get(n).is_some() {
                Ok(())
            } else {
                Err(Diagnostic {
                    message: format!("undefined variable '{n}'"),
                    span: *span,
                })
            }
        }
        Expr::Unary(_, a, _) => check_expr(a, env, fns),
        Expr::Binary(a, _, b, _) => {
            check_expr(a, env, fns)?;
            check_expr(b, env, fns)
        }
        Expr::Call(n, args, span) => {
            for a in args {
                check_expr(a, env, fns)?
            }
            if n == "print" {
                return Ok(());
            }
            let ar = fns.get(n).ok_or_else(|| Diagnostic {
                message: format!("undefined function '{n}'"),
                span: *span,
            })?;
            if *ar != args.len() {
                return Err(Diagnostic {
                    message: format!("function '{n}' expects {ar} arguments, got {}", args.len()),
                    span: *span,
                });
            }
            Ok(())
        }
    }
}
fn check_scoped(
    stmts: &[Stmt],
    env: &mut CheckScope,
    fns: &HashMap<String, usize>,
) -> Result<(), Diagnostic> {
    env.push();
    let result = check_stmts(stmts, env, fns);
    env.pop();
    result
}
fn check_stmts(
    stmts: &[Stmt],
    env: &mut CheckScope,
    fns: &HashMap<String, usize>,
) -> Result<(), Diagnostic> {
    for s in stmts {
        match s {
            Stmt::TypedLet(n, _, x, m) => {
                if env.contains_current(n) {
                    return Err(diag("", format!("'{n}' already declared in this scope")));
                }
                check_expr(x, env, fns)?;
                env.define(n.clone(), *m);
            }
            Stmt::While(c, b) => {
                check_expr(c, env, fns)?;
                check_scoped(b, env, fns)?;
            }
            Stmt::Let(n, x, m) => {
                if env.contains_current(n) {
                    return Err(diag("", format!("'{n}' already declared in this scope")));
                }
                check_expr(x, env, fns)?;
                env.define(n.clone(), *m);
            }
            Stmt::Assign(n, x) => {
                let m = env
                    .get(n)
                    .ok_or_else(|| diag("", format!("undefined variable '{n}'")))?;
                if !m {
                    return Err(diag(
                        "",
                        format!("cannot assign to immutable variable '{n}'"),
                    ));
                }
                check_expr(x, env, fns)?;
            }
            Stmt::Print(x) | Stmt::Expr(x) => check_expr(x, env, fns)?,
            Stmt::Return(x) => {
                if let Some(x) = x {
                    check_expr(x, env, fns)?
                }
            }
            Stmt::If(c, a, b) => {
                check_expr(c, env, fns)?;
                check_scoped(a, env, fns)?;
                check_scoped(b, env, fns)?;
            }
            Stmt::Block(b) => check_scoped(b, env, fns)?,
        }
    }
    Ok(())
}
fn validate(p: &Program) -> Result<(), Diagnostic> {
    let fns: HashMap<String, usize> = p
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.params.len()))
        .collect();
    for f in &p.functions {
        let mut env = CheckScope::new();
        for n in &f.params {
            env.define(n.clone(), true);
        }
        check_stmts(&f.body, &mut env, &fns)?;
    }
    if !fns.contains_key("main") {
        return Err(diag("", "program must define fn main()"));
    }
    if fns["main"] != 0 {
        return Err(diag("", "main must not take parameters"));
    }
    Ok(())
}
pub mod typecheck;
pub fn compile(src: &str) -> Result<Program, Diagnostic> {
    let p = Parser::new(Lexer::new(src).tokenize()?).parse()?;
    validate(&p)?;
    typecheck::check(&p)?;
    Ok(p)
}
pub fn run_file(path: &str) -> Result<(), Diagnostic> {
    let s = fs::read_to_string(path).map_err(|e| diag("", e.to_string()))?;
    Interpreter::new(compile(&s)?).run()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexer_parser() {
        assert!(compile("fn main(){print(1+2);}").is_ok())
    }
    #[test]
    fn parser_adds_eof_to_external_token_streams() {
        let tokens = Lexer::new("fn main() {}").tokenize().unwrap();
        let mut parser = Parser::new(tokens[..tokens.len() - 1].to_vec());
        assert!(parser.parse().is_ok());
        assert!(Parser::new(Vec::new()).parse().is_ok());
    }
    #[test]
    fn parser_rejects_excessive_expression_nesting_without_panicking() {
        let source = format!(
            "fn main(){{print({}1{});}}",
            "(".repeat(512),
            ")".repeat(512)
        );
        let diagnostic = compile(&source).unwrap_err();
        assert!(diagnostic.message.contains("parser nesting limit"));
    }
    #[test]
    fn parser_rejects_excessive_block_nesting_without_panicking() {
        let source = format!("fn main(){{{} }}", "{".repeat(512));
        let diagnostic = compile(&source).unwrap_err();
        assert!(diagnostic.message.contains("parser nesting limit"));
    }
    #[test]
    fn parses_and_executes_weave_syntax() {
        let program = compile(include_str!("../../../examples/weave-hello.avq")).unwrap();
        let (value, output) = Interpreter::new(program).run_captured().unwrap();
        assert_eq!(value, Value::Nil);
        assert_eq!(output, vec!["42"]);
    }
    #[test]
    fn executes() {
        let p = compile("fn main(){let x=2+3*4;print(x);}").unwrap();
        Interpreter::new(p).run().unwrap();
    }
    #[test]
    fn bad_char() {
        assert!(Lexer::new("@").tokenize().is_err())
    }
    #[test]
    fn immutable() {
        assert!(compile("fn main(){let x=1;x=2;}").is_err())
    }
    #[test]
    fn recursive_calls_are_bounded() {
        let p = compile("fn loop(){loop()} fn main(){loop()}").unwrap();
        assert!(Interpreter::new(p).run().is_err())
    }
    #[test]
    fn infinite_empty_loop_is_bounded() {
        let p = compile("fn main(){while true {}}").unwrap();
        assert!(Interpreter::new(p).run().is_err())
    }
    #[test]
    fn while_loop_runs() {
        let p = compile("fn main(){var x=0;while x<3{x=x+1;}print(x);}").unwrap();
        Interpreter::new(p).run().unwrap();
    }
    #[test]
    fn lexical_scopes_hide_block_locals() {
        assert!(compile("fn main(){{let hidden=1;}print(hidden);}").is_err());
        let p = compile("fn main(){var x=1;{let y=2;x=x+y;}print(x);}").unwrap();
        Interpreter::new(p).run().unwrap();
    }
    #[test]
    fn print_statement_preserves_all_arguments() {
        let p = compile("fn main(){print(1,2,3);}").unwrap();
        Interpreter::new(p).run().unwrap();
    }
    #[test]
    fn loop_iteration_locals_are_recreated() {
        let p =
            compile("fn main(){var i=0;while i<2{let per_iteration=i;i=i+1;}print(i);}").unwrap();
        Interpreter::new(p).run().unwrap();
    }
    #[test]
    fn division_zero() {
        let p = compile("fn main(){print(1/0);}").unwrap();
        assert!(Interpreter::new(p).run().is_err())
    }
    #[test]
    fn runtime_errors_retain_expression_locations() {
        let program = compile("fn main() {\n  print(1 / 0)\n}").unwrap();
        let diagnostic = Interpreter::new(program).run().unwrap_err();
        assert_eq!(diagnostic.span.line, 2);
        assert_eq!(diagnostic.span.column, 9);
    }
    #[test]
    fn catches_undefined_before_execution() {
        let diagnostic = compile("fn main() {\n  print(missing)\n}").unwrap_err();
        assert_eq!(diagnostic.span.line, 2);
        assert_eq!(diagnostic.span.column, 9);
    }
    #[test]
    fn catches_call_arity() {
        assert!(compile("fn add(a,b){return a+b} fn main(){print(add(1));}").is_err())
    }
}
