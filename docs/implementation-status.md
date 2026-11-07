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
- Static primitive inference/operator validation in `typecheck.rs`, including typed local declarations, parameters, call arguments, explicit return types and conservative all-path return-flow checks for annotated non-Nil functions.
- Runtime and static-checker lexical scopes for blocks, conditional branches and loop iterations, including legal inner-block shadowing and outer mutable binding updates.
- Target-independent stack IR lowering with local slots, calls, print, arithmetic, short-circuit branches, conditional/loop jumps and returns.
- Executable reference IR virtual machine with local frames, function calls, stack checks, jump validation, execution fuel and recursion-depth limits.
- CLI commands `run`, `check`, `tokens`, `ast`, `ir`, and `ir-run`.
- Unit tests and hello example.

## Recovery and verification checkpoint (2026-11-07)
- Branch `feat/compiler-foundation` was at `4e48b38` and tracked `origin/feat/compiler-foundation`; the local baseline changes were reviewed and recorded in commit `bb4a99f` (`feat: preserve typed bootstrap checks and IR VM`).
- Worktree changes covered the IR VM, type checking, interpreter scopes, CLI exposure, and tests. They passed `cargo fmt --all -- --check`, `cargo test --workspace` (32 passed), and `cargo build --workspace` before checkpointing.
- `examples/hello.nexa` ran successfully through both `run` and `ir-run`. Both files under `tests/compile-fail/` exited with failure as expected.
- Six pre-existing stashes were inspected and left intact; their changes overlap earlier versions of the recovered lexer/parser, scope, type-checker, and IR work. No stash was applied or dropped.
- Added `language-specification.md` to distinguish accepted bootstrap semantics from planned, unimplemented design choices.
- Recorded the user's explicit requirement for a distinct NEXA language identity and a proposed, unimplemented next-version syntax direction. The existing bootstrap syntax remains documented as current behavior.
- Added captured-output APIs for the interpreter and IR VM and aligned their call-depth limits. Differential coverage compares results and printed lines across recursion, branching, loops, nested shadowing, and short-circuit evaluation.
- After these changes, `cargo fmt --all -- --check`, `cargo test --workspace` (33 passed), `cargo build --workspace`, `git diff --check`, and interpreter/IR execution of `examples/hello.nexa` all passed.
- GitHub push and PR update have not been attempted; the configured remote is `origin`.

## Not implemented
- Declared type AST/HIR and a complete, sound static type system; bootstrap Unknown inference and source-span propagation remain limited.
- Integer widths, overflow modes, generics, traits, structs, enums, pattern matching.
- Ownership/borrow/lifetime checking and destructor semantics.
- Full typed MIR, optimization pipeline, LLVM backend, object/executable emission.
- Runtime ABI, native allocator, platform FFI.
- Modules/import resolution and package manager.
- Complete standard library.
- Async runtime and concurrency.
- Formatter, debugger, LSP, test runner, docs website.
- Fuzzing, benchmarks, full platform CI and release signing.

## Accuracy rule
A feature is considered implemented only when code, tests, documentation, and a verified CI/build result support the claim. Roadmap completion is not a substitute for these artifacts.
