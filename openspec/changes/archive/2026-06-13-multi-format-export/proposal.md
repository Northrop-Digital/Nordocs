## Why

The library is PDF-only, but Typst can also export SVG and PNG, and downstream renderers need both (SVG for crisp, click-mappable on-screen preview; PNG for thumbnails and raster contexts). The compiler currently produces a `PagedDocument`, exports it to PDF, and immediately discards the document — so adding formats is purely a matter of branching the export step and retaining the document. This change also adds `sys.inputs` variable support, which the C# reference (`SetSysInputs`) already relies on and FFI parity will require.

## What Changes

- Refactor `nordocs-core::compiler` so compilation yields a retained `CompiledDoc { world, document }` value, and PDF/SVG/PNG become exporters over it. PDF output is byte-for-byte unchanged.
- Add `typst-svg` and `typst-render` to the workspace; SVG via `typst_svg::svg`/`svg_merged`, PNG via `typst_render::render(...).encode_png()`.
- Support multi-page documents: SVG/PNG produce one file per page (`out-1.svg`, `out-2.svg`, …) by default, or a single merged canvas with `--merged`.
- Add output-format selection to `render` and `build`: inferred from the `-o` extension when present, otherwise an explicit `--format pdf|svg|png` flag (`render`/`build` only). PNG resolution is controlled by `--dpi` (default 144).
- Add `sys.inputs` variable support to the core compile entry points (key/value pairs injected before compilation).
- `preview` remains PDF-only (explicit scope decision); it is untouched.

## Capabilities

### New Capabilities

<!-- None — this extends existing capabilities. -->

### Modified Capabilities

- `render-pipeline`: The embedded compiler gains SVG and PNG export and `sys.inputs` injection in addition to PDF; the document is retained as a reusable `CompiledDoc`.
- `cli-surface`: `render` and `build` gain `--format`, `--dpi`, and `--merged`, plus extension-inferred format selection and the per-page output-naming convention.

## Impact

- `crates/nordocs-core/Cargo.toml`: add `typst-svg`, `typst-render` (pinned 0.14).
- `crates/nordocs-core/src/compiler.rs`: introduce `CompiledDoc`; `compile()` returns it; `to_pdf`/`to_svg`/`to_png` exporters; `sys.inputs` plumbed through the world.
- `crates/nordocs-cli/src/cli`: `RenderArgs`/`BuildArgs` gain format/dpi/merged options and format-resolution logic.
- Tests: `insta` snapshots for SVG output; PNG asserted by decoded dimensions/non-empty bytes; CLI E2E for each format and multi-page naming. No external `typst` binary is invoked.
- `openspec/config.yaml`: revise the "v1 scope is CLI-first and PDF-only" convention to "PDF, SVG, and PNG".
