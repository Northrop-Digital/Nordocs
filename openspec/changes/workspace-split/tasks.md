## 1. Establish the workspace

- [ ] 1.1 Add a root workspace `Cargo.toml` with `members = ["crates/nordocs-core", "crates/nordocs-cli", "crates/nordocs-ffi"]` and a shared `[workspace.dependencies]` table for the pinned typst 0.14 stack, serde, anyhow, thiserror
- [ ] 1.2 Create `crates/nordocs-core/Cargo.toml` (name `nordocs-core`, lib) depending on the engine deps only (no `clap`)
- [ ] 1.3 Create `crates/nordocs-cli/Cargo.toml` (bin `ndoc`) depending on `nordocs-core` + `clap`
- [ ] 1.4 Create `crates/nordocs-ffi/Cargo.toml` (empty `cdylib` skeleton) depending on `nordocs-core`; `lib.rs` contains only a crate doc comment for now
- [ ] 1.5 Confirm `cargo build` succeeds for the empty workspace skeleton before moving code

## 2. Move the engine into nordocs-core (no behaviour change)

- [ ] 2.1 Move `compiler.rs`, `markdown.rs`, `model.rs`, `error.rs`, `typst_world.rs`, `validation.rs`, and the `fatfile/`, `schema/`, `authoring/`, `item/` module trees into `crates/nordocs-core/src/`, carrying their `#[cfg(test)]` modules
- [ ] 2.2 Write `crates/nordocs-core/src/lib.rs` re-exporting the modules and `Error`/`Result`, matching the current public API minus `cli`
- [ ] 2.3 Move engine-facing integration tests (`tests/markdown.rs`, `tests/fatfile.rs`, `tests/ndoc.rs`) under `crates/nordocs-core/tests/`
- [ ] 2.4 Confirm `cargo test -p nordocs-core` passes with no assertion changes

## 3. Move the CLI into nordocs-cli

- [ ] 3.1 Move `cli/mod.rs`, `cli/output.rs`, and `open_with_default_viewer` into `crates/nordocs-cli/src/`, plus `main.rs`
- [ ] 3.2 Repoint all `crate::` engine references to `nordocs_core::`
- [ ] 3.3 Move `tests/cli.rs` under `crates/nordocs-cli/tests/` and confirm `assert_cmd` targets the `ndoc` binary in the new location
- [ ] 3.4 Confirm `cargo test -p nordocs-cli` passes unchanged

## 4. Extract the service façade

- [ ] 4.1 For each `cmd_*` handler, split engine logic into a `nordocs-core` function returning a serialisable result struct; keep only file/stdin reading, `--json` rendering, and exit codes in the CLI adapter
- [ ] 4.2 Provide both a content form and a path form for compile/render operations (mirroring the reference's `CompileToPdf` / `CompileFileToPdf`)
- [ ] 4.3 Ensure no façade function calls `println!`, `eprintln!`, `std::process::exit`, or spawns a process
- [ ] 4.4 Re-run the full `tests/cli.rs` suite to prove behaviour is byte-for-byte unchanged

## 5. Workspace housekeeping

- [ ] 5.1 Update `tarpaulin.toml` to workspace mode and confirm aggregate line coverage ≥ 80%
- [ ] 5.2 Update `openspec/config.yaml` `context`: replace "single binary crate `nordocs`" with the three-crate workspace description
- [ ] 5.3 Update `AGENTS.md` / `CLAUDE.md` module-layout sections to reflect the workspace
- [ ] 5.4 Confirm `cargo build`, `cargo test`, `cargo clippy --all-targets`, and `cargo fmt --check` are green at the workspace root
