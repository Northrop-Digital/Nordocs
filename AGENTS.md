# AGENTS.md — northdoc

Guidance for AI agents working in this repository.

## What this is

northdoc is a Rust-native port-and-refine of a C# Typst document toolset. It
embeds the native `typst` crate (no external process), builds a single binary
`ndoc`, and turns Markdown/data into PDF via a fat-file (`.ndoc.typ`) model.

The C# reference implementation lives at `.reference/Typst`.
**Do not modify anything under `.reference/`** — it is read-only source material
to port from.

## Build / test / lint commands

```sh
cargo build            # must stay green
cargo test             # unit + insta snapshots + assert_cmd CLI tests
cargo clippy --all-targets   # lints; keep warnings at zero where practical
cargo fmt              # rustfmt (see rustfmt.toml, 100-col)
cargo tarpaulin        # coverage; config in tarpaulin.toml; 80% minimum (install: cargo install cargo-tarpaulin)
```

When you change snapshot-tested output, review and accept with
`cargo insta review` (do not blindly accept).

## ndoc CLI commands

```sh
ndoc build <file.ndoc.typ>    # compile fat file to PDF
ndoc render <file.typ>        # compile raw Typst to PDF
ndoc validate <file>          # validate .ndoc.typ or .md file; exits 1 with violations on failure
ndoc preview <file>           # render to temp PDF and open in OS default viewer
```

Set `NDOC_NO_OPEN=1` to skip viewer spawn in headless/CI environments (PDF is still
rendered and verified non-empty).

## Layout & where things go

- `src/cli/`         — clap derive commands + dispatch. New commands go here.
- `src/typst_world.rs` — the `typst::World` impl (highest-risk area; keep on the
  canonical typst API).
- `src/compiler.rs`  — `.typ` -> PDF wrapper.
- `src/markdown.rs`  — Markdown -> Typst (comrak).
- `src/fatfile/`     — compose/extract/hash of `.ndoc.typ` (STATE / TEMPLATE /
  DOCUMENT / IMAGES).
- `src/authoring/`   — transactional read-validate-write over fat files.
- `src/schema/`      — component/template input schemas + catalogue.
- `src/validation.rs` — schema-based validation for `.ndoc.typ` and `.md` documents.
- `src/model.rs`     — shared domain types.
- `tests/`           — integration tests.

## Conventions

- Errors: `thiserror` typed errors in `src/error.rs` for the library; `anyhow`
  with context at the CLI/app boundary.
- Keep `main.rs` a thin shell; real logic lives in library modules so it stays
  testable.
- The Typst version pins in `Cargo.toml` (`typst`, `typst-library`,
  `typst-syntax`, `typst-pdf`, `typst-kit`, `typst-assets`) must move together.
- Prefer snapshot tests (insta) for composed `.typ` output and CLI tests
  (assert_cmd) for the binary surface.

## Scope reminders (from the charter)

v1 is CLI-first and **PDF-only**. Out of scope: GUI, non-PDF output, plugin
system, and the AgentTools (MCP) programmatic surface (deferred). Do not build
those unless explicitly asked.

The defining success measure is **correct end-to-end output** with fidelity at
least equal to the C# tool.
