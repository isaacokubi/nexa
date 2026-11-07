NEXA Compiler Architecture
=========================

Bootstrap pipeline
------------------
source → lexer → tokens with source spans → recursive-descent parser → AST → interpreter

The Rust crate intentionally starts dependency-free. CLI is the user-facing executable.

Planned native pipeline
-----------------------
source → lexer → parser → AST → name resolution → typed HIR → ownership/borrow analysis → MIR → optimization → LLVM IR → native object/linker

Safety analyses must precede code generation. LLVM is an optimization/backend tool, not a performance guarantee; equivalent workloads must be benchmarked against C and Rust.

Engineering constraints
----------------------
- Diagnostics retain byte offsets and 1-based line/column.
- Compiler phases are independently testable.
- Target triples, runtime ABI and language semantics require versioning.
- Unsafe operations should eventually require explicit unsafe scopes; not present in bootstrap.