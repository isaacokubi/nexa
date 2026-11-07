//! Static type inference and operator validation for the Averqun bootstrap AST.
use crate::{Diagnostic, Expr, Function, Program, Span, Stmt, TokenKind, Value};
use std::collections::HashMap;

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
    params: Vec<Option<String>>,
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
            Stmt::Let(name, value, _) => {
                let ty = infer(value, scope, functions)?;
                scope.define(name.clone(), ty);
                None
            }
            Stmt::TypedLet(name, annotation, value, _) => {
                let ty = declared_type(annotation)?;
                let actual = infer(value, scope, functions)?;
                if actual != ty && actual != Type::Unknown {
                    return Err(error(format!(
                        "cannot initialize {annotation} variable '{name}' with {actual}"
                    )));
                }
                scope.define(name.clone(), ty);
                None
            }
            Stmt::If(condition, yes, no) => {
                let condition_type = infer(condition, scope, functions)?;
                if condition_type != Type::Bool && condition_type != Type::Unknown {
                    return Err(error(format!(
                        "if condition must be Bool, found {condition_type}"
                    )));
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
                    return Err(error(format!(
                        "while condition must be Bool, found {condition_type}"
                    )));
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
            Stmt::Assign(_, value) | Stmt::Print(value) | Stmt::Expr(value) => {
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
    match e {
        Expr::Lit(v) => Ok(match v {
            Value::Number(_) => Type::Number,
            Value::String(_) => Type::String,
            Value::Bool(_) => Type::Bool,
            Value::Nil => Type::Nil,
        }),
        Expr::Var(n) => s
            .get(n)
            .ok_or_else(|| error(format!("undefined variable '{n}'"))),
        Expr::Unary(op, x) => {
            let t = infer(x, s, fs)?;
            match op {
                TokenKind::Minus if t == Type::Number => Ok(Type::Number),
                TokenKind::Bang if t == Type::Bool => Ok(Type::Bool),
                TokenKind::Minus => Err(error(format!("unary '-' requires Number, found {t}"))),
                TokenKind::Bang => Err(error(format!("'!' requires Bool, found {t}"))),
                _ => Ok(Type::Unknown),
            }
        }
        Expr::Binary(a, op, b) => {
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
        Expr::Call(n, args) => {
            if n == "print" {
                for a in args {
                    infer(a, s, fs)?;
                }
                return Ok(Type::Nil);
            }
            let sig = fs
                .get(n)
                .ok_or_else(|| error(format!("undefined function '{n}'")))?;
            if sig.params.len() != args.len() {
                return Err(error(format!(
                    "function '{n}' expects {} arguments, got {}",
                    sig.params.len(),
                    args.len()
                )));
            }
            for (a, expected) in args.iter().zip(&sig.params) {
                let actual = infer(a, s, fs)?;
                if let Some(name) = expected {
                    let ty = declared_type(name)?;
                    if actual != ty && actual != Type::Unknown {
                        return Err(error(format!(
                            "argument to '{n}' expects {ty}, found {actual}"
                        )));
                    }
                }
            }
            Ok(sig.inferred_ret)
        }
    }
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
            Stmt::TypedLet(n, name, e, _) => {
                let expected = declared_type(name)?;
                let actual = infer(e, s, fs)?;
                if actual != expected && actual != Type::Unknown {
                    return Err(error(format!(
                        "cannot initialize {name} variable '{n}' with {actual}"
                    )));
                }
                if s.contains_current(n) {
                    return Err(error(format!("'{n}' already declared in this scope")));
                }
                s.define(n.clone(), expected);
            }
            Stmt::While(c, b) => {
                let t = infer(c, s, fs)?;
                if t != Type::Bool && t != Type::Unknown {
                    return Err(error(format!("while condition must be Bool, found {t}")));
                }
                scoped_stmts(b, s, fs, ret)?;
            }
            Stmt::Let(n, e, _) => {
                let t = infer(e, s, fs)?;
                if s.contains_current(n) {
                    return Err(error(format!("'{n}' already declared in this scope")));
                }
                s.define(n.clone(), t);
            }
            Stmt::Assign(n, e) => {
                let old = s
                    .get(n)
                    .ok_or_else(|| error(format!("undefined variable '{n}'")))?;
                let new = infer(e, s, fs)?;
                if old != new && new != Type::Unknown {
                    return Err(error(format!("cannot assign {new} to '{n}' of type {old}")));
                }
            }
            Stmt::Print(e) | Stmt::Expr(e) => {
                infer(e, s, fs)?;
            }
            Stmt::Return(e) => {
                let actual = if let Some(e) = e {
                    infer(e, s, fs)?
                } else {
                    Type::Nil
                };
                if let Some(expected) = ret {
                    if actual != expected && actual != Type::Unknown {
                        return Err(error(format!(
                            "return type mismatch: expected {expected}, found {actual}"
                        )));
                    }
                }
            }
            Stmt::If(c, a, b) => {
                let t = infer(c, s, fs)?;
                if t != Type::Bool && t != Type::Unknown {
                    return Err(error(format!("if condition must be Bool, found {t}")));
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
pub fn check(p: &Program) -> Result<(), Diagnostic> {
    let mut fs: HashMap<String, Signature> = p
        .functions
        .iter()
        .map(|f| {
            Ok((
                f.name.clone(),
                Signature {
                    params: f.param_types.clone(),
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

    // Resolve straightforward inferred returns to a fixed point. Annotated
    // signatures seed recursive calls; unannotated recursive cycles remain
    // Unknown and are rejected below instead of weakening call-site checks.
    for _ in 0..=p.functions.len() {
        let mut changed = false;
        for function in &p.functions {
            if function.return_type.is_some() {
                continue;
            }
            let mut scope = Scope::new();
            for (name, annotation) in function.params.iter().zip(&function.param_types) {
                let ty = annotation
                    .as_deref()
                    .map(declared_type)
                    .transpose()?
                    .unwrap_or(Type::Unknown);
                scope.define(name.clone(), ty);
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
        param_types,
        return_type: _,
        body,
        ..
    } in &p.functions
    {
        let expected = Some(fs[name].inferred_ret);
        let mut s = Scope::new();
        for (p, t) in params.iter().zip(param_types) {
            let ty = match t {
                Some(name) => declared_type(name)?,
                None => Type::Unknown,
            };
            s.define(p.clone(), ty);
        }
        if expected == Some(Type::Unknown) {
            return Err(error(format!(
                "in function '{name}': cannot infer return type; add an explicit return annotation"
            )));
        }
        stmts(body, &mut s, &fs, expected)
            .map_err(|e| error(format!("in function '{name}': {}", e.message)))?;
        if expected.is_some_and(|t| t != Type::Nil) && can_fall_through(body) {
            return Err(error(format!(
                "in function '{name}': not all control-flow paths return a value"
            )));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, Parser};
    fn c(src: &str) -> Result<(), Diagnostic> {
        check(&Parser::new(Lexer::new(src).tokenize()?).parse()?)
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
