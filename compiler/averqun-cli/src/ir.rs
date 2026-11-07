//! Stack-based intermediate representation for the Averqun bootstrap compiler.
//!
//! This is a stable boundary between the source AST and future optimization /
//! native backends. Instructions are deliberately target-independent.

use crate::typecheck::{
    HirExpression, HirExpressionKind, HirFunction, HirLocal, HirProgram, HirStatement, LocalId,
    Type,
};
use crate::{has_placeholder_span, Diagnostic, Program, Span, TokenKind, Value};
#[cfg(test)]
use crate::{Expr, Function, Stmt};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Constant(Value),
    LoadLocal(u32),
    StoreLocal(u32),
    Unary(TokenKind),
    Binary(TokenKind),
    Call { function: String, arguments: usize },
    Print { arguments: usize },
    Pop,
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfFalseKeep(usize),
    JumpIfTrueKeep(usize),
    Return,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionIr {
    pub name: String,
    pub parameter_count: usize,
    pub local_count: u32,
    pub instructions: Vec<Instruction>,
    pub instruction_spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModuleIr {
    pub functions: Vec<FunctionIr>,
}

struct HirLowerer<'a> {
    function: &'a HirFunction,
    functions: &'a [HirFunction],
    slots: HashMap<LocalId, u32>,
    local_types: HashMap<LocalId, Type>,
    local_mutability: HashMap<LocalId, bool>,
    visible: Vec<std::collections::HashSet<LocalId>>,
    code: Vec<Instruction>,
    spans: Vec<Span>,
    current_span: Span,
}

impl<'a> HirLowerer<'a> {
    fn new(function: &'a HirFunction, functions: &'a [HirFunction]) -> Result<Self, Diagnostic> {
        let mut lowerer = Self {
            function,
            functions,
            slots: HashMap::new(),
            local_types: HashMap::new(),
            local_mutability: HashMap::new(),
            visible: vec![std::collections::HashSet::new()],
            code: Vec::new(),
            spans: Vec::new(),
            current_span: function.span,
        };
        for parameter in &function.params {
            lowerer.define(parameter)?;
        }
        Ok(lowerer)
    }

    fn define(&mut self, local: &HirLocal) -> Result<u32, Diagnostic> {
        if local.ty == Type::Unknown {
            return Err(error("unresolved type on HIR local", local.span));
        }
        let Some(scope) = self.visible.last_mut() else {
            return Err(error("HIR lowerer has no active local scope", local.span));
        };
        if !scope.insert(local.id) {
            return Err(error("duplicate local ID in HIR scope", local.span));
        }
        if self.slots.contains_key(&local.id) {
            return Err(error("HIR local ID is reused", local.span));
        }
        let slot = u32::try_from(self.slots.len())
            .map_err(|_| error("IR local slot space exhausted", local.span))?;
        self.slots.insert(local.id, slot);
        self.local_types.insert(local.id, local.ty);
        self.local_mutability.insert(local.id, local.mutable);
        Ok(slot)
    }

    fn resolve(&self, local: LocalId, span: Span) -> Result<u32, Diagnostic> {
        if !self
            .visible
            .iter()
            .rev()
            .any(|scope| scope.contains(&local))
        {
            return Err(error(format!("unresolved HIR local ID {local}"), span));
        }
        self.slots
            .get(&local)
            .copied()
            .ok_or_else(|| error(format!("missing IR slot for HIR local {local}"), span))
    }

    fn emit(&mut self, instruction: Instruction) {
        self.code.push(instruction);
        self.spans.push(self.current_span);
    }

    fn emit_at(&mut self, instruction: Instruction, span: Span) {
        let previous = self.current_span;
        self.current_span = span;
        self.emit(instruction);
        self.current_span = previous;
    }

    fn patch(&mut self, at: usize, target: usize, span: Span) -> Result<(), Diagnostic> {
        let instruction = self
            .code
            .get_mut(at)
            .ok_or_else(|| error("invalid HIR jump patch location", span))?;
        match instruction {
            Instruction::Jump(destination)
            | Instruction::JumpIfFalse(destination)
            | Instruction::JumpIfFalseKeep(destination)
            | Instruction::JumpIfTrueKeep(destination) => {
                *destination = target;
                Ok(())
            }
            _ => Err(error("attempted to patch a non-jump instruction", span)),
        }
    }

    fn expression(&mut self, expression: &HirExpression) -> Result<(), Diagnostic> {
        self.validate_expression(expression)?;
        let previous = self.current_span;
        self.current_span = expression.span;
        let result = self.expression_inner(expression);
        self.current_span = previous;
        result
    }

