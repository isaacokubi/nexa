//! Static type inference and operator validation for the Averqun bootstrap AST.
use crate::{
    has_placeholder_span, Diagnostic, Expr, Function, Program, Span, Stmt, TokenKind, Value,
};
use std::collections::HashMap;

pub type FunctionId = u32;
pub type LocalId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct HirProgram {
    pub functions: Vec<HirFunction>,
    pub entry: FunctionId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirFunction {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<HirLocal>,
    pub return_type: Type,
    pub body: Vec<HirStatement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirLocal {
    pub id: LocalId,
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirExpressionKind {
    Literal(Value),
    Local(LocalId),
    Unary(TokenKind, Box<HirExpression>),
    Binary(Box<HirExpression>, TokenKind, Box<HirExpression>),
    Call(FunctionId, Vec<HirExpression>),
    BuiltinPrint(Vec<HirExpression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirStatement {
    Let(HirLocal, HirExpression),
    Assign(LocalId, Span, HirExpression),
    Print(HirExpression),
    Expression(HirExpression),
    Return(Option<HirExpression>),
    If(HirExpression, Vec<HirStatement>, Vec<HirStatement>),
    While(HirExpression, Vec<HirStatement>),
    Block(Vec<HirStatement>),
}

struct HirBuilder<'a> {
    function_ids: &'a HashMap<String, FunctionId>,
    scopes: Vec<HashMap<String, HirLocal>>,
    next_local: LocalId,
}

impl HirBuilder<'_> {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn scope_types(&self) -> Scope {
        Scope {
            layers: self
                .scopes
                .iter()
                .map(|layer| {
                    layer
                        .iter()
                        .map(|(name, local)| (name.clone(), local.ty))
                        .collect()
                })
                .collect(),
        }
    }

    fn resolve_local(&self, name: &str, span: Span) -> Result<HirLocal, Diagnostic> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .cloned()
            .ok_or_else(|| error_at(span, format!("unresolved HIR local '{name}'")))
    }

    fn define_local(
        &mut self,
        name: &str,
        ty: Type,
        mutable: bool,
        span: Span,
    ) -> Result<HirLocal, Diagnostic> {
        let Some(scope) = self.scopes.last_mut() else {
            return Err(error_at(span, "internal HIR error: no active local scope"));
        };
        if scope.contains_key(name) {
            return Err(error_at(span, format!("duplicate HIR local '{name}'")));
        }
        let local = HirLocal {
            id: self.next_local,
            name: name.to_owned(),
            ty,
            mutable,
            span,
        };
        self.next_local = self
            .next_local
            .checked_add(1)
            .ok_or_else(|| error_at(span, "HIR local identifier space exhausted"))?;
        scope.insert(name.to_owned(), local.clone());
        Ok(local)
    }

    fn expression(
        &self,
        expression: &Expr,
        functions: &HashMap<String, Signature>,
    ) -> Result<HirExpression, Diagnostic> {
        let span = expression.span();
        let ty = infer(expression, &self.scope_types(), functions)?;
        let kind = match expression {
            Expr::Lit(value, _) => HirExpressionKind::Literal(value.clone()),
            Expr::Var(name, _) => HirExpressionKind::Local(self.resolve_local(name, span)?.id),
            Expr::Unary(operator, operand, _) => HirExpressionKind::Unary(
                operator.clone(),
                Box::new(self.expression(operand, functions)?),
            ),
            Expr::Binary(left, operator, right, _) => HirExpressionKind::Binary(
                Box::new(self.expression(left, functions)?),
                operator.clone(),
                Box::new(self.expression(right, functions)?),
            ),
            Expr::Call(name, arguments, _) if name == "print" => HirExpressionKind::BuiltinPrint(
                arguments
                    .iter()
                    .map(|argument| self.expression(argument, functions))
                    .collect::<Result<_, _>>()?,
            ),
            Expr::Call(name, arguments, _) => {
                let id =
                    self.function_ids.get(name).copied().ok_or_else(|| {
                        error_at(span, format!("unresolved HIR function '{name}'"))
                    })?;
                HirExpressionKind::Call(
                    id,
                    arguments
                        .iter()
                        .map(|argument| self.expression(argument, functions))
                        .collect::<Result<_, _>>()?,
                )
            }
        };
        Ok(HirExpression { kind, ty, span })
    }

    fn statements(
        &mut self,
        statements: &[Stmt],
        functions: &HashMap<String, Signature>,
    ) -> Result<Vec<HirStatement>, Diagnostic> {
        let mut output = Vec::with_capacity(statements.len());
        for statement in statements {
            let lowered = match statement {
                Stmt::Let(name, value, mutable, span) => {
                    let value_hir = self.expression(value, functions)?;
                    let local = self.define_local(name, value_hir.ty, *mutable, *span)?;
                    HirStatement::Let(local, value_hir)
                }
                Stmt::TypedLet(name, annotation, value, mutable, span) => {
                    let value_hir = self.expression(value, functions)?;
                    let ty = declared_type(annotation)?;
                    let local = self.define_local(name, ty, *mutable, *span)?;
                    HirStatement::Let(local, value_hir)
                }
                Stmt::Assign(name, value, span) => {
                    let local = self.resolve_local(name, *span)?;
                    HirStatement::Assign(local.id, *span, self.expression(value, functions)?)
                }
                Stmt::Print(value) => HirStatement::Print(self.expression(value, functions)?),
                Stmt::Expr(value) => HirStatement::Expression(self.expression(value, functions)?),
                Stmt::Return(value) => HirStatement::Return(
                    value
                        .as_ref()
                        .map(|expression| self.expression(expression, functions))
                        .transpose()?,
                ),
                Stmt::If(condition, yes, no) => {
                    let condition = self.expression(condition, functions)?;
                    self.push_scope();
                    let yes = self.statements(yes, functions)?;
                    self.pop_scope();
                    self.push_scope();
                    let no = self.statements(no, functions)?;
                    self.pop_scope();
                    HirStatement::If(condition, yes, no)
                }
                Stmt::While(condition, body) => {
                    let condition = self.expression(condition, functions)?;
                    self.push_scope();
                    let body = self.statements(body, functions)?;
                    self.pop_scope();
                    HirStatement::While(condition, body)
                }
                Stmt::Block(body) => {
                    self.push_scope();
                    let body = self.statements(body, functions)?;
                    self.pop_scope();
                    HirStatement::Block(body)
                }
            };
            output.push(lowered);
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Number,
    String,
    Bool,
    Nil,
    Unknown,
}
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Number => "Number",
                Self::String => "String",
                Self::Bool => "Bool",
                Self::Nil => "Nil",
                Self::Unknown => "Unknown",
            }
        )
    }
}
fn error(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        message: message.into(),
        span: Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        },
    }
}
fn error_at(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        message: message.into(),
        span,
    }
}
#[derive(Clone, Default)]
struct Scope {
    layers: Vec<HashMap<String, Type>>,
}
impl Scope {
    fn new() -> Self {
        Self {
            layers: vec![HashMap::new()],
        }
    }
    fn get(&self, n: &str) -> Option<Type> {
        self.layers.iter().rev().find_map(|s| s.get(n).copied())
    }
    fn contains_current(&self, n: &str) -> bool {
        self.layers.last().is_some_and(|s| s.contains_key(n))
    }
    fn define(&mut self, n: String, t: Type) {
        self.layers
            .last_mut()
            .expect("scope stack is never empty")
            .insert(n, t);
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
#[derive(Clone)]
struct Signature {
    params: Vec<Type>,
    inferred_ret: Type,
}

fn merge_return_type(current: Option<Type>, next: Type) -> Result<Option<Type>, Diagnostic> {
    match current {
        None => Ok(Some(next)),
        Some(existing) if existing == next => Ok(Some(existing)),
        Some(existing) if existing == Type::Unknown || next == Type::Unknown => {
            Ok(Some(Type::Unknown))
        }
        Some(existing) => Err(error(format!(
            "function returns incompatible types {existing} and {next}"
        ))),
    }
}

fn collect_returns(
    body: &[Stmt],
    scope: &mut Scope,
    functions: &HashMap<String, Signature>,
) -> Result<Option<Type>, Diagnostic> {
    let mut result = None;
    for statement in body {
        let found = match statement {
            Stmt::Return(value) => Some(match value {
                Some(value) => infer(value, scope, functions)?,
                None => Type::Nil,
            }),
            Stmt::Let(name, value, _, _) => {
                let ty = infer(value, scope, functions)?;
                scope.define(name.clone(), ty);
                None
            }
            Stmt::TypedLet(name, annotation, value, _, declaration_span) => {
                let ty = declared_type(annotation).map_err(|mut diagnostic| {
                    diagnostic.span = *declaration_span;
                    diagnostic
                })?;
                let actual = infer(value, scope, functions)?;
                if actual != ty && actual != Type::Unknown {
                    return Err(error_at(
                        value.span(),
                        format!("cannot initialize {annotation} variable '{name}' with {actual}"),
                    ));
                }
                scope.define(name.clone(), ty);
                None
            }
            Stmt::If(condition, yes, no) => {
                let condition_type = infer(condition, scope, functions)?;
                if condition_type != Type::Bool && condition_type != Type::Unknown {
                    return Err(error_at(
                        condition.span(),
                        format!("if condition must be Bool, found {condition_type}"),
                    ));
                }
                let mut branch_scope = scope.clone();
                branch_scope.push();
                let yes_returns = collect_returns(yes, &mut branch_scope, functions)?;
                let mut branch_scope = scope.clone();
                branch_scope.push();
                let no_returns = collect_returns(no, &mut branch_scope, functions)?;
                let combined = match (yes_returns, no_returns) {
                    (Some(left), Some(right)) => merge_return_type(Some(left), right)?,
                    (left, right) => left.or(right),
                };
                if let Some(ty) = combined {
                    result = merge_return_type(result, ty)?;
                }
                None
            }
            Stmt::While(condition, loop_body) => {
                let condition_type = infer(condition, scope, functions)?;
                if condition_type != Type::Bool && condition_type != Type::Unknown {
                    return Err(error_at(
                        condition.span(),
                        format!("while condition must be Bool, found {condition_type}"),
                    ));
                }
                let mut nested = scope.clone();
                nested.push();
                if let Some(ty) = collect_returns(loop_body, &mut nested, functions)? {
                    result = merge_return_type(result, ty)?;
                }
                None
            }
            Stmt::Block(loop_body) => {
                let mut nested = scope.clone();
                nested.push();
                if let Some(ty) = collect_returns(loop_body, &mut nested, functions)? {
                    result = merge_return_type(result, ty)?;
                }
                None
            }
            Stmt::Assign(_, value, _) | Stmt::Print(value) | Stmt::Expr(value) => {
                infer(value, scope, functions)?;
                None
            }
        };
        if let Some(ty) = found {
            result = merge_return_type(result, ty)?;
        }
    }
    Ok(result)
}

fn infer(e: &Expr, s: &Scope, fs: &HashMap<String, Signature>) -> Result<Type, Diagnostic> {
    infer_inner(e, s, fs).map_err(|mut diagnostic| {
        if has_placeholder_span(diagnostic.span) {
            diagnostic.span = e.span();
        }
        diagnostic
    })
}

fn infer_inner(e: &Expr, s: &Scope, fs: &HashMap<String, Signature>) -> Result<Type, Diagnostic> {
    match e {
        Expr::Lit(v, _) => Ok(match v {
            Value::Number(_) => Type::Number,
            Value::String(_) => Type::String,
            Value::Bool(_) => Type::Bool,
            Value::Nil => Type::Nil,
        }),
        Expr::Var(n, _) => s
            .get(n)
            .ok_or_else(|| error(format!("undefined variable '{n}'"))),
        Expr::Unary(op, x, _) => {
            let t = infer(x, s, fs)?;
            match op {
                TokenKind::Minus if t == Type::Number => Ok(Type::Number),
                TokenKind::Bang if t == Type::Bool => Ok(Type::Bool),
                TokenKind::Minus => Err(error(format!("unary '-' requires Number, found {t}"))),
                TokenKind::Bang => Err(error(format!("'!' requires Bool, found {t}"))),
                _ => Ok(Type::Unknown),
            }
        }
        Expr::Binary(a, op, b, _) => {
            let l = infer(a, s, fs)?;
            let r = infer(b, s, fs)?;
            use TokenKind::*;
            match op {
                Plus if l == Type::Unknown || r == Type::Unknown => Ok(Type::Unknown),
                Plus if l == Type::Number && r == Type::Number => Ok(Type::Number),
                Plus if l == Type::String && r == Type::String => Ok(Type::String),
                Plus => Err(error(format!(
                    "'+' requires two Numbers or two Strings; got {l} and {r}"
                ))),
                Minus | Star | Slash | Percent if l == Type::Unknown || r == Type::Unknown => {
                    Ok(Type::Unknown)
                }
                Minus | Star | Slash | Percent if l == Type::Number && r == Type::Number => {
                    Ok(Type::Number)
                }
                Minus | Star | Slash | Percent => Err(error(format!(
                    "arithmetic requires Number operands; got {l} and {r}"
                ))),
                Lt | LtEq | Gt | GtEq if l == Type::Unknown || r == Type::Unknown => Ok(Type::Bool),
                Lt | LtEq | Gt | GtEq if l == Type::Number && r == Type::Number => Ok(Type::Bool),
                Lt | LtEq | Gt | GtEq => Err(error(format!(
                    "comparison requires Number operands; got {l} and {r}"
                ))),
                AndAnd | OrOr if l == Type::Unknown || r == Type::Unknown => Ok(Type::Bool),
                AndAnd | OrOr if l == Type::Bool && r == Type::Bool => Ok(Type::Bool),
                AndAnd | OrOr => Err(error(format!(
                    "logical operators require Bool operands; got {l} and {r}"
                ))),
                EqEq | BangEq if l == Type::Unknown || r == Type::Unknown || l == r => {
                    Ok(Type::Bool)
                }
                EqEq | BangEq => Err(error(format!(
                    "equality operands must have same type; got {l} and {r}"
                ))),
                _ => Ok(Type::Unknown),
            }
        }
        Expr::Call(n, args, span) => {
            if n == "print" {
                for a in args {
                    infer(a, s, fs)?;
                }
                return Ok(Type::Nil);
            }
            let sig = fs.get(n).ok_or_else(|| Diagnostic {
                message: format!("undefined function '{n}'"),
                span: *span,
            })?;
            if sig.params.len() != args.len() {
                return Err(error(format!(
                    "function '{n}' expects {} arguments, got {}",
                    sig.params.len(),
                    args.len()
                )));
            }
            for (a, expected) in args.iter().zip(&sig.params) {
                let actual = infer(a, s, fs)?;
                if *expected != Type::Unknown && actual != *expected && actual != Type::Unknown {
                    return Err(error(format!(
                        "argument to '{n}' expects {expected}, found {actual}"
                    )));
                }
            }
            Ok(sig.inferred_ret)
        }
    }
}

fn validate_expression(
    expression: &Expr,
    scope: &Scope,
    functions: &HashMap<String, Signature>,
) -> Result<Type, Diagnostic> {
    match expression {
        Expr::Lit(_, _) | Expr::Var(_, _) => {}
        Expr::Unary(_, value, _) => {
            validate_expression(value, scope, functions)?;
        }
        Expr::Binary(left, _, right, _) => {
            validate_expression(left, scope, functions)?;
            validate_expression(right, scope, functions)?;
        }
        Expr::Call(_, arguments, _) => {
            for argument in arguments {
                validate_expression(argument, scope, functions)?;
            }
        }
    }
    let ty = infer(expression, scope, functions)?;
    if ty == Type::Unknown {
        return Err(error_at(
            expression.span(),
            "cannot infer expression type; add an explicit annotation",
        ));
    }
    Ok(ty)
}

fn collect_call_constraints_expr(
    expression: &Expr,
    scope: &Scope,
    functions: &HashMap<String, Signature>,
    constraints: &mut HashMap<(String, usize), Type>,
) -> Result<(), Diagnostic> {
    match expression {
        Expr::Lit(_, _) | Expr::Var(_, _) => {}
        Expr::Unary(_, value, _) => {
            collect_call_constraints_expr(value, scope, functions, constraints)?;
        }
        Expr::Binary(left, _, right, _) => {
            collect_call_constraints_expr(left, scope, functions, constraints)?;
            collect_call_constraints_expr(right, scope, functions, constraints)?;
        }
        Expr::Call(name, arguments, span) => {
            for argument in arguments {
                collect_call_constraints_expr(argument, scope, functions, constraints)?;
            }
            if name == "print" {
                return Ok(());
            }
            let signature = functions.get(name).ok_or_else(|| Diagnostic {
                message: format!("undefined function '{name}'"),
                span: *span,
            })?;
            if signature.params.len() != arguments.len() {
                return Err(error_at(
                    *span,
                    format!(
                        "function '{name}' expects {} arguments, got {}",
                        signature.params.len(),
                        arguments.len()
                    ),
                ));
            }
            for (index, argument) in arguments.iter().enumerate() {
                let actual = infer(argument, scope, functions)?;
                let declared = signature.params[index];
                if actual != Type::Unknown {
                    if declared != Type::Unknown && actual != declared {
                        return Err(error_at(
                            *span,
                            format!("argument to '{name}' expects {declared}, found {actual}"),
                        ));
                    }
                    let key = (name.clone(), index);
                    if let Some(previous) = constraints.get(&key) {
                        if *previous != actual {
                            return Err(error_at(*span, format!(
                                "calls to '{name}' pass incompatible types {previous} and {actual} for parameter {}",
                                index + 1
                            )));
                        }
                    } else {
                        constraints.insert(key, actual);
                    }
                }
            }
        }
    }
    Ok(())
}

fn collect_call_constraints(
    statements: &[Stmt],
    scope: &mut Scope,
    functions: &HashMap<String, Signature>,
    constraints: &mut HashMap<(String, usize), Type>,
) -> Result<(), Diagnostic> {
    for statement in statements {
        match statement {
            Stmt::Let(name, value, _, _) => {
                collect_call_constraints_expr(value, scope, functions, constraints)?;
                scope.define(name.clone(), infer(value, scope, functions)?);
            }
            Stmt::TypedLet(name, annotation, value, _, declaration_span) => {
                collect_call_constraints_expr(value, scope, functions, constraints)?;
                let ty = declared_type(annotation).map_err(|mut diagnostic| {
                    diagnostic.span = *declaration_span;
                    diagnostic
                })?;
                scope.define(name.clone(), ty);
            }
            Stmt::Assign(_, value, _) | Stmt::Print(value) | Stmt::Expr(value) => {
                collect_call_constraints_expr(value, scope, functions, constraints)?;
            }
            Stmt::Return(value) => {
                if let Some(value) = value {
                    collect_call_constraints_expr(value, scope, functions, constraints)?;
                }
            }
            Stmt::If(condition, yes, no) => {
                collect_call_constraints_expr(condition, scope, functions, constraints)?;
                let mut yes_scope = scope.clone();
                yes_scope.push();
                collect_call_constraints(yes, &mut yes_scope, functions, constraints)?;
                let mut no_scope = scope.clone();
                no_scope.push();
                collect_call_constraints(no, &mut no_scope, functions, constraints)?;
            }
            Stmt::While(condition, body) => {
                collect_call_constraints_expr(condition, scope, functions, constraints)?;
                let mut loop_scope = scope.clone();
                loop_scope.push();
                collect_call_constraints(body, &mut loop_scope, functions, constraints)?;
            }
            Stmt::Block(body) => {
                let mut nested = scope.clone();
                nested.push();
                collect_call_constraints(body, &mut nested, functions, constraints)?;
            }
        }
    }
    Ok(())
}

fn declared_type(name: &str) -> Result<Type, Diagnostic> {
    match name {
        "Number" => Ok(Type::Number),
        "String" => Ok(Type::String),
        "Bool" => Ok(Type::Bool),
        "Nil" => Ok(Type::Nil),
        _ => Err(error(format!("unknown type '{name}'"))),
    }
}
fn stmts(
    b: &[Stmt],
    s: &mut Scope,
    fs: &HashMap<String, Signature>,
    ret: Option<Type>,
) -> Result<(), Diagnostic> {
    for x in b {
        match x {
            Stmt::TypedLet(n, name, e, _, declaration_span) => {
                let expected = declared_type(name).map_err(|mut diagnostic| {
                    diagnostic.span = *declaration_span;
                    diagnostic
                })?;
                let actual = validate_expression(e, s, fs)?;
                if actual != expected {
                    return Err(error_at(
                        e.span(),
                        format!("cannot initialize {name} variable '{n}' with {actual}"),
                    ));
                }
                if s.contains_current(n) {
                    return Err(error_at(
                        *declaration_span,
                        format!("'{n}' already declared in this scope"),
                    ));
                }
                s.define(n.clone(), expected);
            }
            Stmt::While(c, b) => {
                let t = validate_expression(c, s, fs)?;
                if t != Type::Bool {
                    return Err(error_at(
                        c.span(),
                        format!("while condition must be Bool, found {t}"),
                    ));
                }
                scoped_stmts(b, s, fs, ret)?;
            }
            Stmt::Let(n, e, _, declaration_span) => {
                let t = validate_expression(e, s, fs)?;
                if t == Type::Unknown {
                    return Err(error_at(
                        e.span(),
                        format!("cannot infer type of local '{n}'"),
                    ));
                }
                if s.contains_current(n) {
                    return Err(error_at(
                        *declaration_span,
                        format!("'{n}' already declared in this scope"),
                    ));
                }
                s.define(n.clone(), t);
            }
            Stmt::Assign(n, e, assignment_span) => {
                let old = s.get(n).ok_or_else(|| {
                    error_at(*assignment_span, format!("undefined variable '{n}'"))
                })?;
                let new = validate_expression(e, s, fs)?;
                if old != new {
                    return Err(error_at(
                        e.span(),
                        format!("cannot assign {new} to '{n}' of type {old}"),
                    ));
                }
            }
            Stmt::Print(e) | Stmt::Expr(e) => {
                validate_expression(e, s, fs)?;
            }
            Stmt::Return(e) => {
                let actual = if let Some(e) = e {
                    validate_expression(e, s, fs)?
                } else {
                    Type::Nil
                };
                if let Some(expected) = ret {
                    if actual != expected {
                        return Err(error_at(
                            e.as_ref().map(Expr::span).unwrap_or_default(),
                            format!("return type mismatch: expected {expected}, found {actual}"),
                        ));
                    }
                }
            }
            Stmt::If(c, a, b) => {
                let t = validate_expression(c, s, fs)?;
                if t != Type::Bool {
                    return Err(error_at(
                        c.span(),
                        format!("if condition must be Bool, found {t}"),
                    ));
                }
                scoped_stmts(a, s, fs, ret)?;
                scoped_stmts(b, s, fs, ret)?;
            }
            Stmt::Block(b) => scoped_stmts(b, s, fs, ret)?,
        }
    }
    Ok(())
}
fn scoped_stmts(
    b: &[Stmt],
    s: &mut Scope,
    fs: &HashMap<String, Signature>,
    ret: Option<Type>,
) -> Result<(), Diagnostic> {
    s.push();
    let result = stmts(b, s, fs, ret);
    s.pop();
    result
}
// Conservative control-flow analysis: true means execution may reach the
// end of this statement list without returning. Loops are treated as
// potentially fall-through because this AST has no break/termination proof.
fn can_fall_through(body: &[Stmt]) -> bool {
    for stmt in body {
        let falls_through = match stmt {
            Stmt::Return(_) => false,
            Stmt::Block(inner) => can_fall_through(inner),
            Stmt::If(_, yes, no) => can_fall_through(yes) || no.is_empty() || can_fall_through(no),
            Stmt::While(_, _) => true,
            _ => true,
        };
        if !falls_through {
            return false;
        }
    }
    true
}
fn check_signatures(p: &Program) -> Result<HashMap<String, Signature>, Diagnostic> {
    let mut fs: HashMap<String, Signature> = p
        .functions
        .iter()
        .map(|f| {
            Ok((
                f.name.clone(),
                Signature {
                    params: f
                        .param_types
                        .iter()
                        .map(|annotation| {
                            annotation
                                .as_deref()
                                .map(declared_type)
                                .transpose()
                                .map(|ty| ty.unwrap_or(Type::Unknown))
                        })
                        .collect::<Result<_, _>>()?,
                    inferred_ret: f
                        .return_type
                        .as_deref()
                        .map(declared_type)
                        .transpose()?
                        .unwrap_or(Type::Unknown),
                },
            ))
        })
        .collect::<Result<_, Diagnostic>>()?;

    // Infer unannotated parameters from known call arguments and resolve
    // unannotated returns to a fixed point. Recursive cycles without an
    // annotation remain Unknown and are rejected rather than weakening checks.
    for _ in 0..=(2 * p.functions.len()) {
        let mut changed = false;
        let mut constraints = HashMap::new();
        for function in &p.functions {
            let mut scope = Scope::new();
            for (index, name) in function.params.iter().enumerate() {
                scope.define(name.clone(), fs[&function.name].params[index]);
            }
            collect_call_constraints(&function.body, &mut scope, &fs, &mut constraints)?;
        }
        for ((function, index), ty) in constraints {
            let parameter = &mut fs
                .get_mut(&function)
                .expect("resolved call target has a signature")
                .params[index];
            if *parameter == Type::Unknown {
                *parameter = ty;
                changed = true;
            } else if *parameter != ty {
                return Err(error(format!(
                    "calls to '{function}' pass incompatible types {} and {ty} for parameter {}",
                    *parameter,
                    index + 1
                )));
            }
        }
        for function in &p.functions {
            if function.return_type.is_some() {
                continue;
            }
            let mut scope = Scope::new();
            for (index, name) in function.params.iter().enumerate() {
                scope.define(name.clone(), fs[&function.name].params[index]);
            }
            let inferred = collect_returns(&function.body, &mut scope, &fs)?.unwrap_or(Type::Nil);
            if inferred != Type::Unknown && fs[&function.name].inferred_ret != inferred {
                fs.get_mut(&function.name)
                    .expect("function signature exists")
                    .inferred_ret = inferred;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for Function {
        name,
        params,
        param_types: _,
        return_type: _,
        body,
        span,
        ..
    } in &p.functions
    {
        let expected = Some(fs[name].inferred_ret);
        let mut s = Scope::new();
        for (index, parameter) in params.iter().enumerate() {
            let ty = fs[name].params[index];
            if ty == Type::Unknown {
                return Err(error_at(*span, format!(
                    "in function '{name}': cannot infer type of parameter '{parameter}'; add an annotation or call the function with a typed value"
                )));
            }
            s.define(parameter.clone(), ty);
        }
        if expected == Some(Type::Unknown) {
            return Err(error_at(
                *span,
                format!(
                "in function '{name}': cannot infer return type; add an explicit return annotation"
            ),
            ));
        }
        stmts(body, &mut s, &fs, expected).map_err(|mut diagnostic| {
            diagnostic.message = format!("in function '{name}': {}", diagnostic.message);
            if has_placeholder_span(diagnostic.span) {
                diagnostic.span = *span;
            }
            diagnostic
        })?;
        if expected.is_some_and(|t| t != Type::Nil) && can_fall_through(body) {
            return Err(error_at(
                *span,
                format!("in function '{name}': not all control-flow paths return a value"),
            ));
        }
    }
    Ok(fs)
}

pub fn check_typed(p: &Program) -> Result<HirProgram, Diagnostic> {
    let fs = check_signatures(p)?;
    let function_ids: HashMap<String, FunctionId> = p
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.name.clone(), index as FunctionId))
        .collect();
    let entry = function_ids
        .get("main")
        .copied()
        .ok_or_else(|| error("missing HIR entry function 'main'"))?;
    let mut functions = Vec::with_capacity(p.functions.len());
    for (index, function) in p.functions.iter().enumerate() {
        let mut builder = HirBuilder {
            function_ids: &function_ids,
            scopes: vec![HashMap::new()],
            next_local: 0,
        };
        let mut params = Vec::with_capacity(function.params.len());
        for (parameter_index, name) in function.params.iter().enumerate() {
            params.push(builder.define_local(
                name,
                fs[&function.name].params[parameter_index],
                false,
                function.param_spans[parameter_index],
            )?);
        }
        let body = builder.statements(&function.body, &fs)?;
        functions.push(HirFunction {
            id: index as FunctionId,
            name: function.name.clone(),
            params,
            return_type: fs[&function.name].inferred_ret,
            body,
            span: function.span,
        });
    }
    Ok(HirProgram { functions, entry })
}

pub fn check(p: &Program) -> Result<(), Diagnostic> {
    check_signatures(p).map(|_| ())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};
    fn c(src: &str) -> Result<(), Diagnostic> {
        check(&Parser::new(Lexer::new(src).tokenize()?).parse()?)
    }
    #[test]
    fn typed_hir_resolves_function_and_local_ids_and_expression_types() {
        let hir = crate::compile_to_hir(
            "fn twice(x: Number) -> Number { let y = x * 2; return y }\nfn main() { var n = twice(21); if true { let t = n; print(t); } }",
        )
        .unwrap();
        assert_eq!(hir.entry, 1);
        assert_eq!(hir.functions[0].return_type, Type::Number);
        assert_eq!(hir.functions[0].params[0].id, 0);
        assert_eq!(hir.functions[0].params[0].span.column, 10);
        assert!(hir.functions[0].span.end > hir.functions[0].span.start);
        let HirStatement::Let(y, HirExpression { ty, .. }) = &hir.functions[0].body[0] else {
            panic!("expected a typed local declaration");
        };
        assert_eq!(y.id, 1);
        assert_eq!(*ty, Type::Number);
        let HirStatement::Let(_, initializer) = &hir.functions[1].body[0] else {
            panic!("expected main local declaration");
        };
        assert_eq!(initializer.ty, Type::Number);
        assert!(matches!(initializer.kind, HirExpressionKind::Call(0, _)));
        let HirStatement::If(_, branch, _) = &hir.functions[1].body[1] else {
            panic!("expected branch");
        };
        let HirStatement::Let(branch_local, _) = &branch[0] else {
            panic!("expected branch local declaration");
        };
        assert_eq!(branch_local.id, 1);
        assert!(!branch_local.mutable);
        assert!(branch_local.span.line > 0);
    }

    #[test]
    fn typed_hir_is_not_built_when_typechecking_fails() {
        let error =
            crate::compile_to_hir("fn main() { let answer: Number = \"bad\" }").unwrap_err();
        assert!(error.message.contains("cannot initialize"));
    }
    #[test]
    fn rejects_string_math() {
        assert!(c("fn main(){print(\"x\"-1);}").is_err())
    }
    #[test]
    fn rejects_non_bool_condition() {
        assert!(c("fn main(){if 1 {print(2);}}").is_err())
    }
    #[test]
    fn validates_function_return_types() {
        assert!(c("fn answer()->Number{return 42;} fn main(){print(answer());}").is_ok());
        assert!(c("fn answer()->Number{return \"wrong\";} fn main(){answer();}").is_err());
        assert!(c("fn answer()->Number{return;} fn main(){answer();}").is_err())
    }
    #[test]
    fn validates_typed_function_parameters() {
        assert!(c(
            "fn add(a: Number,b: Number){let total: Number=a+b;print(total);} fn main(){add(1,2);}"
        )
        .is_ok());
        assert!(c("fn add(a: Number){print(a);} fn main(){add(\"bad\");}").is_err())
    }
    #[test]
    fn validates_declared_local_types() {
        assert!(c("fn main(){let x: Number=1;let y: String=\"ok\";print(x);print(y);}").is_ok());
        assert!(c("fn main(){let x: Number=\"wrong\";}").is_err());
        assert!(c("fn main(){let x: Unknown=1;}").is_err())
    }
    #[test]
    fn type_errors_retain_expression_locations() {
        let diagnostic = c("fn main() {\n  let n: Number = \"bad\"\n}").unwrap_err();
        assert_eq!(diagnostic.span.line, 2);
        assert_eq!(diagnostic.span.column, 19);
    }
    #[test]
    fn declaration_and_assignment_type_errors_retain_statement_locations() {
        let duplicate = c("fn main() {\n  let item = 1\n  let item = 2\n}").unwrap_err();
        assert_eq!((duplicate.span.line, duplicate.span.column), (3, 7));

        let undefined = c("fn main() {\n  missing = 1\n}").unwrap_err();
        assert_eq!((undefined.span.line, undefined.span.column), (2, 3));

        let immutable = crate::compile("fn main() {\n  let fixed = 1\n  fixed = 2\n}").unwrap_err();
        assert_eq!((immutable.span.line, immutable.span.column), (3, 3));

        let unknown_type = c("fn main() {\n  let value: Missing = 1\n}").unwrap_err();
        assert_eq!((unknown_type.span.line, unknown_type.span.column), (2, 7));
    }
    #[test]
    fn checks_function_return_type_at_call_sites() {
        assert!(c("fn answer()->Number{return 42;} fn main(){let n: Number=answer();}").is_ok());
        assert!(c("fn answer()->Number{return 42;} fn main(){let s: String=answer();}").is_err())
    }
    #[test]
    fn infers_unannotated_return_types_and_checks_call_sites() {
        assert!(c("fn answer(){return 42;} fn main(){let n: Number=answer();}").is_ok());
        assert!(c("fn answer(){return 42;} fn main(){let s: String=answer();}").is_err());
        assert!(c("fn wrapped(){return answer();} fn answer(){return 42;} fn main(){let n:Number=wrapped();}").is_ok());
        assert!(c(
            "fn answer(){if true{return 42;}else{return 7;}} fn main(){let n: Number=answer();}"
        )
        .is_ok());
        assert!(c("fn answer(){if true{return 42;}else{return \"wrong\";}} fn main(){}").is_err());
    }

    #[test]
    fn infers_unannotated_parameters_from_calls() {
        assert!(c("fn double(value){return value*2;} fn main(){let n:Number=double(3);}").is_ok());
        assert!(
            c("fn identity(value){return value;} fn main(){let n:Number=identity(3);}").is_ok()
        );
        assert!(c("fn relay(value){return answer(value);} fn answer(x:Number)->Number{return x;} fn main(){let n:Number=relay(4);}").is_ok());
        assert!(
            c("fn consume(value){print(value);} fn main(){consume(1);consume(\"one\");}").is_err()
        );
        assert!(c("fn never_called(value){print(value);} fn main(){}").is_err());
    }

    #[test]
    fn rejects_unresolved_inferred_return_types() {
        assert!(c("fn identity(value){return value;} fn main(){}").is_err());
        assert!(c("fn identity(value: Number)->Number{return value;} fn main(){let n:Number=identity(1);}").is_ok());
    }
    #[test]
    fn requires_all_paths_to_return_annotated_values() {
        assert!(c("fn choose(flag: Bool)->Number{if flag{return 1;}else{return 2;}} fn main(){print(choose(true));}").is_ok());
        assert!(c(
            "fn choose(flag: Bool)->Number{if flag{return 1;}} fn main(){print(choose(true));}"
        )
        .is_err());
        assert!(c("fn choose(flag: Bool)->Number{if flag{{return 1;}}else{{return 2;}}} fn main(){print(choose(true));}").is_ok());
        assert!(c("fn choose(flag: Bool)->Number{if flag{return 1;}return 2;} fn main(){print(choose(true));}").is_ok());
        assert!(c("fn choose(flag: Bool)->Number{if flag{return 1;}else{print(2);}} fn main(){print(choose(true));}").is_err());
        assert!(c("fn value()->Number{while true{return 1;}} fn main(){print(value());}").is_err());
    }
    #[test]
    fn lexical_shadowing_is_typechecked() {
        assert!(c("fn main(){let value: Number=1;{let value: String=\"inner\";print(value);}print(value);}").is_ok());
        assert!(c("fn main(){let value=1;let value=2;}").is_err());
        assert!(c("fn main(){var value: Number=1;{value=value+1;}print(value);}").is_ok());
    }
    #[test]
    fn accepts_typed_ops() {
        assert!(c("fn main(){let x=1+2;let y=\"a\"+\"b\";print(x);print(y);}").is_ok())
    }
}
