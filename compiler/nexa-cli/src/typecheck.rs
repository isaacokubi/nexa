//! Static type inference and operator validation for the NEXA bootstrap AST.
use std::collections::HashMap;
use crate::{Diagnostic,Expr,Function,Program,Stmt,TokenKind,Value,Span};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Type { Number, String, Bool, Nil, Unknown }
impl std::fmt::Display for Type { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { write!(f,"{}",match self{Self::Number=>"Number",Self::String=>"String",Self::Bool=>"Bool",Self::Nil=>"Nil",Self::Unknown=>"Unknown"}) } }
fn error(message:impl Into<String>)->Diagnostic { Diagnostic{message:message.into(),span:Span{start:0,end:0,line:1,column:1}} }
type Scope=HashMap<String,Type>;
fn infer(e:&Expr,s:&Scope,fs:&HashMap<String,Vec<Option<String>>>)->Result<Type,Diagnostic>{
 match e {
  Expr::Lit(v)=>Ok(match v{Value::Number(_)=>Type::Number,Value::String(_)=>Type::String,Value::Bool(_)=>Type::Bool,Value::Nil=>Type::Nil}),
  Expr::Var(n)=>s.get(n).copied().ok_or_else(||error(format!("undefined variable '{n}'"))),
  Expr::Unary(op,x)=>{let t=infer(x,s,fs)?;match op{TokenKind::Minus if t==Type::Number=>Ok(Type::Number),TokenKind::Bang if t==Type::Bool=>Ok(Type::Bool),TokenKind::Minus=>Err(error(format!("unary '-' requires Number, found {t}"))),TokenKind::Bang=>Err(error(format!("'!' requires Bool, found {t}"))),_=>Ok(Type::Unknown)}},
  Expr::Binary(a,op,b)=>{let l=infer(a,s,fs)?;let r=infer(b,s,fs)?;use TokenKind::*;match op{
   Plus if l==Type::Unknown||r==Type::Unknown=>Ok(Type::Unknown),Plus if l==Type::Number&&r==Type::Number=>Ok(Type::Number),Plus if l==Type::String&&r==Type::String=>Ok(Type::String),Plus=>Err(error(format!("'+' requires two Numbers or two Strings; got {l} and {r}"))),
   Minus|Star|Slash|Percent if l==Type::Unknown||r==Type::Unknown=>Ok(Type::Unknown),Minus|Star|Slash|Percent if l==Type::Number&&r==Type::Number=>Ok(Type::Number),Minus|Star|Slash|Percent=>Err(error(format!("arithmetic requires Number operands; got {l} and {r}"))),
   Lt|LtEq|Gt|GtEq if l==Type::Unknown||r==Type::Unknown=>Ok(Type::Bool),Lt|LtEq|Gt|GtEq if l==Type::Number&&r==Type::Number=>Ok(Type::Bool),Lt|LtEq|Gt|GtEq=>Err(error(format!("comparison requires Number operands; got {l} and {r}"))),
   AndAnd|OrOr if l==Type::Unknown||r==Type::Unknown=>Ok(Type::Bool),AndAnd|OrOr if l==Type::Bool&&r==Type::Bool=>Ok(Type::Bool),AndAnd|OrOr=>Err(error(format!("logical operators require Bool operands; got {l} and {r}"))),EqEq|BangEq if l==Type::Unknown||r==Type::Unknown||l==r=>Ok(Type::Bool),EqEq|BangEq=>Err(error(format!("equality operands must have same type; got {l} and {r}"))),_=>Ok(Type::Unknown)}},
  Expr::Call(n,args)=>{if n=="print"{for a in args{infer(a,s,fs)?;}return Ok(Type::Nil)}let sig=fs.get(n).ok_or_else(||error(format!("undefined function '{n}'")))?;if sig.len()!=args.len(){return Err(error(format!("function '{n}' expects {} arguments, got {}",sig.len(),args.len())))}for (a,expected) in args.iter().zip(sig){let actual=infer(a,s,fs)?;if let Some(name)=expected{let ty=declared_type(name)?;if actual!=ty&&actual!=Type::Unknown{return Err(error(format!("argument to '{n}' expects {ty}, found {actual}")))}}}Ok(Type::Unknown)}
 }
}
fn declared_type(name:&str)->Result<Type,Diagnostic>{match name{"Number"=>Ok(Type::Number),"String"=>Ok(Type::String),"Bool"=>Ok(Type::Bool),"Nil"=>Ok(Type::Nil),_=>Err(error(format!("unknown type '{name}'")))}}
fn stmts(b:&[Stmt],s:&mut Scope,fs:&HashMap<String,Vec<Option<String>>>,ret:Option<Type>)->Result<(),Diagnostic>{for x in b{match x{
 Stmt::TypedLet(n,name,e,_)=>{let expected=declared_type(name)?;let actual=infer(e,s,fs)?;if actual!=expected&&actual!=Type::Unknown{return Err(error(format!("cannot initialize {name} variable '{n}' with {actual}")))}if s.contains_key(n){return Err(error(format!("'{n}' already declared in this scope")))}s.insert(n.clone(),expected);},
 Stmt::While(c,b)=>{let t=infer(c,s,fs)?;if t!=Type::Bool&&t!=Type::Unknown{return Err(error(format!("while condition must be Bool, found {t}")))}let mut x=s.clone();stmts(b,&mut x,fs,ret)?;},
 Stmt::Let(n,e,_)=>{let t=infer(e,s,fs)?;if s.contains_key(n){return Err(error(format!("'{n}' already declared in this scope")))}s.insert(n.clone(),t);},
 Stmt::Assign(n,e)=>{let old=*s.get(n).ok_or_else(||error(format!("undefined variable '{n}'")))?;let new=infer(e,s,fs)?;if old!=new&&new!=Type::Unknown{return Err(error(format!("cannot assign {new} to '{n}' of type {old}")))}},
 Stmt::Print(e)|Stmt::Expr(e)=>{infer(e,s,fs)?;},
 Stmt::Return(e)=>{let actual=if let Some(e)=e{infer(e,s,fs)?}else{Type::Nil};if let Some(expected)=ret{if actual!=expected&&actual!=Type::Unknown{return Err(error(format!("return type mismatch: expected {expected}, found {actual}")))}}},
 Stmt::If(c,a,b)=>{let t=infer(c,s,fs)?;if t!=Type::Bool{return Err(error(format!("if condition must be Bool, found {t}")))}let mut x=s.clone();stmts(a,&mut x,fs,ret)?;let mut y=s.clone();stmts(b,&mut y,fs,ret)?;},
 Stmt::Block(b)=>{let mut x=s.clone();stmts(b,&mut x,fs)?;}
}}Ok(())}
pub fn check(p:&Program)->Result<(),Diagnostic>{let fs:HashMap<String,Vec<Option<String>>>=p.functions.iter().map(|f|(f.name.clone(),f.param_types.clone())).collect();for Function{name,params,param_types,return_type,body,..}in &p.functions{let expected=return_type.as_deref().map(declared_type).transpose()?;let mut s=Scope::new();for (p,t) in params.iter().zip(param_types){let ty=match t{Some(name)=>declared_type(name)?,None=>Type::Unknown};s.insert(p.clone(),ty);}stmts(body,&mut s,&fs,expected).map_err(|e|error(format!("in function '{name}': {}",e.message)))?;}Ok(())}
#[cfg(test)]mod tests{use super::*;use crate::{Lexer,Parser};fn c(src:&str)->Result<(),Diagnostic>{check(&Parser::new(Lexer::new(src).tokenize()?).parse()?)}#[test]fn rejects_string_math(){assert!(c("fn main(){print(\"x\"-1);}").is_err())}#[test]fn rejects_non_bool_condition(){assert!(c("fn main(){if 1 {print(2);}}").is_err())}#[test]fn validates_function_return_types(){assert!(c("fn answer()->Number{return 42;} fn main(){print(answer());}").is_ok());assert!(c("fn answer()->Number{return \"wrong\";} fn main(){answer();}").is_err());assert!(c("fn answer()->Number{return;} fn main(){answer();}").is_err())}#[test]fn validates_typed_function_parameters(){assert!(c("fn add(a: Number,b: Number){let total: Number=a+b;print(total);} fn main(){add(1,2);}").is_ok());assert!(c("fn add(a: Number){print(a);} fn main(){add(\"bad\");}").is_err())}#[test]fn validates_declared_local_types(){assert!(c("fn main(){let x: Number=1;let y: String=\"ok\";print(x);print(y);}").is_ok());assert!(c("fn main(){let x: Number=\"wrong\";}").is_err());assert!(c("fn main(){let x: Unknown=1;}").is_err())}#[test]fn accepts_typed_ops(){assert!(c("fn main(){let x=1+2;let y=\"a\"+\"b\";print(x);print(y);}").is_ok())}}
