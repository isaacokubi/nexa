Averqun Roadmap
============

- [x] Rust workspace and CLI bootstrap
- [x] Lexer, tokens, spans and diagnostics
- [x] AST/parser for function, block, variable, return, if and expression subset
- [x] Interpreter and initial unit tests
- [x] Bootstrap lexical name, scope, mutability, primitive annotation, operator, return-flow, and inferable function return checks
- [x] Reject unresolved expression types after fixed-point parameter/return inference; preserve expression spans through type checking and execution
- [x] Declaration/assignment source spans and typed HIR with resolved function/local IDs for primitive types
- [x] Produce typed HIR with resolved local/function IDs and source spans
- [x] Lower typed HIR to stack IR and validate HIR-to-IR boundary
- [ ] Typed HIR and multi-file/module name resolution
- [ ] Compile-pass/compile-fail harness and diagnostic snapshots
- [ ] Specify and implement integer widths, overflow, float semantics, ABI and target triples
- [ ] Sound ownership/borrowing model and tests
- [ ] MIR and LLVM backend; native linking
- [ ] Core runtime, collections, IO and platform abstraction
- [ ] Modules/imports, generics, traits, enums and structs
- [ ] Formatter, package manager, LSP, debugger and testing framework
- [ ] Linux/Windows/macOS CI, fuzzing and performance benchmarks
- [ ] Complete reference docs, tutorials and stable release process
- [x] Bound recursive parser nesting and safely normalize externally supplied token streams

A feature is complete only when implementation, tests, docs and compatibility guarantees exist.
