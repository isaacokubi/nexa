# Implementation Status

This status is tied to the current repository branch and must not be confused with the intended NEXA feature set.

## Present in bootstrap
- Rust workspace and CLI.
- Lexer/token model with byte offsets and line/column.
- Recursive descent parser and AST.
- Function declarations and calls.
- Variables, assignment, blocks, returns, conditionals.
- Primitive values and interpreter.
- Basic semantic checks (function existence/arity, names, mutability, main entry).
- Static primitive inference/operator validation in `typecheck.rs`, including typed local declarations, parameters, call arguments, explicit return types and all-path return checks for annotated non-Nil functions.
- Runtime and static-checker lexical scopes for blocks, conditional branches and loop iterations, including legal inner-block shadowing and outer mutable binding updates.
- Unit tests and hello example.

## Not implemented
- Declared type AST/HIR and complete signature/return checking.
- Integer widths, overflow modes, generics, traits, structs, enums, pattern matching.
- Ownership/borrow/lifetime checking and destructor semantics.
- MIR, optimization pipeline, LLVM backend, object/executable emission.
- Runtime ABI, native allocator, platform FFI.
- Modules/import resolution and package manager.
- Complete standard library.
- Async runtime and concurrency.
- Formatter, debugger, LSP, test runner, docs website.
- Fuzzing, benchmarks, full platform CI and release signing.

## Accuracy rule
A feature is considered implemented only when code, tests, documentation, and a verified CI/build result support the claim. Roadmap completion is not a substitute for these artifacts.
