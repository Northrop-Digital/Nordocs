## 1. Wire up typst-ide

- [ ] 1.1 Add `typst-ide` (pinned 0.14) to the shared workspace dependency table and `nordocs-core`
- [ ] 1.2 `impl IdeWorld for NordocsWorld` with `upcast(&self) -> &dyn World { self }`
- [ ] 1.3 Confirm `cargo build -p nordocs-core` succeeds with the new dependency

## 2. Backward map: click → source

- [ ] 2.1 Add `CompiledDoc::jump_from_click(page_index, point) -> Option<Jump>` delegating to `typst_ide::jump_from_click`
- [ ] 2.2 Define a serialisable `Jump` enum: `File { path, offset, line, column }`, `Url { url }`, `Position { page, x_pt, y_pt }`
- [ ] 2.3 Resolve `FileId` → path string and byte `offset` → 1-based line/column via the source
- [ ] 2.4 Unit test: a known click on a fixture resolves to the expected source offset/line/column; a click on empty space returns `None`

## 3. Forward map and geometry

- [ ] 3.1 Add `CompiledDoc::jump_from_cursor(file, offset) -> Vec<Position>` via `typst_ide::jump_from_cursor`
- [ ] 3.2 Add `page_count()` and `page_size(i) -> (width_pt, height_pt)` accessors
- [ ] 3.3 Document the screen↔document transform (`point_pt = px / scale`, explicit page index) in the API docs
- [ ] 3.4 Unit test: cursor in a fixture maps to a non-empty set of on-page positions

## 4. CLI diagnostic subcommand

- [ ] 4.1 Add a hidden `ndoc jump <file> --page <n> --at <x>,<y> [--json]` subcommand (`#[command(hide = true)]`) to the CLI, compiling the file and calling `jump_from_click`
- [ ] 4.2 Emit the resolved `Jump` as the `--json` envelope and as human-readable text
- [ ] 4.3 CLI E2E (`assert_cmd`): `ndoc jump --json` on a fixture returns the expected file/line/column

## 5. Docs and checks

- [ ] 5.1 Document `source-mapping` in `README.md`/`AGENTS.md`, noting it is a primitive for downstream renderers (and that SVG carries no spans by itself)
- [ ] 5.2 Confirm no test invokes an external `typst` binary
- [ ] 5.3 Confirm `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`, and ≥ 80% coverage are green
