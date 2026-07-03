## Context

`nordocs` is currently one crate: `src/lib.rs` exposes every module including `pub mod cli`, and `src/main.rs` is the `ndoc` binary. Because `cli` is part of the library, `clap` is a library dependency and the engine cannot be linked without it. The forthcoming FFI (`dotnet-ffi`) and a possible WASM shell need the engine *without* the CLI surface.

The C# reference models the right separation: `Common.Typst.csproj` (engine, references `TypstSharp`) and `Common.Typst.CLI.csproj` (executable, references the engine + `System.CommandLine`). The CLI project is a thin shell; all real work lives in the engine's services, which return structured results (`AuthoringResult`, `BuildResult`, `DocumentOutline`, …).

This change mirrors that separation in Rust and pulls the operation logic currently embedded in `cmd_*` into reusable core functions.

## Goals / Non-Goals

**Goals:**
- A Cargo workspace with `nordocs-core` (engine), `nordocs-cli` (binary), and a `nordocs-ffi` skeleton.
- A binding-agnostic façade in `nordocs-core`: every operation the CLI performs is available as a function returning structured data, free of stdout/stderr/exit/viewer side effects.
- Byte-for-byte identical `ndoc` behaviour, proven by the existing `assert_cmd` E2E suite passing unchanged.

**Non-Goals:**
- Any new command, flag, or output format (those are later changes).
- Implementing the FFI surface (only the empty `nordocs-ffi` crate is created here).
- Changing the embedded Typst world, fat-file format, or schema logic.

## Decisions

### Three crates, not two

`nordocs-ffi` must be a separate `cdylib` crate (distinct `crate-type`, `catch_unwind` boundary, `panic` strategy concerns) so it cannot share a manifest with the `ndoc` binary. Given a workspace is required for that, extracting `nordocs-cli` as its own bin crate is nearly free and keeps `clap` out of the engine. Alternative considered: one crate with `crate-type = ["rlib", "cdylib"]` — rejected because it forces FFI concerns and CLI concerns into the same compilation unit and keeps `clap` in the engine.

### Façade shape: structured results, no I/O

Each `cmd_*` is split into (a) a core function in `nordocs-core` that takes owned inputs and returns a `Result<T, nordocs_core::Error>` where `T` is a serialisable struct, and (b) a CLI adapter that reads files/stdin, calls the core function, and renders output. File reading stays in the CLI for path-based commands; the core operates on content + already-resolved inputs where practical, so the same functions serve the FFI (which passes content across the boundary, not paths). Where the reference resolves paths internally (e.g. `CompileFileToPdf`), the core offers both a content form and a path form.

### Module ownership

`compiler`, `markdown`, `fatfile`, `schema`, `authoring`, `item`, `validation`, `model`, `typst_world`, `error` move to `nordocs-core`. `cli` (clap structs + `cmd_*` + `output` envelope + `open_with_default_viewer`) moves to `nordocs-cli`. `nordocs-core` re-exports `Error`/`Result` as today.

### Test placement

Unit tests (`#[cfg(test)]`) travel with their source files into `nordocs-core`. Integration suites (`tests/cli.rs`, `tests/markdown.rs`, `tests/fatfile.rs`, `tests/ndoc.rs`) are distributed to the crate they exercise: CLI E2E to `nordocs-cli`, engine snapshot/behaviour tests to `nordocs-core`. `cargo test` at the workspace root runs all of them; coverage threshold (≥ 80%) is enforced workspace-wide.

## Risks / Trade-offs

- **Large mechanical diff** → Moving files risks accidental behaviour drift. Mitigation: the `assert_cmd` E2E suite is the safety net; it must pass unchanged before and after. Do the move first with zero edits, then refactor the façade in a second commit.
- **Façade boundary churn** → Picking the wrong content-vs-path split could force a later reshuffle when the FFI lands. Mitigation: `dotnet-ffi` is designed in parallel (see that change's design); the façade signatures are chosen to satisfy both callers now.
- **`tarpaulin` config** → Coverage config (`tarpaulin.toml`) is single-crate today. Mitigation: update it to workspace mode and confirm the aggregate still reports ≥ 80%.
- **Workspace context in config.yaml** → The `context` block claims "single binary crate". Mitigation: update the wording in this change; defer the "PDF-only" revision to `multi-format-export` to avoid overlapping edits.
