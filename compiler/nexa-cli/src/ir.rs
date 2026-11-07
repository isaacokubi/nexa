//! Stack-based intermediate representation for the NEXA bootstrap compiler.
//!
//! This is a stable boundary between the source AST and future optimization /
//! native backends. Instructions are deliberately target-independent.

use std::collections::HashMap;
use crate::{Diagnostic, Expr, Function, Program, Span, Stmt, TokenKind, Value};

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
    Return,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionIr {
    pub name: String,
    pub parameter_count: usize,
    pub local_count: u32,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModuleIr {
    pub functions: Vec<FunctionIr>,
}

fn error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic { message: message.into(), span }
}

#[derive(Default)]
struct Locals {
    scopes: Vec<HashMap<String, u32>>,
    next: u32,
}
impl Locals {
    fn new() -> Self { Self { scopes: vec![HashMap::new()], next: 0 } }
    fn push(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop(&mut self) { if self.scopes.len() > 1 { self.scopes.pop(); } }
    fn define(&mut self, name: &str, span: Span) -> Result<u32, Diagnostic> {
        let current = self.scopes.last_mut().expect("local scope stack is non-empty");
        if current.contains_key(name) {
            return Err(error(format!("'{name}' already declared in this scope"), span));
        }
        let slot = self.next;
        self.next += 1;
        current.insert(name.to_owned(), slot);
        Ok(slot)
    }
    fn resolve(&self, name: &str, span: Span) -> Result<u32, Diagnostic> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name).copied())
            .ok_or_else(|| error(format!("undefined variable '{name}'"), span))
    }
}

struct Builder<'a> {
    function: &'a Function,
    locals: Locals,
    code: Vec<Instruction>,
    functions: &'a HashMap<String, usize>,
}
impl<'a> Builder<'a> {
    fn new(function: &'a Function, functions: &'a HashMap<String, usize>) -> Result<Self, Diagnostic> {
        let mut locals = Locals::new();
        for parameter in &function.params {
            locals.define(parameter, function.span)?;
        }
        Ok(Self { function, locals, code: Vec::new(), functions })
    }
    fn emit(&mut self, instruction: Instruction) { self.code.push(instruction); }
    fn patch(&mut self, at: usize, target: usize, span: Span) -> Result<(), Diagnostic> {
        let instruction = self.code.get_mut(at).ok_or_else(|| error("invalid jump patch location", span))?;
        match instruction {
            Instruction::Jump(destination) | Instruction::JumpIfFalse(destination) => {
                *destination = target;
                Ok(())
            }
            _ => Err(error("attempted to patch a non-jump instruction", span)),
        }
    }
    fn expression(&mut self, expression: &Expr) -> Result<(), Diagnostic> {
        match expression {
            Expr::Lit(value) => self.emit(Instruction::Constant(value.clone())),
            Expr::Var(name) => {
                let slot = self.locals.resolve(name, self.function.span)?;
                self.emit(Instruction::LoadLocal(slot));
            }
            Expr::Unary(operator, value) => {
                self.expression(value)?;
                self.emit(Instruction::Unary(operator.clone()));
            }
            Expr::Binary(left, operator, right) => {
                self.expression(left)?;
                self.expression(right)?;
                self.emit(Instruction::Binary(operator.clone()));
            }
            Expr::Call(name, arguments) => {
                for argument in arguments { self.expression(argument)?; }
                if name == "print" {
                    self.emit(Instruction::Print { arguments: arguments.len() });
                } else {
                    let expected = self.functions.get(name).ok_or_else(|| error(
                        format!("undefined function '{name}'"), self.function.span
                    ))?;
                    if *expected != arguments.len() {
                        return Err(error(format!(
                            "function '{name}' expects {expected} arguments, got {}",
                            arguments.len()
                        ), self.function.span));
                    }
                    self.emit(Instruction::Call { function: name.clone(), arguments: arguments.len() });
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
                Stmt::Let(name, value, _) | Stmt::TypedLet(name, _, value, _) => {
                    self.expression(value)?;
                    let slot = self.locals.define(name, self.function.span)?;
                    self.emit(Instruction::StoreLocal(slot));
                }
                Stmt::Assign(name, value) => {
                    let slot = self.locals.resolve(name, self.function.span)?;
                    self.expression(value)?;
                    self.emit(Instruction::StoreLocal(slot));
                }
                Stmt::Print(value) => {
                    self.expression(value)?;
                    self.emit(Instruction::Print { arguments: 1 });
                }
                Stmt::Expr(value) => {
                    self.expression(value)?;
                    self.emit(Instruction::Pop);
                }
                Stmt::Return(value) => {
                    if let Some(value) = value { self.expression(value)?; }
                    else { self.emit(Instruction::Constant(Value::Nil)); }
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
        })
    }
}

/// Lower a validated source AST into target-independent stack IR.
pub fn lower(program: &Program) -> Result<ModuleIr, Diagnostic> {
    let functions: HashMap<String, usize> = program.functions.iter()
        .map(|function| (function.name.clone(), function.params.len()))
        .collect();
    if !functions.contains_key("main") {
        return Err(error("program must define fn main()", Span { start: 0, end: 0, line: 1, column: 1 }));
    }
    if functions["main"] != 0 {
        return Err(error("main must not take parameters", Span { start: 0, end: 0, line: 1, column: 1 }));
    }
    let mut module = ModuleIr::default();
    for function in &program.functions {
        module.functions.push(Builder::new(function, &functions)?.finish()?);
    }
    Ok(module)
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
    fn lowers_functions_and_local_slots() {
        let module = lower_source("fn add(a: Number,b: Number)->Number{return a+b;} fn main(){let total=add(2,3);print(total);}")
            .expect("program should lower");
        assert_eq!(module.functions.len(), 2);
        let main = module.functions.iter().find(|f| f.name == "main").unwrap();
        assert_eq!(main.local_count, 1);
        assert!(main.instructions.iter().any(|i| matches!(i, Instruction::Call { function, arguments: 2 } if function == "add")));
        assert!(main.instructions.iter().any(|i| matches!(i, Instruction::Print { arguments: 1 })));
    }

    #[test]
    fn lowers_nested_shadowed_locals_to_distinct_slots() {
        let module = lower_source("fn main(){let value=1;{let value=2;print(value);}print(value);}")
            .expect("shadowing should lower");
        let main = &module.functions[0];
        assert_eq!(main.local_count, 2);
        let loads: Vec<u32> = main.instructions.iter().filter_map(|i| {
            if let Instruction::LoadLocal(slot) = i { Some(*slot) } else { None }
        }).collect();
        assert_eq!(loads, vec![1, 0]);
    }

    #[test]
    fn lowers_if_and_loop_control_flow() {
        let module = lower_source("fn main(){var i=0;while i<2{if i==1{print(i);}i=i+1;}}")
            .expect("control flow should lower");
        let main = &module.functions[0];
        assert!(main.instructions.iter().any(|i| matches!(i, Instruction::JumpIfFalse(_))));
        assert!(main.instructions.iter().any(|i| matches!(i, Instruction::Jump(_))));
    }

    #[test]
    fn rejects_unknown_variables_and_bad_calls() {
        assert!(lower_source("fn main(){print(missing); }").is_err());
        assert!(lower_source("fn other(){} fn main(){other(1);}").is_err());
    }

    #[test]
    fn preserves_all_print_arguments() {
        let module = lower_source("fn main(){print(1,2,3); }").unwrap();
        assert!(module.functions[0].instructions.iter().any(|i| matches!(i, Instruction::Print { arguments: 3 })));
    }
}
