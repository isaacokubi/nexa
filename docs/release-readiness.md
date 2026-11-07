# Release Readiness and Verification

NEXA is currently an experimental bootstrap interpreter/frontend. It is not alpha-complete, production-ready, or memory-safe by compiler guarantee.

## Required gates for an alpha compiler
- [ ] Grammar and AST fully cover the accepted language subset.
- [ ] Static types and function signatures are represented in the AST/HIR, with sound checking and accurate semantic source spans.
- [ ] Resolver supports lexical scopes and module boundaries.
- [ ] Ownership/borrow checker rejects unsound programs.
- [ ] LLVM backend emits executable object files and links platform targets.
- [ ] Runtime ABI and panic/error model are versioned.
- [ ] Unit, integration, golden, fuzz, and regression suites pass.
- [ ] Cross-platform CI covers Linux, Windows, macOS.
- [ ] Diagnostics include accurate source spans and snippets.
- [ ] Standard library APIs and compatibility policy are published.
- [ ] Reproducible builds, dependency audit, supply-chain policy, and release signing exist.
- [ ] Benchmarks compare equivalent workloads with C, C++, Rust, and Go.

## Bootstrap verification
Run:
```sh
cargo test --workspace
cargo build --workspace
cargo run -p nexa -- check examples/hello.nexa
cargo run -p nexa -- run examples/hello.nexa
```

A successful bootstrap check does not establish native compilation, memory safety, or production readiness.
