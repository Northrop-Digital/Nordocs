# AGENTS.md — nordocs

Guidance for AI agents working in this repository.

## What this is

nordocs is a Rust-native port-and-refine of a C# Typst document toolset. It
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

## Testing

### Mandatory rules

All six rules must be satisfied before landing any change:

a. **`cargo test` must pass** — run the full test suite before committing. Do not commit code that fails any test.

b. **Unit tests live in the same file as the code** — place `#[cfg(test)]` modules at the bottom of the source file; do not put unit tests in a separate `tests/` file.

c. **Snapshot tests via `insta` for Typst output** — every Markdown feature processed by `markdown.rs` (tables, task lists, footnotes, strikethrough, etc.) and every fat-file composition path must have a snapshot test using `insta::assert_snapshot!`.

d. **CLI E2E tests via `assert_cmd` for every command** — every public `ndoc` subcommand (`render`, `build`, `new`, `add`, `edit`, `validate`, `preview`, the `doc` subgroup, `component`, `item`, `template`, `image`) must have at least one `assert_cmd` test in `tests/cli.rs` covering a success path and a failure path.

e. **No test may invoke an external `typst` binary** — all rendering must go through the embedded `typst` compiler. Tests that shell out to an external process are prohibited.

f. **80% line coverage target** — `cargo tarpaulin` must report ≥ 80% overall line coverage. Config lives in `tarpaulin.toml`. Install: `cargo install cargo-tarpaulin`.

### Snapshot review policy

When snapshot tests fail because output has changed, review the diff with `cargo insta review` before accepting. **Blind acceptance (`cargo insta accept` without reviewing the diff) is prohibited.** Accepting a snapshot implies you have verified the new output is correct.

### Test layers

| Layer | Tool | Location | When to use |
|-------|------|----------|-------------|
| Unit | std `#[test]` | Same file as code (`#[cfg(test)]`) | Pure functions; domain logic; error paths |
| Snapshot | `insta` | `tests/snapshots/` | Typst output from `markdown.rs`; fat-file composition |
| CLI E2E | `assert_cmd` | `tests/cli.rs` | Binary surface; exit codes; stdout/stderr content |
| Smoke | `assert_cmd` + `#[ignore]` | `tests/cli.rs` | Release binary at `target/release/ndoc`; run with `cargo test -- --ignored release_smoke_test` |

### Test discipline

- Test behavior through public seams, not implementation internals.
- Mock only genuinely external boundaries (clock, OS, external network). Do not mock your own modules.
- Deterministic only: freeze time where needed; no ordering reliance; no real network calls.
- Before adding any test, ask "what regression would this catch?" — if there is no answer, skip it.

## ndoc CLI commands

The command surface is complete (no stubs). `--json` is a global flag on every
command — see `README.md` for the full flag/output reference.

```sh
# Render & build
ndoc render <file>            # compile .ncmp.typ / .ndoct.typ / .ndoc.typ to PDF/SVG/PNG (-o overrides; --format/--dpi/--merged)
ndoc build <file>             # compile .md or .ndoc.typ to PDF/SVG/PNG (--format pdf|svg|png, --dpi <n> [144], --merged)
ndoc validate <file>          # validate .ndoc.typ or .md; exits 1 with violations
ndoc preview <file>           # render .ndoc.typ or .md to temp PDF and open in OS viewer (PDF-only)
# Format: -o extension wins, else --format, else pdf (a mismatch is an error).
# SVG/PNG split a multi-page doc into <base>-1.<ext> … (bare <base>.<ext> when single-page); --merged writes one file.

# Fat-file entry authoring
ndoc new <path>               # create an empty .ndoc.typ document
ndoc add <doc> <name>         # add a named entry (--kind component|template, --content-file)
ndoc edit <doc> <name>        # replace a named entry's content (--content-file)

# Node-tree authoring (doc subgroup)
ndoc doc new <template>       # create a .ndoc.typ bound to a template (-o; refuses overwrite)
ndoc doc outline <doc>        # print the node tree (stable ids + component types)
ndoc doc add <doc> --type T   # mint a node (--parent/--before/--after, --inputs KEY=VALUE)
ndoc doc remove <doc> <id>    # remove a node (--with-children drops the subtree)
ndoc doc set <doc> [<id>]     # set a schema-validated input (--document, --key, --value)
ndoc doc schema <target>      # show declared inputs for a .ncmp.typ / .ndoct.typ file

# Library introspection
ndoc component schema <file>  # show one component's input schema
ndoc component list <dir>     # list every *.ncmp.typ in a directory
ndoc template show <id|path>  # show a template's document inputs + permitted components
ndoc item load <dir>          # summarise *.item.md collections
ndoc item validate <dir>      # validate items against sibling *.ncmp.typ schemas (exits 1 on issues)
ndoc image add <doc> <image>  # embed an image into a .ndoc.typ (deduped by blake3)

# Diagnostics (hidden from `ndoc --help`)
ndoc jump <file> --page <n> --at <x>,<y>   # map a page click back to its source location (source-mapping primitive)
```

