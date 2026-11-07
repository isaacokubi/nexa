# Getting Started with Averqun (bootstrap)

Install Rust stable from https://rustup.rs, then from the repository root:

```sh
cargo build --workspace
cargo test --workspace
cargo run -p averqun -- run examples/hello.avq
```

Commands:

- `averqun check FILE`: lex and parse (does not perform static type checking).
- `averqun run FILE`: interpret and execute `main()`.
- `averqun tokens FILE`: print lexical tokens and source locations.
- `averqun ast FILE`: print parsed AST.

Example:

```averqun
fn add(a, b) { return a + b }
fn main() {
  let total = add(20, 22)
  print(total)
}
```

Current limitations: no static type checker, no native executable output, no loops or collections, no ownership/borrowing, and no production safety/performance guarantees. See specification and roadmap.