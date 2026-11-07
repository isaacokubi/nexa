NEXA Language Specification — Draft 0.1.0
========================================

Status: bootstrap draft. Normative guarantees are limited to the implemented subset.

Identity and goals
------------------
NEXA is intended to become a statically typed, memory-safe, ahead-of-time compiled language. The current implementation executes a dynamically represented subset in an interpreter; static typing, ownership and native code generation are not implemented.

Lexical rules (implemented)
--------------------------
- UTF-8 input; ASCII identifiers beginning with a letter or underscore.
- Whitespace ignored; `//` comments to newline.
- Decimal integer/fractional numbers currently represented as f64.
- Double-quoted strings: `\\n`, `\\r`, `\\t`, `\\"`, `\\\\` escapes.
- Keywords: fn, let, var, return, if, else, true, false, nil, print, module, while.

Grammar subset
-------------
    program      = { ["module", identifier, [";"]] function } EOF
    function     = "fn", identifier, "(", [identifier,[":",identifier],{",",identifier,[":",identifier]}], ")", ["->",identifier], block
    block        = "{", {statement}, "}"
    statement    = ("let"|"var"), identifier, [":",identifier], "=", expression
                 | "return", [expression] | "if", expression, block, ["else",(block|statement)]
                 | "while", expression, block | block | expression
    expression   = logical/equality/comparison/additive/multiplicative/unary/primary precedence
    primary      = number|string|true|false|nil|identifier|call|"(",expression,")"

Implemented semantics
--------------------
- Functions use positional arguments; `main()` is the entry point.
- `let` immutable; `var` mutable. Reassignment of let fails during execution.
- Only false and nil are falsey.
- Arithmetic is numeric; plus also concatenates two strings.
- Division/modulo by zero reports an error.
- print(expr) writes one value per line.
- One-million execution fuel guards runaway statements and loop iterations; recursive calls have a bounded depth.\n- Bootstrap primitive type inference checks arithmetic, comparisons, logical operators, Boolean conditions, local annotations, function parameter types, call arguments and explicit return annotations (`Number`, `String`, `Bool`, `Nil`). This is not yet a sound whole-program type system.

Not implemented
--------------
Complete static type checking and function signature typing, structs/enums/traits/generics, ownership/borrowing, arrays/maps, closures, async/concurrency, imports, LLVM/native code, package manager, formatter, debugger and language server.