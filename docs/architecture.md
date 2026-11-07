Averqun Compiler Architecture
=========================

Bootstrap pipeline
------------------
source → lexer → tokens with source spans → recursive-descent parser → AST → lexical name/mutability checks → primitive type checking with conservative function return inference → interpreter

The frontend can also lower the validated AST into a target-independent stack IR. IR includes local-slot operations, calls, print operations, arithmetic and logical operators, conditional and loop jumps, and return instructions. Logical AND/OR use keep-value conditional jumps so the IR VM preserves short-circuit behavior.

`averqun ir FILE` prints the lowered IR for inspection. `averqun ir-run FILE` lowers and executes it in the reference stack VM. This VM is not a native backend. Interpreter and VM expose captured-output APIs for differential tests and use the same recursion-depth limit. Their execution fuel counts differ because the interpreter counts source statements and the VM counts IR instructions.

The Rust crate intentionally starts dependency-free. CLI is the user-facing executable. `Interpreter::run_captured` and `ir::execute_with_output` expose printed lines with the result; normal execution streams output as it is produced.

Planned native pipeline
-----------------------
source → lexer → parser → AST → name resolution → typed HIR → ownership/borrow analysis → MIR/stack IR → optimization → LLVM IR → native object/linker

Safety analyses must precede code generation. LLVM is an optimization/backend tool, not a performance guarantee; equivalent workloads must be benchmarked against C and Rust.

The return inference pass resolves unannotated function return types from consistent return expressions, propagates resolved return types through calls, and rejects unresolved return types. The AST does not yet preserve spans on expressions or statements, so many semantic/type diagnostics still use a placeholder location. `Unknown` is still permitted in selected expression contexts; this is not a sound complete type checker.

Engineering constraints
----------------------
- Diagnostics retain byte offsets and 1-based line/column.
- Compiler phases are independently testable.
- Target triples, runtime ABI and language semantics require versioning.
- Unsafe operations should eventually require explicit unsafe scopes; not present in bootstrap.
