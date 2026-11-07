NEXA — Native Execution Architecture
====================================

NEXA is an experimental general-purpose language project aimed at native performance, strict typing and memory safety. This repository currently contains a Rust bootstrap frontend/interpreter—not an LLVM compiler and not production-ready.

Current features
----------------
- UTF-8 lexer, source spans, comments, string escapes and numeric literals
- Recursive-descent parser and AST for functions, optionally typed parameters, blocks, let/var (including checked local, parameter and return type annotations), return, if/else, while loops, calls and expressions
- Interpreter: numbers, strings, booleans, nil, arithmetic, comparisons and short-circuit logic
- CLI: `nexa run|check|tokens|ast FILE`
- Bootstrap static type checks for primitive operators, declarations, parameters, call arguments and return annotations
- Lexical block scopes with shadowing boundaries and assignment to visible outer mutable bindings
- All-path return checking for explicitly annotated non-Nil functions
- Unit tests

Build and test (Rust stable)
---------------------------
    cargo build --workspace
    cargo test --workspace
    cargo run -p nexa -- run examples/hello.nexa
    cargo run -p nexa -- check examples/hello.nexa

See [documentation index](docs/README.md), [specification](docs/specification.md), [type system](docs/type-system.md), [memory model](docs/memory-model.md), and [release gates](docs/release-readiness.md). Language semantics are evolving.