## Why

Adding a .NET FFI (and, later, a WASM shell) means the engine must be consumable as a library by callers that have no interest in `clap`, terminal output, or spawning an OS PDF viewer. Today the `cli` module lives *inside* the library (`pub mod cli` in `lib.rs`), so every consumer drags in argument parsing and `std::process::Command`. The C# reference avoided exactly this by separating `Common.Typst` (engine) from `Common.Typst.CLI` (executable).

This change restructures `nordocs` into a Cargo workspace and extracts a binding-agnostic **service façade** from the `cmd_*` handlers, so the CLI and the upcoming FFI both call the same core operations instead of duplicating logic. It is foundational: the multi-format, source-map, and FFI changes all sit on top of it.

## What Changes

- Convert the single crate into a Cargo workspace with three member crates:
  - `nordocs-core` (rlib) — the engine: `compiler`, `markdown`, `fatfile`, `schema`, `authoring`, `item`, `validation`, `model`, `typst_world`. No `clap`, no viewer-spawn.
  - `nordocs-cli` (bin `ndoc`) — `clap` definitions, the OS-viewer `preview` logic, and `cmd_*` dispatch, as a thin adapter over the façade.
  - `nordocs-ffi` (cdylib) — created as an empty skeleton in this change; populated by `dotnet-ffi`.
- Extract a **service façade** in `nordocs-core`: each authoring/render/validate/introspection operation becomes a function returning structured data (mirroring the C# `AuthoringResult` / `BuildResult` / `DocumentOutline` pattern), with no `println!`/`process::exit`/file-viewer side effects.
- Rewrite `cmd_*` handlers to call the façade and own only presentation (`--json` envelope, human output, exit codes).
- No change to the `ndoc` command surface, flags, output paths, or behaviour — this is a structural refactor verified by the existing CLI E2E suite.

## Capabilities

### New Capabilities

- `core-api`: The binding-agnostic service façade exposed by `nordocs-core` — structured-result operations callable by any front end (CLI, FFI, WASM) without I/O or process side effects.

### Modified Capabilities

<!-- None — the `ndoc` command surface is behaviourally unchanged. The new contract that the CLI performs no engine logic of its own is captured by the `core-api` "thin adapter" requirement rather than a `cli-surface` delta. -->

## Impact

- Repository layout: `src/` is reorganised under `crates/nordocs-core/`, `crates/nordocs-cli/`, `crates/nordocs-ffi/`; a workspace `Cargo.toml` is added at the root.
- `Cargo.toml`: split into a workspace manifest plus per-crate manifests; `clap`/viewer deps move to `nordocs-cli`.
- Tests: unit tests move with their source files; `tests/cli.rs` retargets the `nordocs-cli` binary; no test assertions change.
- `openspec/config.yaml`: the "single binary crate" wording in `context` is updated to describe the workspace; the "PDF-only" note is left for `multi-format-export` to revise.
- No behavioural change to any `ndoc` command.
