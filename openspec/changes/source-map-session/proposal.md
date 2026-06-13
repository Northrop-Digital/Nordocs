## Why

The headline downstream feature is tinymist-style click-to-source: a renderer shows the SVG, the user clicks a glyph, and the editor jumps to the Typst code that produced it. This is **not** an SVG property — the official `typst-svg` export carries no source spans. It comes from `typst-ide`, which maps a click point on a page `Frame` back to a source location, and the reverse (a source cursor to on-page positions). To support this, the library must retain the compiled document + world in a session and expose the jump operations as a binding-agnostic API the FFI can wrap.

## What Changes

- Implement the `typst_ide::IdeWorld` trait for `NordocsWorld` (one required method: `upcast`).
- Add jump operations to the retained `CompiledDoc`:
  - `jump_from_click(page, point) -> Jump` (backward map: click → `Jump::File { file, offset }`, `Jump::Url`, or `Jump::Position`).
  - `jump_from_cursor(file, offset) -> [Position]` (forward map: source cursor → on-page positions, for highlighting).
- Expose page geometry (`page_count`, `page_size` in `pt`) and a documented screen↔document coordinate convention so callers can convert UI pixels to a Typst `Point`.
- Resolve `Jump::File` byte offsets to line/column for caller convenience.
- Add a **hidden** `ndoc jump` diagnostic subcommand (`#[command(hide = true)]`) — exercises the API end-to-end for the CLI E2E suite and standalone debugging, without appearing in `ndoc --help`.

## Capabilities

### New Capabilities

- `source-mapping`: Bidirectional mapping between a rendered Typst document and its source — click-to-source and cursor-to-preview — over a retained compiled session.

### Modified Capabilities

- `cli-surface`: Adds the hidden `ndoc jump` diagnostic subcommand (invocable and testable, but not listed in top-level help).

## Impact

- `crates/nordocs-core/Cargo.toml`: add `typst-ide` (pinned 0.14).
- `crates/nordocs-core/src/typst_world.rs`: `impl IdeWorld for NordocsWorld`.
- `crates/nordocs-core/src/compiler.rs`: `CompiledDoc::jump_from_click` / `jump_from_cursor` / `page_count` / `page_size`; a serialisable `Jump`/`Position` result type plus offset→line/column resolution.
- `crates/nordocs-cli/src/cli`: `ndoc jump <file> --page <n> --at <x>,<y> [--json]`.
- Tests: unit tests resolving a known click on a fixture to the expected source offset; CLI E2E for `ndoc jump --json`. No external `typst` binary.
- Depends on `workspace-split` and `multi-format-export` (uses `CompiledDoc`).
