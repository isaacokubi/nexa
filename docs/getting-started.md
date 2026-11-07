# Getting Started with NEXA (bootstrap)

Install Rust stable from https://rustup.rs, then from the repository root:

```sh
cargo build --workspace
cargo test --workspace
cargo run -p nexa -- run examples/hello.nexa
```

Commands:

- `nexa check FILE`: lex and parse (does not perform static type checking).
- `nexa run FILE`: interpret and execute `main()`.
- `nexa tokens FILE`: print lexical tokens and source locations.
- `nexa ast FILE`: print parsed AST.

Example:

```nexa
fn add(a, b) { return a + b }
fn main() {
  let total = add(20, 22)
  print(total)
}
```

Current limitations: no static type checker, no native executable output, no loops or collections, no ownership/borrowing, and no production safety/performance guarantees. See specification and roadmap.