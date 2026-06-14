## Context

tinymist/typst-preview implement click-to-source through `typst-ide`, not through annotated SVG. The relevant API (typst-ide 0.14.2):

```rust
pub fn jump_from_click(world: &dyn IdeWorld, document: &PagedDocument,
                       frame: &Frame, click: Point) -> Option<Jump>
pub enum Jump { File(FileId, usize), Url(Url), Position(Position) }
```

`jump_from_click` walks the page `Frame` (which carries `Span`s from layout) to find what was clicked. `Jump::File(FileId, usize)` gives a source file and byte offset; `Jump::Url` a hyperlink; `Jump::Position` an internal jump (e.g. a ref/outline target). The reverse, `jump_from_cursor`, maps a source position to on-page `Position`s for preview highlighting.

`jump_from_click` takes `&dyn IdeWorld`, a supertrait of `World` whose only required method is `upcast(&self) -> &dyn World` (trait upcasting is still unstable, hence the shim). `NordocsWorld` already implements `World`, so the addition is a one-liner; `packages`/`files` have defaults and are not needed for jumps.

This change consumes the `CompiledDoc { world, document }` introduced by `multi-format-export`.

## Goals / Non-Goals

**Goals:**
- Backward map (click → source) and forward map (cursor → preview) as methods on `CompiledDoc`.
- A coordinate contract precise enough that a downstream renderer can convert a UI click to the correct page-local `Point`.
- Serialisable result types so the FFI can marshal them without re-deriving anything.

**Non-Goals:**
- Span-annotated SVG (the typst.ts approach) — not pursued; the coordinate-query model is what the C#/FFI consumer wants.
- A live-reloading preview server or websocket protocol (that would be a separate, larger effort).
- Editor integration (LSP) — out of scope; this exposes primitives, not an editor.

## Decisions

### Coordinate model: page-local points in `pt`
`jump_from_click` wants a `Point` in the page's coordinate space (Typst `Abs`, i.e. points). The library exposes `page_count()` and `page_size(i) -> (width_pt, height_pt)`. The documented transform for a renderer showing page *i* scaled by `s` (rendered pixels per pt) is `point_pt = (px / s, py / s)`, with the caller responsible for choosing the page and subtracting any inter-page gutter. The library takes an explicit `(page_index, Point)` and does not guess which page a global coordinate falls in — that keeps the contract unambiguous across merged vs per-page layouts.

### Serialisable result types
`Jump` is re-expressed as a serde-friendly enum: `File { path, offset, line, column }`, `Url { url }`, `Position { page, x_pt, y_pt }`. The library resolves `FileId` to a path string and `offset` to 1-based `line`/`column` via the source, so neither the CLI nor the FFI needs the raw `FileId`. `jump_from_cursor` returns `Vec<Position>`.

### `IdeWorld` shim
`impl IdeWorld for NordocsWorld { fn upcast(&self) -> &dyn World { self } }`. Defaults for `packages`/`files` are accepted; they only enhance autocompletion, which is irrelevant to jumps.

### `ndoc jump` is a hidden diagnostic, not a product feature
The CLI subcommand exists to exercise and snapshot the API (`ndoc jump fixture.typ --page 1 --at 120,80 --json`) and to debug coordinate issues in isolation from the .NET layer. The intended end-user workflow is the in-process FFI, not the CLI, so the command is marked `#[command(hide = true)]`: it is fully invocable and testable via `assert_cmd`, and `ndoc jump --help` still works, but it does not appear in `ndoc --help`. This keeps the capability testable without advertising a command whose real audience is the FFI. Alternatives considered: a visible subcommand (rejected — grows the public surface with a command users won't use) and a test-only Rust harness (rejected — loses the standalone debugging path and a CLI contract).

## Risks / Trade-offs

- **Coordinate convention misuse** → A downstream renderer that mis-handles scale or page offset will jump to the wrong place. Mitigation: document the transform precisely, expose page sizes in `pt`, and demand explicit `(page, point)` rather than a global coordinate.
- **`Jump::Position`/`Url` handling** → Consumers may only expect `File`. Mitigation: model all three variants in the serialisable type and document each so the FFI surface is complete from day one.
- **Span availability** → Synthetic content (no source span) yields `None`. Mitigation: treat `None` as "no jump target" and test that path explicitly.
- **typst-ide version coupling** → `typst-ide` must track the pinned 0.14 stack. Mitigation: pin it in the shared workspace dependency table alongside the other typst crates.
