# Rash

Rash is a visual server application builder.

This repository contains:
- Rust workspace crates for spec parsing, validation, and IR (`crates/`)
- CLI prototype (`rash-cli/`)
- Spec fixtures (`fixtures/`)
- Architecture and roadmap docs (`docs/`)

## Quick start

Build and test:

```bash
cargo test --workspace
```

Run the CLI (dev):

```bash
cargo run -p rash-cli -- --help
```

Docs:
- `docs/README.md`