    fn validate_expression(&self, expression: &HirExpression) -> Result<(), Diagnostic> {
        let invalid = || error("inconsistent types in HIR expression", expression.span);
        match &expression.kind {
            HirExpressionKind::Literal(value) => {
                let expected = match value {
                    Value::Number(_) => Type::Number,
                    Value::String(_) => Type::String,
                    Value::Bool(_) => Type::Bool,
                    Value::Nil => Type::Nil,
                };
                if expression.ty != expected {
                    return Err(invalid());
                }
            }
            HirExpressionKind::Local(local) => {
                self.resolve(*local, expression.span)?;
                if self.local_types.get(local) != Some(&expression.ty) {
                    return Err(invalid());
                }
            }
            HirExpressionKind::Unary(operator, operand) => {
                self.validate_expression(operand)?;
                let result = match (operator, operand.ty) {
                    (TokenKind::Minus, Type::Number) => Type::Number,
                    (TokenKind::Bang, Type::Bool) => Type::Bool,
                    _ => return Err(invalid()),
                };
                if expression.ty != result {
                    return Err(invalid());
                }
            }
            HirExpressionKind::Binary(left, operator, right) => {
                self.validate_expression(left)?;
                self.validate_expression(right)?;
                let result = match operator {
                    TokenKind::Plus if left.ty == Type::Number && right.ty == Type::Number => {
                        Type::Number
                    }
                    TokenKind::Plus if left.ty == Type::String && right.ty == Type::String => {
                        Type::String
                    }
                    TokenKind::Minus | TokenKind::Star | TokenKind::Slash | TokenKind::Percent
                        if left.ty == Type::Number && right.ty == Type::Number =>
                    {
                        Type::Number
                    }
                    TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq
                        if left.ty == Type::Number && right.ty == Type::Number =>
                    {
                        Type::Bool
                    }
                    TokenKind::EqEq | TokenKind::BangEq if left.ty == right.ty => Type::Bool,
                    TokenKind::AndAnd | TokenKind::OrOr
                        if left.ty == Type::Bool && right.ty == Type::Bool =>
                    {
                        Type::Bool
                    }
                    _ => return Err(invalid()),
                };
                if expression.ty != result {
                    return Err(invalid());
                }
            }
            HirExpressionKind::Call(function_id, arguments) => {
                let function = self
                    .functions
                    .get(*function_id as usize)
                    .filter(|function| function.id == *function_id)
                    .ok_or_else(|| error("invalid function ID in HIR call", expression.span))?;
                if arguments.len() != function.params.len()
                    || arguments
                        .iter()
                        .zip(&function.params)
                        .any(|(argument, parameter)| argument.ty != parameter.ty)
                    || expression.ty != function.return_type
                {
                    return Err(invalid());
                }
                for argument in arguments {
                    self.validate_expression(argument)?;
                }
            }
            HirExpressionKind::BuiltinPrint(arguments) => {
                if expression.ty != Type::Nil {
                    return Err(invalid());
                }
                for argument in arguments {
                    self.validate_expression(argument)?;
                }
            }
        }
        if expression.ty == Type::Unknown {
            return Err(error("unresolved type in HIR expression", expression.span));
        }
        Ok(())
    }

    fn expression_inner(&mut self, expression: &HirExpression) -> Result<(), Diagnostic> {
        match &expression.kind {
            HirExpressionKind::Literal(value) => {
                self.emit(Instruction::Constant(value.clone()));
            }
            HirExpressionKind::Local(local) => {
                let slot = self.resolve(*local, expression.span)?;
                self.emit(Instruction::LoadLocal(slot));
            }
            HirExpressionKind::Unary(operator, operand) => {
                self.expression(operand)?;
                self.emit(Instruction::Unary(operator.clone()));
            }
            HirExpressionKind::Binary(left, operator, right) => {
                self.expression(left)?;
                match operator {
                    TokenKind::AndAnd => {
                        let branch = self.code.len();
                        self.emit(Instruction::JumpIfFalseKeep(usize::MAX));
                        self.emit(Instruction::Pop);
                        self.expression(right)?;
                        self.patch(branch, self.code.len(), expression.span)?;
                    }
                    TokenKind::OrOr => {
                        let branch = self.code.len();
                        self.emit(Instruction::JumpIfTrueKeep(usize::MAX));
                        self.emit(Instruction::Pop);
                        self.expression(right)?;
                        self.patch(branch, self.code.len(), expression.span)?;
                    }
                    _ => {
                        self.expression(right)?;
                        self.emit(Instruction::Binary(operator.clone()));
                    }
                }
            }
            HirExpressionKind::Call(function_id, arguments) => {
                let function = self
                    .functions
                    .get(*function_id as usize)
                    .filter(|function| function.id == *function_id)
                    .ok_or_else(|| error("invalid function ID in HIR call", expression.span))?;
                for argument in arguments {
                    self.expression(argument)?;
                }
                if arguments.len() != function.params.len() {
                    return Err(error("invalid call arity in HIR", expression.span));
                }
                self.emit(Instruction::Call {
                    function: function.name.clone(),
                    arguments: arguments.len(),
                });
            }
            HirExpressionKind::BuiltinPrint(arguments) => {
                for argument in arguments {
                    self.expression(argument)?;
                }
                self.emit(Instruction::Print {
                    arguments: arguments.len(),
                });
            }
        }
        Ok(())
    }

    fn scoped(&mut self, statements: &[HirStatement]) -> Result<(), Diagnostic> {
        self.visible.push(std::collections::HashSet::new());
        let result = self.statements(statements);
        self.visible.pop();
        result
    }

