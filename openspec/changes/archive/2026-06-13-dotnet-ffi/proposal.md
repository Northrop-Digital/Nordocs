## Why

The downstream consumer is a C# application that today calls `Common.Typst` (managed) over `TypstSharp` (native, PDF-only). nordocs already ports the entire `Common.Typst` engine — compiler, markdown, fat-file, schema, authoring, validation — into Rust, plus the new SVG/PNG export and click-to-source. This change exposes that engine over a .NET FFI so the C# app can swap its native+managed Typst stack for a single `nordocs` native library with minimal churn, keeping a surface close to the reference interfaces (`ITypstCompiler`, `IPreviewRenderer`, `IMarkdownToTypstConverter`) plus the authoring/validation operations the app drives.

## What Changes

- Build out `nordocs-ffi` (cdylib) as an [interoptopus](https://github.com/ralfbiedert/interoptopus)-annotated surface over the `nordocs-core` façade, generating C# P/Invoke bindings.
- Expose, mirroring the reference where practical:
  - **Compiler** — `compile_to_pdf(source, vars)`, `compile_file_to_pdf(path)`; plus multi-format `compile(source, vars, format, dpi) -> CompileResult { format, buffers[] }` (one buffer per page, matching `TypstSharp.CompileResult.Buffers[]`).
  - **Markdown** — `markdown_to_typst(md) -> string`.
  - **Preview** — `render_component_preview(...)`, `render_document_preview(...)`.
  - **Authoring + validation + catalogues** — the operations exposed by `core-api` (compose, validate, doc authoring, schema/template/item/component introspection), returning the same structured results the CLI uses.
  - **Source-map session** — an opaque `CompiledDoc` handle with `svg(page)`, `png(page, dpi)`, `jump_from_click`, `jump_from_cursor`, `page_count`, `page_size`, and explicit `free`.
- Guard the boundary: every fallible call uses `catch_unwind` and returns an error code / structured error; no Rust panic unwinds across FFI.
- Provide a C# wrapper that presents the session handle as `IDisposable` (mirroring the reference's `using var compiler = ...`).
- Ship a parity test asserting the generated C# surface covers the reference `ITypstCompiler` / `IPreviewRenderer` / `IMarkdownToTypstConverter` operations.

## Capabilities

### New Capabilities

- `ffi-binding`: A C-ABI / .NET binding over the nordocs engine — compile (multi-format), markdown, preview, authoring, validation, and the source-map session — with safe error marshalling and explicit handle lifetime.

### Modified Capabilities

<!-- None — the FFI wraps existing core-api operations; no engine behaviour changes. -->

## Impact

- `crates/nordocs-ffi`: populated with interoptopus-annotated `extern` functions, the opaque handle type, error marshalling, and a binding-generation step.
- Build: a generation entry point (test or `xtask`/`build.rs`) emits the C# binding into a published location; CI runs it and fails if the checked-in binding is stale.
- Tests: Rust-side FFI smoke tests (round-trip a compile, a jump, handle free); a generated-binding parity check against the reference interface surface. Optionally a minimal C# consumer test if a .NET toolchain is available in CI.
- Native packaging: document producing `nordocs` cdylib artifacts per platform (the reference ships `libtypst_core.dylib` for osx-arm64; nordocs replaces it).
- Depends on `workspace-split`, `multi-format-export`, and `source-map-session`.
