## 1. FFI crate foundation

- [ ] 1.1 Add `interoptopus` (and the C# backend generator) to `nordocs-ffi`; set `crate-type = ["cdylib"]`
- [ ] 1.2 Add a panic-guard wrapper (macro or helper) that runs each export under `catch_unwind` and converts `Err`/panic into a structured FFI error (code + message)
- [ ] 1.3 Define flat, `#[repr(C)]`-friendly FFI DTOs and conversions from the `core-api` result types
- [ ] 1.4 Define byte-buffer and UTF-8 string marshalling helpers with explicit length and matching `free`

## 2. Compiler and markdown surface

- [ ] 2.1 Export `compile_to_pdf(source, vars)` and `compile_file_to_pdf(path)` mirroring the reference `ITypstCompiler`
- [ ] 2.2 Export multi-format `compile(source, vars, format, dpi) -> CompileResult { format, buffers[] }` (one buffer per page; one for PDF/merged)
- [ ] 2.3 Export `markdown_to_typst(md) -> string` mirroring `IMarkdownToTypstConverter`
- [ ] 2.4 Rust FFI smoke test: round-trip a PDF compile and a markdown conversion across the boundary

## 3. Preview, authoring, validation surface

- [ ] 3.1 Export `render_component_preview(...)` and `render_document_preview(...)` mirroring `IPreviewRenderer`
- [ ] 3.2 Export the authoring operations (compose, doc new/add/set/remove, image embed) over `core-api`, returning structured results
- [ ] 3.3 Export validation and catalogue introspection (validate, schema/template/item/component) over `core-api`
- [ ] 3.4 Rust FFI smoke tests for one authoring op and one validation op

## 4. Source-map session handle

- [ ] 4.1 Export `compile_session(source, vars) -> *CompiledDoc` (opaque handle) and `session_free(handle)`
- [ ] 4.2 Export `session_svg(handle, page)`, `session_png(handle, page, dpi)`, `session_page_count(handle)`, `session_page_size(handle, i)`
- [ ] 4.3 Export `session_jump_from_click(handle, page, x, y)` and `session_jump_from_cursor(handle, file, offset)` returning the serialisable `Jump`/`Position` DTOs
- [ ] 4.4 Rust FFI smoke test: allocate session, export SVG, run a jump, free; assert no leak/crash and a clean double-free guard

## 5. C# binding generation and parity

- [ ] 5.1 Add a generation entry point (test or `xtask`) emitting the interoptopus C# binding to a published path; check the generated binding into the repo
- [ ] 5.2 Hand-write a thin C# wrapper presenting the session handle as `IDisposable` (mirroring `using var compiler = ...`)
- [ ] 5.3 Parity test: enumerate the reference `ITypstCompiler` / `IPreviewRenderer` / `IMarkdownToTypstConverter` operations and assert the generated surface covers them; document intentional divergences
- [ ] 5.4 CI step regenerates the binding and fails if it differs from the committed copy

## 6. Packaging and checks

- [ ] 6.1 Document producing the `nordocs` cdylib per platform (the artifact that replaces the reference `libtypst_core.dylib`)
- [ ] 6.2 (Optional, toolchain-gated) a minimal C# consumer test that loads the cdylib and round-trips a compile
- [ ] 6.3 Confirm `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check`, and ≥ 80% coverage are green across the workspace