    fn statements(&mut self, statements: &[HirStatement]) -> Result<(), Diagnostic> {
        for statement in statements {
            match statement {
                HirStatement::Let(local, value) => {
                    if value.ty != local.ty {
                        return Err(error(
                            "HIR initializer type does not match local",
                            local.span,
                        ));
                    }
                    self.expression(value)?;
                    let slot = self.define(local)?;
                    self.emit_at(Instruction::StoreLocal(slot), local.span);
                }
                HirStatement::Assign(local, span, value) => {
                    let slot = self.resolve(*local, *span)?;
                    if self.local_mutability.get(local) != Some(&true) {
                        return Err(error("assignment to immutable HIR local", *span));
                    }
                    if self.local_types.get(local) != Some(&value.ty) {
                        return Err(error("HIR assignment type does not match local", *span));
                    }
                    self.expression(value)?;
                    self.emit_at(Instruction::StoreLocal(slot), *span);
                }
                HirStatement::Print(value) => {
                    self.expression(value)?;
                    self.emit(Instruction::Print { arguments: 1 });
                    self.emit(Instruction::Pop);
                }
                HirStatement::Expression(value) => {
                    self.expression(value)?;
                    self.emit(Instruction::Pop);
                }
                HirStatement::Return(value) => {
                    if value.as_ref().map_or(Type::Nil, |value| value.ty)
                        != self.function.return_type
                    {
                        return Err(error(
                            "HIR return type does not match function",
                            self.function.span,
                        ));
                    }
                    if let Some(value) = value {
                        self.expression(value)?;
                    } else {
                        self.emit(Instruction::Constant(Value::Nil));
                    }
                    self.emit(Instruction::Return);
                }
                HirStatement::If(condition, yes, no) => {
                    if condition.ty != Type::Bool {
                        return Err(error("HIR if condition must be Bool", condition.span));
                    }
                    self.expression(condition)?;
                    let branch = self.code.len();
                    self.emit(Instruction::JumpIfFalse(usize::MAX));
                    self.scoped(yes)?;
                    let end = self.code.len();
                    self.emit(Instruction::Jump(usize::MAX));
                    self.patch(branch, self.code.len(), condition.span)?;
                    self.scoped(no)?;
                    self.patch(end, self.code.len(), condition.span)?;
                }
                HirStatement::While(condition, body) => {
                    if condition.ty != Type::Bool {
                        return Err(error("HIR loop condition must be Bool", condition.span));
                    }
                    let start = self.code.len();
                    self.expression(condition)?;
                    let exit = self.code.len();
                    self.emit(Instruction::JumpIfFalse(usize::MAX));
                    self.scoped(body)?;
                    self.emit(Instruction::Jump(start));
                    self.patch(exit, self.code.len(), condition.span)?;
                }
                HirStatement::Block(body) => self.scoped(body)?,
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<FunctionIr, Diagnostic> {
        if self.function.return_type == Type::Unknown
            || self
                .function
                .params
                .iter()
                .any(|parameter| parameter.ty == Type::Unknown)
        {
            return Err(error(
                "unresolved type in HIR function signature",
                self.function.span,
            ));
        }
        self.statements(&self.function.body)?;
        if !matches!(self.code.last(), Some(Instruction::Return)) {
            self.current_span = self.function.span;
            self.emit(Instruction::Constant(Value::Nil));
            self.emit(Instruction::Return);
        }
        Ok(FunctionIr {
            name: self.function.name.clone(),
            parameter_count: self.function.params.len(),
            local_count: u32::try_from(self.slots.len())
                .map_err(|_| error("IR local slot space exhausted", self.function.span))?,
            instructions: self.code,
            instruction_spans: self.spans,
        })
    }
}

fn hir_can_fall_through(statements: &[HirStatement]) -> bool {
    for statement in statements {
        let falls_through = match statement {
            HirStatement::Return(_) => false,
            HirStatement::Block(body) => hir_can_fall_through(body),
            HirStatement::If(_, yes, no) => hir_can_fall_through(yes) || hir_can_fall_through(no),
            HirStatement::While(_, _) => true,
            _ => true,
        };
        if !falls_through {
            return false;
        }
    }
    true
}

pub fn lower_hir(program: &HirProgram) -> Result<ModuleIr, Diagnostic> {
    let Some(entry) = program.functions.get(program.entry as usize) else {
        return Err(error("HIR entry function ID is invalid", Span::default()));
    };
    if entry.id != program.entry || entry.name != "main" || !entry.params.is_empty() {
        return Err(error("HIR entry function must be main()", entry.span));
    }
    let mut names = std::collections::HashSet::new();
    let mut module = ModuleIr::default();
    for (index, function) in program.functions.iter().enumerate() {
        if function.id as usize != index || !names.insert(function.name.as_str()) {
            return Err(error(
                "invalid or duplicate function ID/name in HIR",
                function.span,
            ));
        }
        if function.return_type != Type::Nil && hir_can_fall_through(&function.body) {
            return Err(error(
                "non-Nil HIR function can fall through",
                function.span,
            ));
        }
        module
            .functions
            .push(HirLowerer::new(function, &program.functions)?.finish()?);
    }
    validate(&module)?;
    Ok(module)
}

/// Validate module structure and stack/local invariants before execution or
/// handing IR to a future backend.
pub fn validate(module: &ModuleIr) -> Result<(), Diagnostic> {
    let span = Span {
        start: 0,
        end: 0,
        line: 1,
        column: 1,
    };
    let mut functions = HashMap::new();
    for function in &module.functions {
        if functions.insert(function.name.as_str(), function).is_some() {
            return Err(error(
                format!("duplicate IR function '{}'", function.name),
                span,
            ));
        }
        if function.parameter_count > function.local_count as usize {
            return Err(error(
                format!(
                    "IR function '{}' has fewer locals than parameters",
                    function.name
                ),
                span,
            ));
        }
        if function.instructions.is_empty() {
            return Err(error(
                format!("IR function '{}' has no instructions", function.name),
                span,
            ));
        }
        if function.instruction_spans.len() != function.instructions.len() {
            return Err(error(
                format!(
                    "IR function '{}' has incomplete source locations",
                    function.name
                ),
                span,
            ));
        }
    }
    let Some(main) = functions.get("main") else {
        return Err(error("IR module has no main function", span));
    };
    if main.parameter_count != 0 {
        return Err(error("IR main function must not take parameters", span));
    }

    for function in &module.functions {
        let len = function.instructions.len();
        let mut depths = vec![None; len];
        let mut work = vec![(0usize, 0usize)];
        while let Some((pc, depth)) = work.pop() {
            if pc == len {
                return Err(error(
                    format!("IR function '{}' can fall off its end", function.name),
                    span,
                ));
            }
            if let Some(previous) = depths[pc] {
                if previous != depth {
                    return Err(error(
                        format!(
                            "inconsistent stack depth at instruction {pc} in '{}'",
                            function.name
                        ),
                        span,
                    ));
                }
                continue;
            }
            depths[pc] = Some(depth);
            let underflow = || {
                error(
                    format!(
                        "IR stack underflow at instruction {pc} in '{}'",
                        function.name
                    ),
                    span,
                )
            };
            let mut next_depth = depth;
            let mut successors = Vec::with_capacity(2);
            match &function.instructions[pc] {
                Instruction::Constant(_) => next_depth += 1,
                Instruction::LoadLocal(slot) => {
                    if *slot >= function.local_count {
                        return Err(error(
                            format!("invalid local slot {slot} in '{}'", function.name),
                            span,
                        ));
                    }
                    next_depth += 1;
                }
                Instruction::StoreLocal(slot) => {
                    if *slot >= function.local_count {
                        return Err(error(
                            format!("invalid local slot {slot} in '{}'", function.name),
                            span,
                        ));
                    }
                    if depth < 1 {
                        return Err(underflow());
                    }
                    next_depth -= 1;
                }
                Instruction::Unary(op) => {
                    if depth < 1 {
                        return Err(underflow());
                    }
                    if !matches!(op, TokenKind::Bang | TokenKind::Minus) {
                        return Err(error("unsupported IR unary operator", span));
                    }
                }
                Instruction::Binary(op) => {
                    if depth < 2 {
                        return Err(underflow());
                    }
                    if !matches!(
                        op,
                        TokenKind::Plus
                            | TokenKind::Minus
                            | TokenKind::Star
                            | TokenKind::Slash
                            | TokenKind::Percent
                            | TokenKind::EqEq
                            | TokenKind::BangEq
                            | TokenKind::Lt
                            | TokenKind::LtEq
                            | TokenKind::Gt
                            | TokenKind::GtEq
                    ) {
                        return Err(error("unsupported IR binary operator", span));
                    }
                    next_depth -= 1;
                }
                Instruction::Call {
                    function: callee,
                    arguments,
                } => {
                    let Some(target) = functions.get(callee.as_str()) else {
                        return Err(error(format!("undefined IR function '{callee}'"), span));
                    };
                    if target.parameter_count != *arguments {
                        return Err(error(
                            format!("IR call to '{callee}' has incorrect argument count"),
                            span,
                        ));
                    }
                    if depth < *arguments {
                        return Err(underflow());
                    }
                    next_depth = depth - arguments + 1;
                }
                Instruction::Print { arguments } => {
                    if depth < *arguments {
                        return Err(underflow());
                    }
                    next_depth = depth - arguments + 1;
                }
                Instruction::Pop => {
                    if depth < 1 {
                        return Err(underflow());
                    }
                    next_depth -= 1;
                }
                Instruction::Jump(target) => {
                    successors.push(*target);
                }
                Instruction::JumpIfFalse(target) => {
                    if depth < 1 {
                        return Err(underflow());
                    }
                    next_depth -= 1;
                    successors.push(*target);
                }
                Instruction::JumpIfFalseKeep(target) | Instruction::JumpIfTrueKeep(target) => {
                    if depth < 1 {
                        return Err(underflow());
                    }
                    successors.push(*target);
                }
                Instruction::Return => {
                    if depth != 1 {
                        return Err(error(
                            format!(
                                "IR return in '{}' requires exactly one stack value, found {depth}",
                                function.name
                            ),
                            span,
                        ));
                    }
                    continue;
                }
            }
            if matches!(function.instructions[pc], Instruction::Jump(_)) {
                // Unconditional jumps have no fallthrough edge.
            } else {
                successors.push(pc + 1);
            }
            for target in successors {
                if target >= len {
                    return Err(error(
                        format!(
                            "IR jump target {target} is out of bounds in '{}'",
                            function.name
                        ),
                        span,
                    ));
                }
                work.push((target, next_depth));
            }
        }
    }
    Ok(())
}

fn error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic {
        message: message.into(),
        span,
    }
}
fn locate(mut diagnostic: Diagnostic, span: Span) -> Diagnostic {
    if has_placeholder_span(diagnostic.span) {
        diagnostic.span = span;
    }
    diagnostic
}

#[cfg(test)]
#[derive(Default)]
struct Locals {
    scopes: Vec<HashMap<String, u32>>,
    next: u32,
}
#[cfg(test)]
impl Locals {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            next: 0,
        }
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
    fn define(&mut self, name: &str, span: Span) -> Result<u32, Diagnostic> {
        let current = self
            .scopes
            .last_mut()
            .expect("local scope stack is non-empty");
        if current.contains_key(name) {
            return Err(error(
                format!("'{name}' already declared in this scope"),
                span,
            ));
        }
        let slot = self.next;
        self.next += 1;
        current.insert(name.to_owned(), slot);
        Ok(slot)
    }
    fn resolve(&self, name: &str, span: Span) -> Result<u32, Diagnostic> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .ok_or_else(|| error(format!("undefined variable '{name}'"), span))
    }
}

