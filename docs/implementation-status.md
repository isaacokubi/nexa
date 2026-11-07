# Implementation Status

This status is tied to the current repository branch and must not be confused with the intended Averqun feature set.

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
- `examples/hello.avq` ran successfully through both `run` and `ir-run`. Both files under `tests/compile-fail/` exited with failure as expected.
- Six pre-existing stashes were inspected and left intact; their changes overlap earlier versions of the recovered lexer/parser, scope, type-checker, and IR work. No stash was applied or dropped.
- Added `language-specification.md` to distinguish accepted bootstrap semantics from planned, unimplemented design choices.
- Recorded the user's explicit requirement for a distinct Averqun language identity; an alternative syntax extension is implemented while the original bootstrap syntax remains accepted for compatibility.
- Implemented the first alternative-syntax subset in the lexer and parser (`weave`, bracketed parameter lists, `gives`, `kept`/`live`, `:=`, `yield`, `when`/`otherwise`, `again while`, `show`, and `name!{...}` calls). `examples/weave-hello.avq` passes through interpreter and IR VM; differential tests cover loop/branch behavior in the new form. Legacy forms remain accepted during the transition.
- Added captured-output APIs for the interpreter and IR VM and aligned their call-depth limits. Differential coverage compares results and printed lines across recursion, branching, loops, nested shadowing, and short-circuit evaluation.
- After these changes, `cargo fmt --all -- --check`, `cargo test --workspace` (33 passed), `cargo build --workspace`, `git diff --check`, and interpreter/IR execution of `examples/hello.avq` all passed.
- The alternative syntax subset now has 34 passing workspace tests, including differential execution, Clippy with warnings denied, and direct execution of `examples/weave-hello.avq` through both interpreter and IR VM.
- GitHub push and PR update have not been attempted; the configured remote is `origin`.
- At the user's direction, the working product name is now **Averqun**. The Rust package/binary and local source extension have been renamed; the hosted repository is still `github.com/isaacokubi/nexa` because no GitHub CLI or repository-management connection is available in this environment.
- `Averqun` is an un-cleared working name. Preliminary exact-name web searches did not surface an obvious programming-language/software project, but professional clearance remains a pre-release task.

## Compiler foundation milestone: IR validation (2026-11-07)
- Added a public IR validator and made the IR VM validate modules before execution. It checks duplicate/missing entry functions, entry arity, parameter/local frame sizes, reachable stack effects and joins, local indices, supported operators, call targets and arity, jump targets, and return stack shape.
- Added malformed-module tests for frame/local errors, missing callees, invalid jumps, underflow, and incorrect return depth; lowered-program validation is covered too.
- Fixed IR lowering for output statements to discard the output operation's unit result. Before this correction, repeated output accumulated values and functions could return with an invalid operand stack.
- Baseline before this change: formatting clean, 34 workspace tests passed, build succeeded, and legacy/alternative examples ran in both execution paths.
- After this change: formatting clean, 36 workspace tests passed, build succeeded, `examples/hello.avq` and `examples/weave-hello.avq` executed through `ir-run`, and `git diff --check` passed.
- The working tree still includes the pre-existing, uncommitted Averqun rename and documentation changes. They were preserved; no stash was applied or dropped, and no Git history was rewritten.

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
