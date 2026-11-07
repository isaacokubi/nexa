//! Stack-based intermediate representation for the NEXA bootstrap compiler.
//!
//! This is a stable boundary between the source AST and future optimization /
//! native backends. Instructions are deliberately target-independent.

use crate::{Diagnostic, Expr, Function, Program, Span, Stmt, TokenKind, Value};
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
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModuleIr {
    pub functions: Vec<FunctionIr>,
}

fn error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic {
        message: message.into(),
        span,
    }
}

#[derive(Default)]
struct Locals {
    scopes: Vec<HashMap<String, u32>>,
    next: u32,
}
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

struct Builder<'a> {
    function: &'a Function,
    locals: Locals,
    code: Vec<Instruction>,
    functions: &'a HashMap<String, usize>,
}
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
            functions,
        })
    }
    fn emit(&mut self, instruction: Instruction) {
        self.code.push(instruction);
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
            Expr::Call(name, arguments) => {
                for argument in arguments {
                    self.expression(argument)?;
                }
                if name == "print" {
                    self.emit(Instruction::Print {
                        arguments: arguments.len(),
                    });
                } else {
                    let expected = self.functions.get(name).ok_or_else(|| {
                        error(format!("undefined function '{name}'"), self.function.span)
                    })?;
                    if *expected != arguments.len() {
                        return Err(error(
                            format!(
                                "function '{name}' expects {expected} arguments, got {}",
                                arguments.len()
                            ),
                            self.function.span,
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
        })
    }
}

/// Lower a validated source AST into target-independent stack IR.
pub fn lower(program: &Program) -> Result<ModuleIr, Diagnostic> {
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
                    TokenKind::Minus => Value::Number(-super::num(value)?),
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
                stack.push(super::binary(left, operator, right)?);
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
                )?;
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
            }],
        };
        assert!(execute(&module).is_err());
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
