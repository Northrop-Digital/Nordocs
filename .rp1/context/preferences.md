# Project Preferences
**Generated**: 2026-06-04T06:50:43Z | **Finalized**: 2026-06-04 | **Status**: Scaffolded

## Technology Stack

| Concern | Choice | Rationale |
|---------|--------|-----------|
| Language | Rust 2021 (rust-version 1.85) | Native single binary; fast startup vs the C# tool's .NET overhead. |
| Binary | `ndoc` (single crate) | One distributable binary, CLI-first surface per charter. |
| CLI | clap 4 (derive) | Ergonomic, typed command tree; replaces C# System.CommandLine. |
| Errors | anyhow (app) + thiserror (lib) | anyhow for rich CLI error chains; thiserror typed lib errors (Authoring/Validation/Schema/Compile/FatFile) mirroring the C# error split. |
| Serialization | serde + serde_json + serde_yaml_ng | JSON STATE prelude + `--json` envelope; YAML frontmatter. |
| Typst | typst / typst-library / typst-syntax / typst-pdf / typst-kit (fonts) / typst-assets / comemo, all 0.14 | Embedded compiler (charter failure-signal #1). Hand-rolled `World` over typst + typst-kit fonts + typst-assets embedded default fonts (zero-system-font single binary) + typst-pdf, mirroring typst-cli's own architecture. |
| Markdown | comrak 0.30 (GFM) | CommonMark + tables/tasklist/strikethrough/footnotes; arena AST suits the recursive walk ported from C# Markdig. |
| Testing | cargo test + insta (snapshots) + assert_cmd/predicates (CLI E2E) | Snapshot composed `.typ`; E2E the binary surface. |
| Lint/Format | clippy + rustfmt (100-col) | Standard Rust hygiene. |

## Project Structure

```
NorthDoc/
├── Cargo.toml / Cargo.lock
├── .gitignore / rustfmt.toml
├── README.md / AGENTS.md / CLAUDE.md
├── src/
│   ├── main.rs            # ndoc entrypoint (thin shell)
│   ├── lib.rs             # crate root + module map
│   ├── error.rs           # typed library errors
│   ├── model.rs           # Document / Node / inputs / NodeId
│   ├── typst_world.rs     # typst::World over in-memory virtual FS
│   ├── compiler.rs        # .typ -> PDF (typst::compile + typst_pdf)
│   ├── markdown.rs        # Markdown -> Typst (comrak)
│   ├── cli/mod.rs         # clap commands + dispatch
│   ├── fatfile/mod.rs     # compose/extract/hash .ndoc.typ
│   ├── authoring/mod.rs   # transactional authoring over fat files
│   └── schema/mod.rs      # component/template schemas + catalogue
└── tests/
    ├── cli.rs             # assert_cmd CLI tests
    ├── fatfile.rs         # insta snapshot test
    └── snapshots/
```

## Commands

```sh
cargo build              # debug
cargo build --release    # single binary: target/release/ndoc
cargo test               # unit + insta + assert_cmd
cargo clippy --all-targets
cargo fmt
```

## Notes
- `.reference/Typst` is read-only C# source to port from; never modified.
- Scope: v1 is CLI-first and PDF-only. GUI, non-PDF output, plugin system, and
  the AgentTools (MCP) surface are deferred per the charter.