#[cfg(test)]
struct Builder<'a> {
    function: &'a Function,
    locals: Locals,
    code: Vec<Instruction>,
    spans: Vec<Span>,
    current_span: Span,
    functions: &'a HashMap<String, usize>,
}
#[cfg(test)]
impl<'a> Builder<'a> {
    fn new(
        function: &'a Function,
        functions: &'a HashMap<String, usize>,
    ) -> Result<Self, Diagnostic> {
        let mut locals = Locals::new();
        for parameter in &function.params {
            locals.define(parameter, function.span)?;
        }
        Ok(Self {
            function,
            locals,
            code: Vec::new(),
            spans: Vec::new(),
            current_span: function.span,
            functions,
        })
    }
    fn emit(&mut self, instruction: Instruction) {
        self.code.push(instruction);
        self.spans.push(self.current_span);
    }
    fn patch(&mut self, at: usize, target: usize, span: Span) -> Result<(), Diagnostic> {
        let instruction = self
            .code
            .get_mut(at)
            .ok_or_else(|| error("invalid jump patch location", span))?;
        match instruction {
            Instruction::Jump(destination)
            | Instruction::JumpIfFalse(destination)
            | Instruction::JumpIfFalseKeep(destination)
            | Instruction::JumpIfTrueKeep(destination) => {
                *destination = target;
                Ok(())
            }
            _ => Err(error("attempted to patch a non-jump instruction", span)),
        }
    }
    fn expression(&mut self, expression: &Expr) -> Result<(), Diagnostic> {
        let previous = self.current_span;
        self.current_span = expression.span();
        let result = self.expression_inner(expression);
        self.current_span = previous;
        result
    }
    fn expression_inner(&mut self, expression: &Expr) -> Result<(), Diagnostic> {
        match expression {
            Expr::Lit(value, _) => self.emit(Instruction::Constant(value.clone())),
            Expr::Var(name, span) => {
                let slot = self.locals.resolve(name, *span)?;
                self.emit(Instruction::LoadLocal(slot));
            }
            Expr::Unary(operator, value, _) => {
                self.expression(value)?;
                self.emit(Instruction::Unary(operator.clone()));
            }
            Expr::Binary(left, operator, right, _) => {
                self.expression(left)?;
                match operator {
                    TokenKind::AndAnd => {
                        let end = self.code.len();
                        self.emit(Instruction::JumpIfFalseKeep(usize::MAX));
                        self.emit(Instruction::Pop);
                        self.expression(right)?;
                        let after = self.code.len();
                        self.patch(end, after, self.function.span)?;
                    }
                    TokenKind::OrOr => {
                        let end = self.code.len();
                        self.emit(Instruction::JumpIfTrueKeep(usize::MAX));
                        self.emit(Instruction::Pop);
                        self.expression(right)?;
                        let after = self.code.len();
                        self.patch(end, after, self.function.span)?;
                    }
                    _ => {
                        self.expression(right)?;
                        self.emit(Instruction::Binary(operator.clone()));
                    }
                }
            }
            Expr::Call(name, arguments, span) => {
                for argument in arguments {
                    self.expression(argument)?;
                }
                if name == "print" {
                    self.emit(Instruction::Print {
                        arguments: arguments.len(),
                    });
                } else {
                    let expected = self
                        .functions
                        .get(name)
                        .ok_or_else(|| error(format!("undefined function '{name}'"), *span))?;
                    if *expected != arguments.len() {
                        return Err(error(
                            format!(
                                "function '{name}' expects {expected} arguments, got {}",
                                arguments.len()
                            ),
                            *span,
                        ));
                    }
                    self.emit(Instruction::Call {
                        function: name.clone(),
                        arguments: arguments.len(),
                    });
                }
            }
        }
        Ok(())
    }
    fn scoped(&mut self, body: &[Stmt]) -> Result<(), Diagnostic> {
        self.locals.push();
        let result = self.statements(body);
        self.locals.pop();
        result
    }
    fn statements(&mut self, statements: &[Stmt]) -> Result<(), Diagnostic> {
        for statement in statements {
            match statement {
                Stmt::Let(name, value, _, _) | Stmt::TypedLet(name, _, value, _, _) => {
                    self.expression(value)?;
                    let slot = self.locals.define(name, self.function.span)?;
                    self.emit(Instruction::StoreLocal(slot));
                }
                Stmt::Assign(name, value, _) => {
                    let slot = self.locals.resolve(name, self.function.span)?;
                    self.expression(value)?;
                    self.emit(Instruction::StoreLocal(slot));
                }
                Stmt::Print(value) => {
                    self.expression(value)?;
                    self.emit(Instruction::Print { arguments: 1 });
                    self.emit(Instruction::Pop);
                }
                Stmt::Expr(value) => {
                    self.expression(value)?;
                    self.emit(Instruction::Pop);
                }
                Stmt::Return(value) => {
                    if let Some(value) = value {
                        self.expression(value)?;
                    } else {
                        self.emit(Instruction::Constant(Value::Nil));
                    }
                    self.emit(Instruction::Return);
                }
                Stmt::Block(body) => self.scoped(body)?,
                Stmt::If(condition, yes, no) => {
                    self.expression(condition)?;
                    let branch = self.code.len();
                    self.emit(Instruction::JumpIfFalse(usize::MAX));
                    self.scoped(yes)?;
                    let end = self.code.len();
                    self.emit(Instruction::Jump(usize::MAX));
                    let else_start = self.code.len();
                    self.patch(branch, else_start, self.function.span)?;
                    self.scoped(no)?;
                    let after = self.code.len();
                    self.patch(end, after, self.function.span)?;
                }
                Stmt::While(condition, body) => {
                    let start = self.code.len();
                    self.expression(condition)?;
                    let exit = self.code.len();
                    self.emit(Instruction::JumpIfFalse(usize::MAX));
                    self.scoped(body)?;
                    self.emit(Instruction::Jump(start));
                    let after = self.code.len();
                    self.patch(exit, after, self.function.span)?;
                }
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<FunctionIr, Diagnostic> {
        self.statements(&self.function.body)?;
        if !matches!(self.code.last(), Some(Instruction::Return)) {
            self.emit(Instruction::Constant(Value::Nil));
            self.emit(Instruction::Return);
        }
        Ok(FunctionIr {
            name: self.function.name.clone(),
            parameter_count: self.function.params.len(),
            local_count: self.locals.next,
            instructions: self.code,
            instruction_spans: self.spans,
        })
    }
}

/// Test-only direct AST lowering retained for backend differential tests.
#[cfg(test)]
pub fn lower_ast(program: &Program) -> Result<ModuleIr, Diagnostic> {
    let functions: HashMap<String, usize> = program
        .functions
        .iter()
        .map(|function| (function.name.clone(), function.params.len()))
        .collect();
    if !functions.contains_key("main") {
        return Err(error(
            "program must define fn main()",
            Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        ));
    }
    if functions["main"] != 0 {
        return Err(error(
            "main must not take parameters",
            Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        ));
    }
    let mut module = ModuleIr::default();
    for function in &program.functions {
        module
            .functions
            .push(Builder::new(function, &functions)?.finish()?);
    }
    Ok(module)
}

/// Type-check a source AST, lower it to typed HIR, then emit target-independent stack IR.
pub fn lower(program: &Program) -> Result<ModuleIr, Diagnostic> {
    let hir = crate::typecheck::check_typed(program)?;
    lower_hir(&hir)
}

/// Execute a lowered module with the reference stack-machine runtime.
///
/// The VM deliberately shares the bootstrap interpreter's value and operator
/// semantics. It is a correctness/reference backend, not a native code backend.
pub fn execute(module: &ModuleIr) -> Result<Value, Diagnostic> {
    execute_internal(module, false).map(|(value, _)| value)
}

/// Execute a module and return its printed lines alongside the result.
pub fn execute_with_output(module: &ModuleIr) -> Result<(Value, Vec<String>), Diagnostic> {
    execute_internal(module, true)
}

fn execute_internal(
    module: &ModuleIr,
    capture_output: bool,
) -> Result<(Value, Vec<String>), Diagnostic> {
    validate(module)?;
    let functions: HashMap<String, FunctionIr> = module
        .functions
        .iter()
        .cloned()
        .map(|function| (function.name.clone(), function))
        .collect();
    if !functions.contains_key("main") {
        return Err(error(
            "IR module has no main function",
            Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        ));
    }
    let mut fuel = 1_000_000u64;
    let mut output = Vec::new();
    let value = execute_function(
        "main",
        Vec::new(),
        &functions,
        &mut fuel,
        0,
        &mut output,
        capture_output,
    )?;
    Ok((value, output))
}

fn execute_function(
    name: &str,
    arguments: Vec<Value>,
    functions: &HashMap<String, FunctionIr>,
    fuel: &mut u64,
    depth: usize,
    output: &mut Vec<String>,
    capture_output: bool,
) -> Result<Value, Diagnostic> {
    let span = Span {
        start: 0,
        end: 0,
        line: 1,
        column: 1,
    };
    if depth >= super::MAX_CALL_DEPTH {
        return Err(error("maximum IR call depth exceeded", span));
    }
    let function = functions
        .get(name)
        .ok_or_else(|| error(format!("undefined IR function '{name}'"), span))?;
    if arguments.len() != function.parameter_count {
        return Err(error(
            format!(
                "IR function '{name}' expects {} arguments, got {}",
                function.parameter_count,
                arguments.len()
            ),
            span,
        ));
    }
    let mut locals = vec![Value::Nil; function.local_count as usize];
    for (index, value) in arguments.into_iter().enumerate() {
        locals[index] = value;
    }
    let mut stack: Vec<Value> = Vec::new();
    let mut pc = 0usize;
    while pc < function.instructions.len() {
        if *fuel == 0 {
            return Err(error("IR execution limit reached", span));
        }
        *fuel -= 1;
        let instruction_span = function.instruction_spans[pc];
        let span = instruction_span;
        let instruction = &function.instructions[pc];
        pc += 1;
        match instruction {
            Instruction::Constant(value) => stack.push(value.clone()),
            Instruction::LoadLocal(slot) => {
                let value = locals
                    .get(*slot as usize)
                    .ok_or_else(|| error(format!("invalid local slot {slot}"), span))?;
                stack.push(value.clone());
            }
            Instruction::StoreLocal(slot) => {
                let value = stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on store", span))?;
                let destination = locals
                    .get_mut(*slot as usize)
                    .ok_or_else(|| error(format!("invalid local slot {slot}"), span))?;
                *destination = value;
            }
            Instruction::Unary(operator) => {
                let value = stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on unary operation", span))?;
                let result = match operator {
                    TokenKind::Bang => Value::Bool(!super::truth(&value)),
                    TokenKind::Minus => {
                        Value::Number(-super::num(value).map_err(|e| locate(e, span))?)
                    }
                    _ => return Err(error("unsupported IR unary operator", span)),
                };
                stack.push(result);
            }
            Instruction::Binary(operator) => {
                let right = stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on binary operation", span))?;
                let left = stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on binary operation", span))?;
                stack.push(super::binary(left, operator, right).map_err(|e| locate(e, span))?);
            }
            Instruction::Call {
                function: callee,
                arguments: count,
            } => {
                if stack.len() < *count {
                    return Err(error("IR stack underflow on call", span));
                }
                let values = stack.split_off(stack.len() - *count);
                let result = execute_function(
                    callee,
                    values,
                    functions,
                    fuel,
                    depth + 1,
                    output,
                    capture_output,
                )
                .map_err(|e| locate(e, span))?;
                stack.push(result);
            }
            Instruction::Print { arguments: count } => {
                if stack.len() < *count {
                    return Err(error("IR stack underflow on print", span));
                }
                let values = stack.split_off(stack.len() - count);
                for value in values {
                    let line = value.to_string();
                    if capture_output {
                        output.push(line);
                    } else {
                        println!("{line}");
                    }
                }
                stack.push(Value::Nil);
            }
            Instruction::Pop => {
                stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on pop", span))?;
            }
            Instruction::Jump(target) => {
                if *target > function.instructions.len() {
                    return Err(error("IR jump target is out of bounds", span));
                }
                pc = *target;
            }
            Instruction::JumpIfFalse(target) => {
                let condition = stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on conditional jump", span))?;
                if !super::truth(&condition) {
                    if *target > function.instructions.len() {
                        return Err(error("IR jump target is out of bounds", span));
                    }
                    pc = *target;
                }
            }
            Instruction::JumpIfFalseKeep(target) => {
                let condition = stack
                    .last()
                    .ok_or_else(|| error("IR stack underflow on short-circuit jump", span))?;
                if !super::truth(condition) {
                    if *target > function.instructions.len() {
                        return Err(error("IR jump target is out of bounds", span));
                    }
                    pc = *target;
                }
            }
            Instruction::JumpIfTrueKeep(target) => {
                let condition = stack
                    .last()
                    .ok_or_else(|| error("IR stack underflow on short-circuit jump", span))?;
                if super::truth(condition) {
                    if *target > function.instructions.len() {
                        return Err(error("IR jump target is out of bounds", span));
                    }
                    pc = *target;
                }
            }
            Instruction::Return => {
                return stack
                    .pop()
                    .ok_or_else(|| error("IR stack underflow on return", span));
            }
        }
    }
    Err(error(
        format!("IR function '{name}' reached the end without return"),
        span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};

    fn lower_source(source: &str) -> Result<ModuleIr, Diagnostic> {
        let tokens = Lexer::new(source).tokenize()?;
        let program = Parser::new(tokens).parse()?;
        lower(&program)
    }

    #[test]
    fn interpreter_and_ir_vm_match_values_and_output() {
        let programs = [
            "fn main(){let x=6*7;print(x);return x;}",
            "fn sum(n: Number)->Number{if n<=0{return 0;}else{return n+sum(n-1);}} fn main(){print(sum(5));}",
            "fn main(){var i=0;var total=0;while i<4{total=total+i;i=i+1;} {let total=\"inner\";print(total);} print(total);}",
            "fn side()->Bool{print(\"visited\");return true;} fn main(){let a=false&&side();let b=true||side();print(a,b);}",
            "weave main[] { live count: Number := 0 again while count<3 { count := count+1 } when count==3 { show count } otherwise { show 0 } yield }",
        ];

        for source in programs {
            let program = crate::compile(source).unwrap();
            let module = lower(&program).unwrap();
            let expected = crate::Interpreter::new(program).run_captured().unwrap();
            let actual = execute_with_output(&module).unwrap();
            assert_eq!(actual, expected, "source: {source}");
        }
    }

    #[test]
    fn typed_hir_and_legacy_ast_lowering_agree() {
        let source = "weave fact[n: Number] gives Number { when n <= 1 { yield 1 } otherwise { yield n * fact!{n - 1} } } weave main[] { live index: Number := 0; kept result: Number := fact!{5}; again while index < 2 { kept hidden := index; index = index + 1; } when false && (1 / 0 == 0) { show 999 } otherwise { show result } }";
        let program = crate::compile(source).unwrap();
        let typed = lower(&program).unwrap();
        let ast = lower_ast(&program).unwrap();
        let typed_result = execute_with_output(&typed).unwrap();
        let ast_result = execute_with_output(&ast).unwrap();
        assert_eq!(typed_result, ast_result);
        assert_eq!(typed_result.1, vec!["120"]);
    }

    #[test]
    fn malformed_hir_local_references_are_rejected() {
        let mut hir = crate::compile_to_hir("fn main() { let value = 1; print(value); }").unwrap();
        let HirStatement::Expression(expression) = &mut hir.functions[0].body[1] else {
            panic!("expected print expression");
        };
        let HirExpressionKind::BuiltinPrint(arguments) = &mut expression.kind else {
            panic!("expected builtin print expression");
        };
        arguments[0].kind = HirExpressionKind::Local(999);
        let diagnostic = lower_hir(&hir).unwrap_err();
        assert!(diagnostic.message.contains("unresolved HIR local ID"));
    }

    #[test]
    fn malformed_hir_types_and_immutable_assignments_are_rejected() {
        let mut bad_type = crate::compile_to_hir("fn main() { let value = 1; }").unwrap();
        let HirStatement::Let(_, value) = &mut bad_type.functions[0].body[0] else {
            panic!("expected local declaration");
        };
        value.ty = Type::String;
        assert!(lower_hir(&bad_type)
            .unwrap_err()
            .message
            .contains("initializer type"));

        let mut immutable =
            crate::compile_to_hir("fn main() { var value = 1; value = 2; }").unwrap();
        let HirStatement::Let(local, _) = &mut immutable.functions[0].body[0] else {
            panic!("expected local declaration");
        };
        local.mutable = false;
        assert!(lower_hir(&immutable)
            .unwrap_err()
            .message
            .contains("immutable HIR local"));

        let mut fallthrough = crate::compile_to_hir("fn main() {}").unwrap();
        fallthrough.functions[0].return_type = Type::Number;
        assert!(lower_hir(&fallthrough)
            .unwrap_err()
            .message
            .contains("can fall through"));
    }

    #[test]
    fn executes_lowered_arithmetic_and_locals() {
        let module = lower_source("fn main(){let answer=(6*7)+1;return answer;}").unwrap();
        assert_eq!(execute(&module).unwrap(), Value::Number(43.0));
    }

    #[test]
    fn executes_calls_and_control_flow() {
        let module = lower_source(
            "fn twice(x:Number)->Number{return x*2;} fn main(){var n=0;while n<3{n=n+1;}if n==3{return twice(n);}else{return 0;}}"
        ).unwrap();
        assert_eq!(execute(&module).unwrap(), Value::Number(6.0));
    }

    #[test]
    fn executes_short_circuit_without_evaluating_right_side() {
        let module = lower_source(
            "fn fail()->Bool{return 1/0==0;} fn main(){if false && fail(){return 1;}if true || fail(){return 9;}return 0;}"
        ).unwrap();
        assert_eq!(execute(&module).unwrap(), Value::Number(9.0));
    }

    #[test]
    fn rejects_malformed_ir_stack_operations() {
        let module = ModuleIr {
            functions: vec![FunctionIr {
                name: "main".into(),
                parameter_count: 0,
                local_count: 0,
                instructions: vec![
                    Instruction::Pop,
                    Instruction::Constant(Value::Nil),
                    Instruction::Return,
                ],
                instruction_spans: vec![Span::default(); 3],
            }],
        };
        assert!(validate(&module).is_err());
        assert!(execute(&module).is_err());
    }

    #[test]
    fn verifier_rejects_bad_frames_calls_and_control_flow() {
        let malformed = [
            ModuleIr {
                functions: vec![FunctionIr {
                    name: "main".into(),
                    parameter_count: 1,
                    local_count: 0,
                    instructions: vec![Instruction::Constant(Value::Nil), Instruction::Return],
                    instruction_spans: vec![Span::default(); 2],
                }],
            },
            ModuleIr {
                functions: vec![FunctionIr {
                    name: "main".into(),
                    parameter_count: 0,
                    local_count: 0,
                    instructions: vec![Instruction::LoadLocal(0), Instruction::Return],
                    instruction_spans: vec![Span::default(); 2],
                }],
            },
            ModuleIr {
                functions: vec![FunctionIr {
                    name: "main".into(),
                    parameter_count: 0,
                    local_count: 0,
                    instructions: vec![
                        Instruction::Call {
                            function: "missing".into(),
                            arguments: 0,
                        },
                        Instruction::Return,
                    ],
                    instruction_spans: vec![Span::default(); 2],
                }],
            },
            ModuleIr {
                functions: vec![FunctionIr {
                    name: "main".into(),
                    parameter_count: 0,
                    local_count: 0,
                    instructions: vec![
                        Instruction::Constant(Value::Nil),
                        Instruction::Jump(9),
                        Instruction::Return,
                    ],
                    instruction_spans: vec![Span::default(); 3],
                }],
            },
            ModuleIr {
                functions: vec![FunctionIr {
                    name: "main".into(),
                    parameter_count: 0,
                    local_count: 0,
                    instructions: vec![
                        Instruction::Constant(Value::Nil),
                        Instruction::Constant(Value::Nil),
                        Instruction::Return,
                    ],
                    instruction_spans: vec![Span::default(); 3],
                }],
            },
        ];
        for module in malformed {
            assert!(validate(&module).is_err());
            assert!(execute(&module).is_err());
        }
    }

    #[test]
    fn verifier_accepts_lowered_programs() {
        let module = lower_source("fn main(){let x=false&&true;if x{print(1);}else{print(2);}}")
            .expect("program should lower");
        validate(&module).expect("lowered IR should satisfy verifier invariants");
    }

    #[test]
    fn lowers_functions_and_local_slots() {
        let module = lower_source("fn add(a: Number,b: Number)->Number{return a+b;} fn main(){let total=add(2,3);print(total);}")
            .expect("program should lower");
        assert_eq!(module.functions.len(), 2);
        let main = module.functions.iter().find(|f| f.name == "main").unwrap();
        assert_eq!(main.local_count, 1);
        assert!(main.instructions.iter().any(
            |i| matches!(i, Instruction::Call { function, arguments: 2 } if function == "add")
        ));
        assert!(main
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Print { arguments: 1 })));
    }

    #[test]
    fn lowers_nested_shadowed_locals_to_distinct_slots() {
        let module =
            lower_source("fn main(){let value=1;{let value=2;print(value);}print(value);}")
                .expect("shadowing should lower");
        let main = &module.functions[0];
        assert_eq!(main.local_count, 2);
        let loads: Vec<u32> = main
            .instructions
            .iter()
            .filter_map(|i| {
                if let Instruction::LoadLocal(slot) = i {
                    Some(*slot)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(loads, vec![1, 0]);
    }

    #[test]
    fn lowers_if_and_loop_control_flow() {
        let module = lower_source("fn main(){var i=0;while i<2{if i==1{print(i);}i=i+1;}}")
            .expect("control flow should lower");
        let main = &module.functions[0];
        assert!(main
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::JumpIfFalse(_))));
        assert!(main
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Jump(_))));
    }

    #[test]
    fn rejects_unknown_variables_and_bad_calls() {
        assert!(lower_source("fn main(){print(missing); }").is_err());
        assert!(lower_source("fn other(){} fn main(){other(1);}").is_err());
    }

    #[test]
    fn lowering_diagnostics_retain_expression_locations() {
        let diagnostic = lower_source("fn main() {\n  print(missing)\n}").unwrap_err();
        assert_eq!(diagnostic.span.line, 2);
        assert_eq!(diagnostic.span.column, 9);
    }

    #[test]
    fn vm_runtime_errors_retain_expression_locations() {
        let module = lower_source("fn main() {\n  let x = 1 / 0\n}").unwrap();
        let diagnostic = execute(&module).unwrap_err();
        assert_eq!(diagnostic.span.line, 2);
        assert_eq!(diagnostic.span.column, 11);
    }

    #[test]
    fn lowers_short_circuit_operators_without_eager_binary_ops() {
        let module = lower_source("fn main(){let a=false && true;let b=true || false;}")
            .expect("logical expressions should lower");
        let code = &module.functions[0].instructions;
        assert!(code
            .iter()
            .any(|i| matches!(i, Instruction::JumpIfFalseKeep(_))));
        assert!(code
            .iter()
            .any(|i| matches!(i, Instruction::JumpIfTrueKeep(_))));
    }

    #[test]
    fn preserves_all_print_arguments() {
        let module = lower_source("fn main(){print(1,2,3); }").unwrap();
        assert!(module.functions[0]
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Print { arguments: 3 })));
    }
}
