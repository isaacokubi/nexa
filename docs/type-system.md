# Averqun Type System (Bootstrap + Design)

## Status and boundary

This document distinguishes implemented bootstrap checks from the intended language contract. The bootstrap frontend infers primitive expression types, validates operators and Boolean conditions, checks typed local declarations, typed function parameters and call arguments, propagates declared function return types to call sites, and validates explicit return values.

Annotated non-Nil functions must return a value on every path that can complete normally. The current analysis handles sequential statements, nested blocks, and both branches of conditionals. Loops are conservatively treated as potentially falling through because the current AST does not prove loop termination. Unannotated functions and functions annotated as Nil may fall through.

The bootstrap interpreter currently exposes Number (f64), String, Bool, and Nil. This is not the complete planned type system.

## Primitive types

The stable design vocabulary is:
- Bool: exactly true or false.
- I8/I16/I32/I64/I128 and U8/U16/U32/U64/U128: fixed-width integers.
- F32/F64: IEEE floating point.
- Char: Unicode scalar value.
- String: UTF-8 text.
- Unit: absence of a meaningful value.
- Never: expressions that cannot complete normally.

Only the bootstrap types listed above are currently implemented.

## Inference and annotations

Local inference derives a binding's type from its initializer. Explicit annotations are checked against the initializer and do not silently coerce incompatible values. Function parameter annotations are checked against calls. For an unannotated function, return types are inferred from its return expressions and propagated to call sites, including through calls to other inferable functions. All returns must agree. A return depending on an unannotated parameter or an unresolved recursive return cycle remains Unknown and is rejected with a request for an explicit return annotation. This improves call-site checking but does not make the bootstrap checker a complete static type system: Unknown operands still remain in some non-return expression contexts, and semantic source spans are not yet retained.

## Operators

- Arithmetic accepts compatible numeric operands.
- String concatenation accepts two strings.
- Equality requires compatible types.
- Ordering currently accepts numeric operands.
- Logical operators require Bool operands.
- The interpreter reports runtime errors such as division by zero.

## Return-flow analysis

For every function inferred or annotated to return a non-Nil value, every path that can reach the end of the function body is rejected. A return in one conditional branch is insufficient unless the other branch also returns or a later sequential statement returns. Nested blocks are analyzed recursively. A loop is not considered a guaranteed return, even when its condition is the literal true, until the language has explicit termination analysis.

## Diagnostics

A type error should include the source span, expected type, actual type, and a concise remediation. Compiler diagnostics must be deterministic and should never panic on invalid user input. Accurate expression-level spans for semantic diagnostics remain future work.

## Future RFCs

Integer overflow policy, numeric literal inference, generic constraints, nullable/optional types, trait-based operator overloading, and ABI representation require RFCs before stabilization.
