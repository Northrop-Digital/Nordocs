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

Early scaffold. The module skeleton compiles and the CLI surface is stubbed;
the C# services are being ported behind it.

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
