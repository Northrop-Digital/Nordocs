## 1. Retain the compiled document

- [x] 1.1 Add `typst-svg` and `typst-render` (pinned 0.14) to the workspace dependency table and `nordocs-core`
- [x] 1.2 Introduce `CompiledDoc { world, document }` in `nordocs-core::compiler`; add `compile(source, sys_inputs)` returning it
- [x] 1.3 Reimplement `to_pdf()` as a method on `CompiledDoc`, preserving the existing `typst_pdf::pdf()` call and options exactly
- [x] 1.4 Confirm all existing PDF tests pass unchanged (byte-for-byte where asserted)

## 2. SVG and PNG exporters

- [x] 2.1 Add `to_svg(page)` (via `typst_svg::svg`) and `to_svg_merged()` (via `typst_svg::svg_merged`)
- [x] 2.2 Add `to_png(page, dpi)` via `typst_render::render` + `Pixmap::encode_png`, converting `pixels_per_pt = dpi / 72.0`
- [x] 2.3 Add `to_png_merged(dpi)` stacking pages into a single pixmap
- [x] 2.4 Add `page_count()` accessor on `CompiledDoc`
- [x] 2.5 `insta` snapshot test for SVG of a stable fixture; PNG tests assert decoded dimensions + non-empty bytes

## 3. sys.inputs support

- [x] 3.1 Plumb an ordered `(String, String)` list into `NordocsWorld` as `sys.inputs` before compilation
- [x] 3.2 Expose `sys_inputs` on the façade compile entry points (content and path forms)
- [x] 3.3 Unit test: a document reading `sys.inputs.foo` reflects the injected value

## 4. CLI format selection

- [x] 4.1 Add `--format pdf|svg|png`, `--dpi <n>` (default 144), and `--merged` to `RenderArgs` and `BuildArgs`
- [x] 4.2 Implement format-resolution precedence: `-o` extension > `--format` > default `pdf`; error on `-o`/`--format` conflict
- [x] 4.3 Implement multi-page output naming (`<base>-N.<ext>`; bare `<base>.<ext>` for single page; `--merged` → one file) and `log`/print the chosen convention
- [x] 4.4 Wire `render` and `build` adapters to the new `CompiledDoc` exporters; leave `preview` untouched (PDF-only)

## 5. Tests and docs

- [x] 5.1 CLI E2E (`assert_cmd`): `render`/`build` to `.pdf`, `.svg`, `.png`; `--format`; `--dpi`; `--merged`; multi-page naming; conflict error path
- [x] 5.2 Confirm no test invokes an external `typst` binary
- [x] 5.3 Update `README.md` (flags, default paths, per-page naming) and `AGENTS.md`
- [x] 5.4 Update `openspec/config.yaml` convention from "PDF-only" to "PDF, SVG, and PNG"
- [x] 5.5 Confirm `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`, and ≥ 80% coverage are green
