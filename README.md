NEXA — Native Execution Architecture
====================================

NEXA is an experimental general-purpose language project aimed at native performance, strict typing and memory safety. This repository currently contains a Rust bootstrap frontend/interpreter—not an LLVM compiler and not production-ready.

Current features
----------------
- UTF-8 lexer, source spans, comments, string escapes and numeric literals
- Recursive-descent parser and AST for functions, parameters, blocks, let/var, return, if/else, calls and expressions
- Interpreter: numbers, strings, booleans, nil, arithmetic, comparisons and short-circuit logic
- CLI: `nexa run|check|tokens|ast FILE`
- Unit tests

Build and test (Rust stable)
---------------------------
    cargo build --workspace
    cargo test --workspace
    cargo run -p nexa -- run examples/hello.nexa
    cargo run -p nexa -- check examples/hello.nexa

See docs/specification.md and docs/roadmap.md. Language semantics are evolving.