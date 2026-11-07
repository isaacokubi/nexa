Averqun Compiler Architecture
=========================

Bootstrap pipeline
------------------
source → lexer → tokens with source spans → recursive-descent parser → AST → lexical name/mutability checks → primitive type checking with conservative function return inference → typed HIR

The frontend can also lower the validated AST into a target-independent stack IR. IR includes local-slot operations, calls, print operations, arithmetic and logical operators, conditional and loop jumps, and return instructions. Logical AND/OR use keep-value conditional jumps so the IR VM preserves short-circuit behavior.

`averqun ir FILE` prints the lowered IR for inspection. `averqun ir-run FILE` lowers and executes it in the reference stack VM. This VM is not a native backend. Interpreter and VM expose captured-output APIs for differential tests and use the same recursion-depth limit. Their execution fuel counts differ because the interpreter counts source statements and the VM counts IR instructions.

The Rust crate intentionally starts dependency-free. CLI is the user-facing executable. `Interpreter::run_captured` and `ir::execute_with_output` expose printed lines with the result; normal execution streams output as it is produced.

Planned native pipeline
-----------------------
source → lexer → parser → AST → name resolution → typed HIR → ownership/borrow analysis → MIR/stack IR → optimization → LLVM IR → native object/linker

Safety analyses must precede code generation. LLVM is an optimization/backend tool, not a performance guarantee; equivalent workloads must be benchmarked against C and Rust.

The type checker infers omitted parameter types from consistent call arguments and unannotated return types from consistent return expressions, propagates both through the call graph, and rejects unresolved types before final body checking. Expression AST nodes retain byte/line/column spans, and variable declarations and assignment targets now retain spans through semantic and runtime checks. Function-level and parameter-annotation errors are not yet consistently precise. The current typed layer is still limited to four primitive types and is not a complete general-purpose type system.

`compile_to_hir` returns typed expression nodes with primitive result types, resolved function IDs, function-local local IDs, mutability, and source spans. `averqun hir FILE` prints this representation. The reference interpreter continues to execute the source AST. The stack IR currently also lowers from the AST; HIR-to-IR lowering is the next compiler-boundary task.

Engineering constraints
----------------------
- The recursive-descent parser bounds recursive expression/unary and nested-block parsing to 128 levels and each same-precedence operator chain to 128 operators. The public parser constructor supplies EOF when an external token stream omits it.
- Diagnostics retain byte offsets and 1-based line/column.
- Compiler phases are independently testable.
- Target triples, runtime ABI and language semantics require versioning.
- Unsafe operations should eventually require explicit unsafe scopes; not present in bootstrap.
