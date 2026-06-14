# nordocs

A Rust-native re-implementation of the C# Typst document toolset. nordocs
**embeds the native Typst compiler** (no external process), ships as a single
binary (`ndoc`), and exposes a refined, CLI-first surface for turning
Markdown/data into PDF documents.

## Why

The previous C# tool shelled out to an external Typst binary, carried an
organically-grown CLI with cruft, and paid .NET startup cost on every
short-lived invocation. nordocs is a **port-and-refine**: same core
capabilities, native embedded Typst, faster startup/render, cleaner commands.

## Status

The full v1 CLI surface is implemented (no stubs): render/build, fat-file and
node-tree authoring, and component/item/template/image introspection. See
[Command reference](#command-reference) below.

## Capabilities (v1 target)

- Markdown/data → Typst → **PDF** generation pipeline (embedded Typst).
- A reusable **component/template library** (`.ncmp.typ`, `.ndoct.typ`,
  `.ntheme.typ`).
- The self-contained **fat file** (`.ndoc.typ`): STATE / TEMPLATE / DOCUMENT /
  IMAGES in one file.
- Document **validation** and **preview** before final build.
- **Source mapping** (`source-mapping`): bidirectional click-to-source / cursor-to-preview
  over a retained compiled session, for downstream renderers.

Out of scope for v1: GUI, a plugin system, and the AgentTools (MCP) programmatic
surface (deferred).

### Source mapping (`source-mapping`)

A *primitive* for downstream renderers (the in-process .NET/FFI consumer, an
editor preview), not an end-user feature. Given a compiled session, it maps a
page click back to the Typst source location that produced the glyph
(click-to-source) and a source cursor forward to the on-page positions it
rendered to (cursor-to-preview), plus page geometry in points.

This comes from `typst-ide`, which walks the laid-out page frame's spans — **it
is not an SVG property**. The official SVG export carries no source spans by
itself, so a renderer must keep the compiled session and query it by
coordinate. Coordinates are page-local points (`pt`) from the page's top-left;
a renderer at scale `s` (pixels per point) converts a pixel click with
`point_pt = (px / s, py / s)` and picks the page explicitly. The hidden
`ndoc jump` command (see Diagnostics) exercises this end-to-end.

## Build & run

```sh
cargo build              # debug build
cargo build --release    # single optimized binary at target/release/ndoc
cargo test               # unit + snapshot (insta) + CLI (assert_cmd) tests
cargo clippy             # lints
cargo fmt                # format

./target/release/ndoc --help
```

## Install

```sh
cargo install --path .   # installs `ndoc` to ~/.cargo/bin/
ndoc --help
```

`cargo install` copies the release binary to `~/.cargo/bin/ndoc`. Make sure
`~/.cargo/bin` is on your `PATH` (added automatically by `rustup`).

## Command reference

`--json` is a **global** flag accepted by every command. Without it, commands
print human-readable output to stdout; with it, they emit a single JSON envelope
(see [JSON envelope](#json-envelope)). Any failure exits non-zero with an
actionable message (the JSON error envelope under `--json`).

### Render & build

| Command | Accepts | Default output | Notes |
|---------|---------|----------------|-------|
| `ndoc render <input> [-o <out>]` | `.ncmp.typ`, `.ndoct.typ`, `.ndoc.typ` (never a bare `.typ`) | input with the recognised suffix replaced by the format extension (`.pdf` by default) | `-o`/`--output` overrides the path; a recognised `.pdf`/`.svg`/`.png` extension also selects the format. `--format pdf\|svg\|png`, `--dpi <n>` (PNG resolution, default `144`), `--merged`. `data: {"output": "<path>", "outputs": ["<path>", ...]}`. |
| `ndoc build <input>` | `.md`, `.ndoc.typ` | input with extension replaced by the format extension (`.pdf` by default) | Composes/converts then compiles. `--format pdf\|svg\|png`, `--dpi <n>` (default `144`), `--merged`. `data: {"output": "<path>", "outputs": ["<path>", ...]}`. |
| `ndoc validate <input>` | `.ndoc.typ`, `.md` | — | Prints each violation as `location: message`; **exits 1** if any. `data: {"violations": [{"location","message"}]}`. |
| `ndoc preview <input>` | `.ndoc.typ`, `.md` | temp PDF in the OS temp dir | Opens the OS default viewer. Set `NDOC_NO_OPEN=1` to skip the viewer (PDF still rendered + verified non-empty). `data: {"preview_path": "<path>"}`. **PDF-only by design.** |

**Output format selection** (`render`/`build`): a recognised `-o` extension
(`.pdf`/`.svg`/`.png`) wins; otherwise `--format`; otherwise `pdf`. A `-o`
extension that disagrees with an explicit `--format` (e.g. `-o out.svg --format
png`) is a hard error. `preview` is unaffected — it stays PDF-only.

**Per-page naming** (SVG/PNG): PDF is always one file. For SVG/PNG, a single-page
document writes the bare `<base>.<ext>`; a multi-page document writes one file
per page, `<base>-1.<ext> … <base>-N.<ext>`. `--merged` instead writes a single
combined `<base>.<ext>` (merged SVG canvas, or vertically stacked PNG). The
chosen naming convention is printed unless `--json` is set.

### Fat-file entry authoring

| Command | Args / flags | Notes |
|---------|--------------|-------|
| `ndoc new <path>` | — | Creates an empty `.ndoc.typ` document. `data: {"path": "<path>"}`. |
| `ndoc add <doc> <name>` | `--kind component\|template` (default `component`), `--content-file <file>` | Adds a named entry; content read from `--content-file` or stdin. `data: null`. |
| `ndoc edit <doc> <name>` | `--content-file <file>` | Replaces a named entry's content (from file or stdin). `data: null`. |

### Node-tree authoring (`doc` subgroup)

| Command | Args / flags | Notes |
|---------|--------------|-------|
| `ndoc doc new <template> [-o <out>]` | `template` is an id (→ `{id}.ndoct.typ`) or a path | Creates a `.ndoc.typ` bound to the template. Default output `{template-id}.ndoc.typ`; **refuses to overwrite**. `data: {"path": "<path>"}`. |
| `ndoc doc outline <doc>` | — | Prints the node tree (stable ids + component types). `data: {"template", "nodes":[{"id","component","children"}]}`. |
| `ndoc doc add <doc> --type <C>` | `--parent`/`--before`/`--after <id>` (mutually exclusive; root when none), `--inputs KEY=VALUE` (repeatable) | Validates the type against the catalogue, mints a stable id, seeds inputs. `data: {"node_id": "<id>"}`. |
| `ndoc doc remove <doc> <node_id>` | `--with-children` | Without the flag, children are promoted into the removed node's slot; with it the subtree is dropped. `data: {"removed": "<id>"}`. |
| `ndoc doc set <doc> [<node_id>]` | `--document` (target a doc-level input), `--key <k>`, `--value <v>` | Exactly one target: a `<node_id>` **or** `--document`. Value is validated/coerced against the input's declared kind (string/number/boolean/color/content/image). `data: {"target","key","value"}`. |
| `ndoc doc schema <target>` | `target` is a `.ncmp.typ` or `.ndoct.typ` file | Prints declared inputs as `name: kind (required\|optional)`. `data: {"component"\|"template": <schema>}`. |

### Library introspection

| Command | Args | Notes |
|---------|------|-------|
| `ndoc component schema <file>` | `.ncmp.typ` file | One component's input schema. `data: {"component": <schema>}`. |
| `ndoc component list <dir>` | directory | Lists every `*.ncmp.typ`. `data: {"components": [{"name","inputs"}]}`. |
| `ndoc template show <id\|path>` | id (→ `{id}.ndoct.typ`) or path | Document inputs + permitted components. `data: {"template": <schema>}`. |
| `ndoc item load <dir>` | directory | Summarises `*.item.md` collections. `data: {"collections": [{"collection","items"}]}`. |
| `ndoc item validate <dir>` | directory | Validates items against sibling `*.ncmp.typ` schemas; **exits 1** on any issue. `data: {"valid", "issues": [{"source","code","message"}]}`. |
| `ndoc image add <doc> <image>` | `.ndoc.typ` doc + image file | Embeds the image (deduped by blake3); re-embedding identical content is a no-op. `data: {"name","hash","added"}`. |

### Diagnostics

| Command | Args / flags | Notes |
|---------|--------------|-------|
| `ndoc jump <input> --page <n> --at <x>,<y>` | `.typ`/`.ndoc.typ`/`.md` input; `--page` is 1-based (default `1`); `--at x,y` is a page-local point in `pt` | **Hidden** from `ndoc --help` (`ndoc jump --help` still works). Compiles the input and maps the click back to its source via the `source-mapping` primitive. Prints `file <path>:<line>:<column>`, a `url`, an on-page `position`, or "no jump target". `data: {"kind":"file","path","offset","line","column"}` / `{"kind":"url","url"}` / `{"kind":"position",...}` / `null` when nothing is hit. Audience is debugging / the FFI, not end users. |

### JSON envelope

Every `--json` invocation prints one envelope to stdout:

```json
{ "status": "ok", "data": { /* command-specific, or null */ } }
```

On failure:

```json
{ "status": "error", "message": "<actionable text>" }
```

The `data` and `message` keys are mutually exclusive (`data` on success,
`message` on error) and the process exit code reflects success (`0`) or failure
(non-zero).

## Architecture

```
src/
├── main.rs          # `ndoc` binary entrypoint (thin shell)
├── lib.rs           # crate root + module map
├── cli/             # clap derive command definitions & dispatch
├── typst_world.rs   # typst::World impl over an in-memory virtual FS
├── compiler.rs      # .typ -> PDF bytes (typst::compile + typst_pdf)
├── markdown.rs      # Markdown -> Typst (comrak GFM AST walk)
├── fatfile/         # compose/extract/hash the .ndoc.typ fat file
├── authoring/       # transactional read-validate-write over fat files
├── schema/          # component/template input schemas + catalogue
├── model.rs         # shared domain types (Document, Node, inputs, IDs)
└── error.rs         # typed library errors
```

The Typst integration mirrors `typst-cli`'s own architecture: a hand-rolled
`World` over `typst` + `typst-kit` (fonts) + `typst-assets` (embedded default
fonts, so the single binary renders with zero system fonts) + `typst-pdf`.

The reference C# implementation lives at `.reference/Typst` (not modified by
this project).

## .NET / FFI binding

`crates/nordocs-ffi` builds a single native library (`libnordocs.dylib` /
`libnordocs.so` / `nordocs.dll`) that exposes the engine over a C ABI, replacing
both the reference's native `libtypst_core.dylib` and its managed `Common.Typst`
engine. The C# P/Invoke binding (`crates/nordocs-ffi/bindings/NordocsFfi.g.cs`)
is generated from the live FFI surface — regenerate with
`cargo test -p nordocs-ffi generate_csharp_binding`; CI fails if it is stale. A
hand-written `NordocsSession.cs` wraps the flat surface as idiomatic C#
(`byte[]`/`string` results, exceptions, and an `IDisposable` session). See
[`docs/ffi-packaging.md`](docs/ffi-packaging.md) for per-platform build and
packaging instructions.

## License

MIT OR Apache-2.0.
