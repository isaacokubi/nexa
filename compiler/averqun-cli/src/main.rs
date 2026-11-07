use averqun::{
    compile,
    ir::{execute, lower},
    Interpreter, Lexer, Parser,
};
use std::{env, fs, process};
fn usage() {
    eprintln!(
        "Averqun 0.1.0 bootstrap\nUsage: averqun <run|ir-run|check|tokens|ast|ir> <file.avq>"
    );
}
fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 3 {
        usage();
        process::exit(2)
    }
    let (cmd, path) = (&a[1], &a[2]);
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("averqun: {e}");
            process::exit(1)
        }
    };
    let result = match cmd.as_str() {
        "tokens" => Lexer::new(&src).tokenize().map(|t| {
            for x in t {
                println!("{:?} @ {}:{}", x.kind, x.span.line, x.span.column)
            }
        }),
        "ast" => Lexer::new(&src)
            .tokenize()
            .and_then(|t| Parser::new(t).parse())
            .map(|p| println!("{p:#?}")),
        "check" => compile(&src).map(|_| println!("✓ {path}: syntax OK")),
        "ir" => compile(&src)
            .and_then(|p| lower(&p))
            .map(|m| println!("{m:#?}")),
        "ir-run" => compile(&src)
            .and_then(|p| lower(&p))
            .and_then(|m| execute(&m).map(|_| ())),
        "run" => compile(&src).map(|p| {
            if let Err(e) = Interpreter::new(p).run() {
                eprintln!("{e}");
                process::exit(1)
            }
        }),
        _ => {
            usage();
            process::exit(2)
        }
    };
    if let Err(e) = result {
        eprintln!("{e}");
        process::exit(1)
    }
}
