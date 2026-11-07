# NEXA Type System (Bootstrap + Design)

## Status and boundary
This document separates **implemented bootstrap checks** from the intended language contract. The current frontend infers primitive expression types from literals and checks operators, assignments with known types, and Boolean conditions. Function parameters and call results currently use `Unknown`; there is no declared-type AST or complete function return checking yet. Do not interpret the design sections as implemented compiler guarantees.

## Primitive types
The stable design vocabulary is:
- `Bool`: exactly `true` or `false`.
- `I8/I16/I32/I64/I128`, `U8/U16/U32/U64/U128`: fixed-width integers.
- `F32/F64`: IEEE floating point.
- `Char`: Unicode scalar value.
- `String`: UTF-8 text.
- `Unit`: absence of a meaningful value.
- `Never`: expressions that cannot complete normally.

The bootstrap interpreter currently exposes Number (f64), String, Bool and Nil only.

## Inference and annotations (planned)
Local inference should infer a binding from its initializer; public API boundaries should prefer explicit parameter and return types. An annotation must be checked, never used to silently coerce an incompatible value. Numeric conversions are explicit.

## Operators
- Arithmetic accepts compatible numeric operands.
- String concatenation is explicit and only accepts strings.
- Equality requires compatible types.
- Ordering is defined only for supported ordered types.
- Logical operators require Bool and short-circuit.

## Diagnostics
A type error should include the source span, expected type, actual type, and a concise remediation. Compiler diagnostics must be deterministic and should never panic on invalid user input.

## Future RFCs
Integer overflow policy, numeric literal inference, generic constraints, nullable/optional types, trait-based operator overloading, and ABI representation require RFCs before stabilization.
