# CLAUDE.md

Project-specific guidance for Claude Code. See `AGENTS.md` for the full agent
brief; this file is the short version and inherits everything there.

## TL;DR

- Rust crate `nordocs`, binary `ndoc`. Embeds the native Typst compiler;
  Markdown/data → Typst → PDF via the `.ndoc.typ` fat-file model.
- **Never touch `.reference/`** — read-only C# source to port from.
- Keep `cargo build`, `cargo test`, and `cargo clippy` green.
- v1 is CLI-first and PDF-only. No GUI / no MCP surface / no plugins yet.
- The `ndoc` command surface is **complete** (no stubs): `render`, `build`,
  `new`/`add`/`edit`, `validate`, `preview`, the `doc` authoring subgroup, plus
  `component`/`item`/`template`/`image`. Extend existing commands rather than
  re-scaffolding. See `README.md` for flags, default output paths, and the
  `--json` envelope; `AGENTS.md` for the full command list and module layout.

## Quick commands

```sh
cargo build
cargo test
cargo clippy --all-targets
cargo fmt
```

See `AGENTS.md` for module layout, conventions, and scope guardrails.

## Testing

All six rules must be satisfied before landing any change:

- `cargo test` passes — run the full suite before every commit.
- Unit tests live in the same file as the code (`#[cfg(test)]` module at the bottom of each source file).
- Snapshot tests via `insta` for all Typst output (`markdown.rs` features, fat-file composition).
- CLI E2E tests via `assert_cmd` for every `ndoc` subcommand in `tests/cli.rs` (success + failure paths).
- No test may invoke an external `typst` binary — use the embedded compiler only.
- `cargo tarpaulin` reports ≥ 80% line coverage (config in `tarpaulin.toml`).

Snapshot policy: always review diffs with `cargo insta review` before accepting. Blind acceptance is prohibited.

See `AGENTS.md` Testing section for the full test-layer table and smoke test (`cargo test -- --ignored release_smoke_test`) instructions.
