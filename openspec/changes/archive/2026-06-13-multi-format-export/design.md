## Context

`compiler::compile_world()` runs `typst::compile::<PagedDocument>(world)`, exports with `typst_pdf::pdf()`, and drops both the document and the world. SVG and PNG are alternative exporters of the *same* `PagedDocument`, so the only structural change needed is to stop discarding it. `typst-svg` 0.14.2 (already in the cargo cache) exposes `svg(&Page) -> String` and `svg_merged(&PagedDocument, padding) -> String`; `typst-render` exposes `render(&Page, pixels_per_pt) -> Pixmap` with `Pixmap::encode_png()`.

PDF is one file; SVG and PNG are inherently per-page. The reference's native binding already returns a `CompileResult.Buffers[]` list, so a multi-buffer result is the shape consumers expect. The reference also injects template variables via `SetSysInputs`, which the current Rust world does not support.

This change depends on `workspace-split` (it edits `nordocs-core::compiler` and the `core-api` façade).

## Goals / Non-Goals

**Goals:**
- A retained `CompiledDoc` with `to_pdf` / `to_svg(page)` / `to_png(page, dpi)` exporters; identical PDF bytes to today.
- `render` and `build` can emit SVG/PNG, selected by `-o` extension or `--format`, with `--dpi` and `--merged`.
- `sys.inputs` key/value injection on the core compile entry points.

**Non-Goals:**
- Changing `preview` (stays PDF-only by decision).
- Click-to-source / span data (that is `source-map-session`; this change only retains the document it will need).
- HTML export, or partial-frame export.

## Decisions

### `CompiledDoc` as the retained unit
`compile()` returns `CompiledDoc { world: NordocsWorld, document: PagedDocument }`. Exporters are methods. This single type is reused by `source-map-session` (jump) and `dotnet-ffi` (session handle), so its shape is chosen with those in mind: it owns the world (needed for `jump_from_click`) and the document (needed for all exports and jumps).

### Format selection precedence
1. If `-o <path>` is given and its extension is `.pdf`/`.svg`/`.png`, that wins.
2. Else if `--format` is given, use it.
3. Else default to `pdf`.
`--format` is offered on `render` and `build` (which can lack `-o`). Conflicting `-o out.svg --format png` is a hard error rather than a silent precedence surprise.

### Multi-page output naming
For SVG/PNG without `--merged`, an N-page document writes `<base>-1.<ext> … <base>-N.<ext>`. A single-page document writes `<base>.<ext>` (no `-1` suffix) to avoid surprising the common case. `--merged` writes one `<base>.<ext>` using `svg_merged` (SVG) or a vertically stacked pixmap (PNG). The chosen convention is logged so truncation/expansion is never silent.

### PNG resolution unit
`typst-render` takes `pixels_per_pt`. The CLI exposes `--dpi` (default 144) and converts: `pixels_per_pt = dpi / 72.0`. DPI is more intuitive for users than a raw scale factor; 144 (2×) is a sensible on-screen default.

### sys.inputs plumbing
The core compile entry points accept an ordered list of `(String, String)` pairs injected as `sys.inputs` before compilation, matching `TypstCompiler.SetSysInputs(Dictionary<string,string>)`. The CLI does not yet surface a flag for these (no reference CLI flag exists); they are exposed through the façade for the FFI. A CLI flag can be added later if needed.

## Risks / Trade-offs

- **PDF byte drift** → Refactoring the compile path could change PDF output. Mitigation: keep the `typst_pdf::pdf()` call and options identical; assert the existing PDF tests still pass.
- **PNG snapshot fragility** → Raster bytes vary across platforms/font rasterisers. Mitigation: assert PNG by decoded width/height and non-emptiness, not by byte snapshot; reserve `insta` snapshots for SVG text.
- **SVG snapshot churn across typst versions** → SVG output is detailed. Mitigation: snapshot a small, stable fixture and review diffs via `cargo insta review`; never blind-accept.
- **Merged PNG memory** → Stacking many pages into one pixmap can be large. Mitigation: document the cost; `--merged` is opt-in and per-page is the default.
