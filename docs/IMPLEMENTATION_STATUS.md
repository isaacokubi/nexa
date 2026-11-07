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
- The Averqun rename and IR validation milestone were committed as `570f057` and pushed to `origin/feat/compiler-foundation`. The six older stashes remain untouched.

## Type-system milestone: inferred function returns (2026-11-07)
- Added conservative return-type inference for unannotated functions, including branch/block/loop returns and propagation through calls to functions whose return types can be resolved.
- Incompatible return types and return types that remain unresolved now fail compilation; users can state an explicit return annotation for unresolved parameters or recursive cycles.
- Inferred return types now participate in call argument, initializer, assignment, and explicit return checks, and non-Nil inferred functions use definite-return analysis.
- Added tests for successful inference, call-site mismatch rejection, conflicting branches, unresolved parameter returns, and annotated identity functions.
- Baseline at `570f057`: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, and `cargo test --workspace` all passed (36 tests).
- After this change: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, and `cargo test --workspace` passed (38 tests); `cargo clippy --workspace --all-targets -- -D warnings` exited successfully. Clippy emitted only Rust incremental dep-graph warnings about empty memory maps in existing `target/` data. Both `.avq` examples ran through interpreter and IR VM. All three compile-fail fixtures, including inferred-return mismatch, were rejected by `averqun check` as expected.
- Cross-platform CI was not available in this local environment. Commit and push for this milestone are pending.
- Latest committed checkpoint remains `570f057`; the type-inference work is currently uncommitted.

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

## Feature status

| Feature area | Status | Evidence / remaining work |
| --- | --- | --- |
| Lexer, parser, primitive AST | TESTED | 38-test suite; parser depth limits and recovery remain absent |
| Basic name/scope/mutability checks | TESTED | Compile-time checks; symbols and semantic spans are not explicit IR |
| Primitive type checking | TESTED | Primitive annotations and inferred returns work; Unknown expression paths and source spans remain |
| Stack IR validation and VM | TESTED | Structural/stack verification before execution; no basic blocks or typed MIR |
| Interpreter | TESTED | Basic semantics and resource bounds; aggregates/closures/errors absent |
| Native backend, modules, package manager, standard library, concurrency, tooling | NOT STARTED | See release gates and roadmap |

## Type-system milestone: parameter inference (2026-11-07)
- Function signatures now represent resolved parameter types rather than syntax annotations alone. Omitted parameter types are inferred from known call arguments across the program alongside return-type inference.
- Constraint propagation runs to a fixed point so wrappers and call graphs with forward declarations can pass inferred types through multiple functions.
- Parameters without any inferable call type, and parameters called with conflicting concrete types, are rejected. Inferred parameters are used while checking each function body and at call sites.
- Added tests for inferred arithmetic parameters, forward/cross-function propagation, conflicting callers, and unresolved uncalled parameters.
- Verification: formatting, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace` (39 tests), Clippy with warnings denied, and `git diff --check` passed. Both examples ran through interpreter and IR VM; all three compile-fail fixtures were rejected by the CLI.
- Committed and pushed as `3294782` (`feat(types): infer parameter types from call sites`).

## Type-system hardening: reject unresolved expression types (2026-11-07)
- Added a recursive final expression-validation pass. It verifies nested operands and call arguments before statement/type checks, so no `Unknown` result can silently pass through a typed local, condition, assignment, call, return, print, or discarded expression.
- `Unknown` remains available only while collecting inference constraints; final checking rejects any expression still unresolved after parameter and return fixed-point inference.
- Verification: formatting, check, build, 39 workspace tests, Clippy with warnings denied, and `git diff --check` passed. Both examples ran through interpreter and IR VM; all compile-fail fixtures remained rejected.
- Latest commit is `3294782`; this hardening work is uncommitted.

## Next implementation task

Add source spans to typed AST/HIR diagnostics, then introduce a typed HIR boundary before expanding language syntax.

## Accuracy rule
A feature is considered implemented only when code, tests, documentation, and a verified CI/build result support the claim. Roadmap completion is not a substitute for these artifacts.
