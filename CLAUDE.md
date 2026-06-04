# CLAUDE.md

Project-specific guidance for Claude Code. See `AGENTS.md` for the full agent
brief; this file is the short version and inherits everything there.

## TL;DR

- Rust crate `northdoc`, binary `ndoc`. Embeds the native Typst compiler;
  Markdown/data → Typst → PDF via the `.ndoc.typ` fat-file model.
- **Never touch `.reference/`** — read-only C# source to port from.
- Keep `cargo build`, `cargo test`, and `cargo clippy` green.
- v1 is CLI-first and PDF-only. No GUI / no MCP surface / no plugins yet.

## Quick commands

```sh
cargo build
cargo test
cargo clippy --all-targets
cargo fmt
```

See `AGENTS.md` for module layout, conventions, and scope guardrails.
