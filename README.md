Averqun Programming Language
============================

**Averqun** is the working name for **A Verifiable Execution Runtime for Quick, Unified Native software**. This expansion describes the project's goal; the current Rust bootstrap does not yet produce native executables.

Averqun is an experimental general-purpose language project aimed at native performance, strict typing and memory safety. This repository currently contains a Rust bootstrap frontend/interpreter—not an LLVM compiler and not production-ready.

Current features
----------------
- UTF-8 lexer, source spans, comments, string escapes and numeric literals
- Recursive-descent parser and AST for functions, optionally typed parameters, blocks, let/var (including checked local, parameter and return type annotations), return, if/else, while loops, calls and expressions
- Interpreter: numbers, strings, booleans, nil, arithmetic, comparisons and short-circuit logic
- CLI: `averqun run|ir-run|check|tokens|ast|ir FILE.avq`
- Target-independent stack IR lowering and an executable reference stack VM (`averqun ir FILE` to inspect; `averqun ir-run FILE` to execute lowered IR). The VM is a correctness backend, not native code generation.
- Experimental alternative source syntax using `weave`, `kept`/`live`, `yield`, `when`, `again while`, `show`, and `name!{arguments}`; see the tested [example](examples/weave-hello.avq).
- Bootstrap static type checks for primitive operators, declarations, parameters, call arguments and return annotations
- Lexical block scopes with shadowing boundaries and assignment to visible outer mutable bindings
- All-path return checking for explicitly annotated non-Nil functions
- Unit tests

Build and test (Rust stable)
---------------------------
    cargo build --workspace
    cargo test --workspace
    cargo run -p averqun -- run examples/hello.avq
    cargo run -p averqun -- check examples/hello.avq
    cargo run -p averqun -- ir examples/hello.avq

See [documentation index](docs/README.md), [language specification](docs/language-specification.md), [type system](docs/type-system.md), [memory model](docs/memory-model.md), and [release gates](docs/release-readiness.md). Language semantics are evolving.
