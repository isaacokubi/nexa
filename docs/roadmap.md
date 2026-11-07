Averqun Roadmap
============

- [x] Rust workspace and CLI bootstrap
- [x] Lexer, tokens, spans and diagnostics
- [x] AST/parser for function, block, variable, return, if and expression subset
- [x] Interpreter and initial unit tests
- [x] Bootstrap lexical name, scope, mutability, primitive annotation, operator, return-flow, and inferable function return checks
- [ ] Sound type checking for all expressions; replace permissive Unknown handling and attach source spans to AST/semantic diagnostics
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

A feature is complete only when implementation, tests, docs and compatibility guarantees exist.
