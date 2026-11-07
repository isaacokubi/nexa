# NEXA — Experimental Programming Language

NEXA is an experimental programming-language and compiler project being developed in Rust.

## Planned compiler architecture

The development work is focused on a small, inspectable language toolchain:

- Lexical analysis and parsing.
- Abstract syntax tree and source-aware diagnostics.
- Semantic analysis and name/binding validation.
- Program execution through an interpreter.
- A reference intermediate-representation virtual machine.

## Project status

**Pre-release / active development.** The current public `main` branch contains only this project overview; the compiler source code and reproducible build instructions have not yet been published to this repository. Do not treat this repository as an installable compiler release.

Once the implementation is pushed, this README should be expanded with the language grammar, supported types and statements, examples, compiler architecture, exact build commands and test results.

## Technology

- Rust
- Cargo workspace tooling (implementation publication pending)

## Contribution and release policy

NEXA is not yet accepting external contributions against a stable language specification. Public release claims will be added only after the source tree, automated tests and usage examples are available in this repository.
