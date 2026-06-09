# northdoc

A Rust-native re-implementation of the C# Typst document toolset. northdoc
**embeds the native Typst compiler** (no external process), ships as a single
binary (`ndoc`), and exposes a refined, CLI-first surface for turning
Markdown/data into PDF documents.

## Why

The previous C# tool shelled out to an external Typst binary, carried an
organically-grown CLI with cruft, and paid .NET startup cost on every
short-lived invocation. northdoc is a **port-and-refine**: same core
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

Out of scope for v1: GUI, non-PDF output, a plugin system, and the AgentTools
(MCP) programmatic surface (deferred).

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
| `ndoc render <input> [-o <out>]` | `.ncmp.typ`, `.ndoct.typ`, `.ndoc.typ` (never a bare `.typ`) | input with the recognised suffix replaced by `.pdf` | `-o`/`--output` overrides the path. `data: {"output": "<path>"}`. |
| `ndoc build <input>` | `.md`, `.ndoc.typ` | input with extension replaced by `.pdf` (`.ndoc.typ` → `.pdf`) | Composes/converts then compiles. `data: {"output": "<path>"}`. |
| `ndoc validate <input>` | `.ndoc.typ`, `.md` | — | Prints each violation as `location: message`; **exits 1** if any. `data: {"violations": [{"location","message"}]}`. |
| `ndoc preview <input>` | `.ndoc.typ`, `.md` | temp PDF in the OS temp dir | Opens the OS default viewer. Set `NDOC_NO_OPEN=1` to skip the viewer (PDF still rendered + verified non-empty). `data: {"preview_path": "<path>"}`. |

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

## License

MIT OR Apache-2.0.
