# Averqun Language Specification

**Version:** 0.1.0 bootstrap with an experimental 0.2 surface-syntax extension
**Status:** normative for the implemented bootstrap subset; design choices are marked **Planned** and are not accepted by the compiler. The syntax in section 10 is implemented as an experimental alternative form.
**Conformance:** this document describes the Rust frontend and interpreter in this repository. A program outside the implemented subset is not an Averqun 0.1 program, even if a future design below describes it.

Averqun is a statically checked language in development. The current compiler is a small Rust bootstrap with an interpreter and a reference stack IR VM. It does not emit native code and does not yet enforce a sound whole-program type system or memory-safety model. The version number and feature labels in this document must not be read as release claims.

## 1. Source files and lexical rules

Source is UTF-8. Identifiers begin with an ASCII letter or `_` and continue with ASCII letters, digits, or `_`. Identifier spelling is case-sensitive. Whitespace is ignored. `//` starts a comment through the end of the line. There are no documentation comments.

The legacy reserved words are `fn`, `let`, `var`, `return`, `if`, `else`, `while`, `true`, `false`, `nil`, `print`, and `module`. The experimental alternative form also reserves `weave`, `kept`, `live`, `yield`, `when`, `otherwise`, `again`, `show`, and `gives`. Reserved words cannot be used as identifiers. `print` is a legacy built-in callable token; `show` is the alternative form's output statement.

Numbers contain decimal digits, optionally followed by `.` and one or more digits. The runtime represents them as IEEE-754 `f64`; there are no integer widths or numeric suffixes. Strings use double quotes and support `\\`, `\n`, `\r`, `\t`, and `\"`. Newlines inside a string and unsupported escapes are errors. Unescaped Unicode scalar values are retained as UTF-8.

The lexer reports the starting byte offset, 1-based line, and 1-based column for each token. Expression nodes retain spans through type checking, interpreter errors, and IR lowering/VM execution. Variable declarations and assignment targets also retain spans for duplicate binding, type annotation, unknown target, and mutability diagnostics. Function-level, parameter-annotation, and some inference diagnostics still use a coarse location; precise locations are not guaranteed for every semantic error yet.

## 2. Grammar

The parser rejects recursive expression/unary nesting and nested blocks deeper than 128 levels, as well as same-precedence operator chains longer than 128 operators, with a diagnostic. These limits apply to both accepted syntax forms.

The grammar below captures the accepted parser subset. A semicolon is optional after variable declarations, returns, assignments, and expression statements. It is not a statement separator requirement; adjacent statements are recognized by their grammar.

```ebnf
program       = { module_decl | function } EOF ;
module_decl   = "module" identifier [ ";" ] ;
function      = "fn" identifier "(" [ parameters ] ")"
                [ "->" identifier ] block ;
parameters    = parameter { "," parameter } ;
parameter     = identifier [ ":" identifier ] ;
block         = "{" { statement } "}" ;
statement     = variable
              | "return" [ expression ] [ ";" ]
              | "if" expression block [ "else" ( block | "if" expression block [ "else" block ] ) ]
              | "while" expression block
              | block
              | identifier "=" expression [ ";" ]
              | expression [ ";" ] ;
variable      = ("let" | "var") identifier [ ":" identifier ] "=" expression [ ";" ] ;
expression    = logical_or ;
logical_or    = logical_and { "||" logical_and } ;
logical_and   = equality { "&&" equality } ;
equality      = comparison { ("==" | "!=") comparison } ;
comparison    = additive { ("<" | "<=" | ">" | ">=") additive } ;
additive      = multiplicative { ("+" | "-") multiplicative } ;
multiplicative= unary { ("*" | "/" | "%") unary } ;
unary         = ("!" | "-") unary | primary ;
primary       = number | string | "true" | "false" | "nil"
              | identifier [ "(" [ arguments ] ")" ]
              | "print" "(" [ arguments ] ")"
              | "(" expression ")" ;
arguments     = expression { "," expression } ;
```

Binary operators associate left-to-right. Precedence from lowest to highest is `||`, `&&`, equality, ordering, addition/subtraction, multiplication/division/remainder, then unary `!` and `-`. There is no assignment expression, indexing, member access, tuple syntax, lambda syntax, or interpolation syntax.

## 3. Declarations, scopes, and functions

A source file may declare functions and module markers. A module marker is currently parsed and ignored; it does not create a module namespace. Function names must be unique within the file. Parameters must be unique within their function. `main()` is required and takes no arguments.

