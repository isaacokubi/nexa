NEXA Compiler Architecture
=========================

Bootstrap pipeline
------------------
source → lexer → tokens with source spans → recursive-descent parser → AST → semantic validation / primitive type checks → interpreter

The frontend can also lower the validated AST into a target-independent stack IR. IR includes local-slot operations, calls, print operations, arithmetic and logical operators, conditional and loop jumps, and return instructions. Logical AND/OR use keep-value conditional jumps so a future IR executor preserves short-circuit behavior.

`nexa ir FILE` prints the lowered IR for inspection. The IR is an intermediate compiler representation, not yet an executable VM or native backend.

The Rust crate intentionally starts dependency-free. CLI is the user-facing executable.

Planned native pipeline
-----------------------
source → lexer → parser → AST → name resolution → typed HIR → ownership/borrow analysis → MIR/stack IR → optimization → LLVM IR → native object/linker

Safety analyses must precede code generation. LLVM is an optimization/backend tool, not a performance guarantee; equivalent workloads must be benchmarked against C and Rust.

Engineering constraints
----------------------
- Diagnostics retain byte offsets and 1-based line/column.
- Compiler phases are independently testable.
- Target triples, runtime ABI and language semantics require versioning.
- Unsafe operations should eventually require explicit unsafe scopes; not present in bootstrap.