Set `NDOC_NO_OPEN=1` to skip viewer spawn in headless/CI environments (PDF is still
rendered and verified non-empty).

## Layout & where things go

The project is a Cargo workspace of three crates under `crates/`:

**`crates/nordocs-core/` (rlib `nordocs_core`)** — the binding-agnostic engine; no
`clap`, no terminal output, no process/viewer side effects.

- `src/service.rs`   — the binding-agnostic **service façade** (`core-api`):
  structured-result compile/render/convert operations called by the CLI and the
  FFI. New cross-binding operations go here.
- `src/typst_world.rs` — the `typst::World` impl (highest-risk area; keep on the
  canonical typst API).
- `src/compiler.rs`  — `.typ` -> PDF/SVG/PNG wrapper (the retained `CompiledDoc`).
- `src/markdown.rs`  — Markdown -> Typst (comrak).
- `src/fatfile/`     — compose/extract/hash of `.ndoc.typ` (STATE / TEMPLATE /
  DOCUMENT / IMAGES).
- `src/authoring/`   — transactional read-validate-write over fat files.
- `src/schema/`      — component/template input schemas + catalogue.
- `src/validation.rs` — schema-based validation for `.ndoc.typ` and `.md` documents.
- `src/model.rs`     — shared domain types.
- `tests/`           — engine integration + snapshot tests.

**`crates/nordocs-cli/` (binary `ndoc`)** — a thin adapter over the façade.

- `src/cli/`         — clap derive commands + dispatch. New commands go here.
- `src/main.rs`      — entrypoint shell (`mod cli;`, parse, dispatch, exit code).
- `tests/cli.rs`     — `assert_cmd` E2E for the binary surface.

**`crates/nordocs-ffi/` (cdylib)** — the .NET/C-ABI binding over the façade
(interoptopus-annotated exports, panic-guarded boundary, generated C# bindings).

## Conventions

- Errors: `thiserror` typed errors in `crates/nordocs-core/src/error.rs` for the
  library; `anyhow` with context at the CLI/app boundary.
- Keep `main.rs` and the `cmd_*` handlers thin: engine logic lives in
  `nordocs-core` (the `service` façade + modules) so it stays testable and is
  reusable by the FFI.
- The Typst version pins live in the root `[workspace.dependencies]` table
  (`typst`, `typst-library`, `typst-syntax`, `typst-pdf`, `typst-svg`,
  `typst-render`, `typst-ide`, `typst-kit`, `typst-assets`) and must move together.
- Prefer snapshot tests (insta) for composed `.typ` output and CLI tests
  (assert_cmd) for the binary surface.

## Scope reminders (from the charter)

v1 is CLI-first and exports **PDF, SVG, and PNG** (`render`/`build`; `preview`
stays PDF-only). It also exposes **`source-mapping`**: bidirectional
click-to-source / cursor-to-preview over a retained compiled session
(`CompiledDoc::jump_from_click` / `jump_from_cursor` plus page geometry). This is
a *primitive* for downstream renderers (the .NET/FFI consumer), not an end-user
feature — it comes from `typst-ide` walking the laid-out frame's spans, and SVG
carries no source spans by itself. The hidden `ndoc jump` command exercises it.
Out of scope: GUI, HTML export, plugin system, and the AgentTools (MCP)
programmatic surface (deferred). Do not build those unless explicitly asked.

The defining success measure is **correct end-to-end output** with fidelity at
least equal to the C# tool.