Parameters and functions may have primitive type annotations. Local declarations may also have annotations. Supported annotation names are `Number`, `String`, `Bool`, and `Nil`. Other type names are rejected by the bootstrap checker. Omitted parameter types are inferred from call arguments when all calls agree; an uncalled parameter or conflicting caller types is rejected. Unannotated function return types are inferred from return expressions and checked at call sites. Return expressions must infer to one consistent type. If return inference remains unresolved (for example, an unresolved recursive return cycle), compilation asks for an explicit return annotation. `Unknown` is only an internal inference state; the final body check rejects expressions whose types remain unresolved. The language still lacks aggregate, generic, reference, and other planned types, so it is not a complete language type system.

`let` creates an immutable binding; `var` creates a mutable binding. Reassignment requires a visible mutable binding and a value accepted by the type checker. A declaration cannot duplicate another declaration in the same scope. Nested blocks, each conditional branch, and each while-loop iteration establish lexical scopes. A nested scope may shadow an outer binding. A binding created in a nested scope is unavailable after that scope ends. Inner code can update a visible outer `var`.

Functions use positional arguments and are not overloaded. Recursion is allowed. Calls must resolve to a declared function and match its arity. `print` accepts zero or more arguments and prints each argument on its own line. Return annotations are optional; annotated non-`Nil` functions must return a value on every statically evident path. The analysis handles sequencing, blocks, and both branches of conditionals. It conservatively considers loops capable of falling through. Closures, defaults, named arguments, methods, and return-type inference are not implemented.

## 4. Types and expression behavior

The runtime values are `Number` (`f64`), `String` (UTF-8), `Bool`, and `Nil`. The checker infers these types from literals, local initializers, and function return expressions; checks explicit local/parameter/return annotations; checks function call arguments when parameter types are present; and verifies inferred or annotated function returns. It checks Boolean conditions, arithmetic operands, and compatible equality operands. Unknown types can pass some expression checks; therefore this subset is not sound for arbitrary programs with unannotated parameters.

`+` adds two numbers or concatenates two strings. `-`, `*`, `/`, and `%` operate on numbers. Ordering operators compare numbers. Equality and inequality require compatible operand types. `&&` and `||` require booleans and short-circuit in both the interpreter and IR VM. Unary `-` requires a number and `!` requires a boolean. There are no implicit cross-type conversions. Division or remainder by zero is a runtime error. The language has no specified integer overflow behavior because it has no integer type yet.

Truthiness is used by the interpreter's runtime helper: only `false` and `nil` are falsey. Static checking requires `Bool` for `if` and `while` conditions, so other values are rejected before normal execution.

## 5. Errors, execution, and resource bounds

Lexical, syntax, static-check, and runtime failures are reported as diagnostics and cause a nonzero CLI exit. The interpreter and IR VM impose a one-million-step execution budget and a bounded recursive call depth (64). Exhausting either bound is an error. The current runtime has no recoverable language-level error value, exception handling, stack trace, or cleanup/unwinding guarantee.

There is no language-level memory ownership or borrowing rule yet. The implementation uses Rust-managed memory for compiler/runtime data, but this does not establish safety guarantees for future compiled Averqun programs. Averqun code currently has no raw pointers, explicit allocation, destructors, or FFI.

## 6. Compilation model and tools

The implemented pipeline is UTF-8 source → lexer → recursive-descent parser → AST → name/mutability checks and bootstrap primitive type checks. The AST may be lowered to target-independent stack IR and executed in a reference VM. `run` uses the AST interpreter. `ir-run` executes lowered IR. These execution paths are not yet covered by comprehensive differential tests.

The CLI currently supports `run`, `check`, `tokens`, `ast`, `ir`, and `ir-run`; invoke a command with a `.avq` file. `ir` prints IR for inspection. No command emits a native object or executable. The repository has no package manifest, dependency resolver, standard-library distribution, formatter, documentation generator, LSP, or debugger.

## 7. Planned design decisions (not accepted syntax)

The following choices guide future RFCs and implementation; they are not features of version 0.1.0.

| Area | Planned decision | Current status |
| --- | --- | --- |
| Function overloading | Disallow overloading; one function name per module | Parser enforces file-local uniqueness |
| Modules | Explicit module identity, imports/exports, visibility, dependency graph; diagnose cycles unless an RFC permits them | `module` marker is ignored; no imports |
| Data types | Structs, enums, tuples, arrays/slices, maps, exhaustive `match` | Not parsed or checked |
| Generics and traits | Parametric generics with explicit trait constraints and compile-time resolution | Not implemented |
| Errors | `Result<T, E>` and propagation; panics reserved for unrecoverable defects | No language-level errors |
| Memory | Ownership and borrowing with static alias/lifetime checks and deterministic destruction | No ownership checker |
| Concurrency | Async tasks with structured lifetimes, cancellation, and explicit synchronization | Single-threaded execution |
| Standard library | Versioned core, collections, text, filesystem, process, time, network, JSON, logging, randomness, and test support | Only built-in `print` |
| FFI | Explicit unsafe boundary and documented ABI; safe wrappers where feasible | No FFI |
| Packages | Manifest plus lockfile, reproducible resolution, integrity verification, local and registry dependencies | No package manager |

