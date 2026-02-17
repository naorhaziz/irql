# Contributing to IRQL

Thank you for your interest in contributing! Here's how to get started.

## Getting started

1. **Fork and clone** the repository
2. **Install Rust nightly** — the `alloc` feature requires it:
   ```sh
   rustup toolchain install nightly
   ```
   The repo includes a `rust-toolchain.toml` that selects nightly automatically.

## Building

```sh
# Build all crates (including alloc)
cargo build --workspace

# Build core crates only (works on stable)
cargo +stable build -p irql_core -p irql_macro -p irql
```

## Running examples

```sh
cargo run -p basic
cargo run -p struct_example
cargo run -p function_traits
cargo run -p alloc_example
```

## Checks before submitting a PR

```sh
# Format
cargo fmt --workspace

# Clippy (treat warnings as errors)
cargo clippy --workspace -- -D warnings

# Build everything
cargo build --workspace

# Run examples
cargo run -p basic
cargo run -p struct_example
cargo run -p function_traits
cargo run -p alloc_example

# Run tests (including doc tests)
cargo test --workspace
```

## Guidelines

- **Open an issue first** to discuss non-trivial changes before submitting a PR.
- **Keep commits focused** — one logical change per commit.
- **Add documentation** for new public items (`///` doc comments).
- **Add tests** where feasible — `trybuild` for macro compile-error tests,
  unit tests for logic, doc tests for examples.
- **Follow existing patterns** — match the style and structure of surrounding code.

## Architecture

```
irql              ← public facade crate (re-exports everything)
├── irql_core     ← IRQL level types, hierarchy traits, function traits,
│                   SafeToDropAt* auto traits
├── irql_macro    ← #[irql] proc macro + call_irql! rewriter
└── irql_alloc    ← IrqlBox, IrqlVec, pool allocator (optional, nightly)
```

See the [README](README.md) for a detailed architecture overview.

## License

By contributing, you agree that your contributions will be dual-licensed under
MIT and Apache 2.0, matching the project's existing license.