Before any planned item becomes normative, its syntax, typing, runtime semantics, compatibility rules, tests, and implementation must land together. The implementation status and release gates are tracked in [implementation-status.md](implementation-status.md) and [release-readiness.md](release-readiness.md).

## 8. Compatibility and versioning

While the language is experimental, compiler behavior may change between minor versions and no source compatibility promise is made. Once a stable release policy exists, syntax or semantic breaking changes require a major version; compatible additions and fixes may use minor and patch versions respectively. The package ABI and native runtime ABI do not exist yet and must be versioned before native artifacts are considered stable.

## 9. Conformance examples

This is within the implemented subset:

```averqun
fn add(a: Number, b: Number) -> Number {
    return a + b
}

fn main() {
    let answer: Number = add(20, 22)
    print(answer)
}
```

This is intentionally outside the implemented subset (generic syntax is not recognized):

```averqun
fn identity<T>(value: T) -> T {
    return value
}
```

## 10. Experimental alternative syntax

The bootstrap grammar above records the original syntax retained by the parser for compatibility. An alternative form uses Averqun-specific vocabulary and call/declaration markers. This form is executable today but remains experimental while its grammar and migration policy are evaluated. New examples should prefer this form; the legacy form will not automatically be part of a stable release.

An initial, unimplemented syntax direction is a **weave** declaration with bracketed parameter clauses, `gives` for a result type, `kept`/`live` bindings, `:=` initialization, `when`/`otherwise`, `again while`, `yield`, and `show`:

```averqun
weave add[left: Number, right: Number] gives Number {
    yield left + right
}

weave main[] {
    kept answer: Number := add!{20, 22}
    show answer
}
```

Its implemented additions to the grammar are:

```ebnf
weave_function = "weave" identifier "[" [ parameters ] "]"
                 [ "gives" identifier ] block ;
weave_variable = ("kept" | "live") identifier [ ":" identifier ] ":=" expression [ ";" ] ;
weave_return   = "yield" [ expression ] [ ";" ] ;
weave_branch   = "when" expression block [ "otherwise" block ] ;
weave_loop     = "again" "while" expression block ;
weave_output   = "show" expression [ ";" ] ;
weave_call     = identifier "!" "{" [ arguments ] "}" ;
```

The alternative form keeps the same type and runtime semantics as the bootstrap form. `kept` is immutable; `live` is mutable. `yield` returns from a function. `when` requires a `Bool` condition, and `otherwise` is optional. `again while` has the same loop semantics as legacy `while`. `show` writes one value on its own output line.

The lexer and parser currently implement these declarations, bindings, returns, calls, output statements, conditionals, and loops. The existing type checker and both execution paths handle their semantics. Other syntax remains as documented in the bootstrap grammar. The parser currently permits legacy and alternative forms in the same file; the compiler does not yet enforce a language-version declaration. Before stabilization, settle whether to remove legacy spelling, provide an automated migration, or maintain both forms. No existing language's tutorial text, examples, standard-library naming, documentation wording, or implementation code should be copied into Averqun. Similarity of common programming concepts or short syntax tokens is not, by itself, a legal conclusion.

Distinctive syntax cannot by itself prevent legal disputes. The U.S. Copyright Office explains that copyright protects software expression but not ideas, systems, or methods of operation ([FAQ](https://copyright.gov/help/faq/faq-protect.html)); names may raise separate trademark issues. A separate public software project already uses the former name “Nexa” for a programming language at [nexa-lang.com](https://www.nexa-lang.com/). Averqun is the new working name, not a cleared mark. The preliminary web search found no obvious exact-name language or software project, but that does not establish availability. Before a public release, perform a professional name clearance in the intended jurisdictions; the [USPTO search guidance](https://www.uspto.gov/trademarks/search/federal-trademark-searching) explains that database searches cannot guarantee registrability. This is not a legal opinion.

## 11. Change control

Every syntax revision must update this specification, the grammar, examples, compiler behavior, tests, and migration notes together. A release may call a feature supported only after executable conformance tests and the implementation status agree. Proposed syntax and semantics stay labeled as planned until then.
