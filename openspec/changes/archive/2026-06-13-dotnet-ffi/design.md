## Context

The C# reference splits into `Common.Typst` (managed engine over `TypstSharp`) and `Common.Typst.CLI`. Its native dependency, `TypstSharp`, is PDF-only and exposes `TypstCompiler.FromSource(source, root?)`, `SetSysInputs(dict)`, and `Compile() -> result.Buffers[]`. The managed engine layers markdown, fat-file composition, schema/template/item catalogues, document authoring, validation, and preview on top — all of which nordocs has reimplemented in Rust.

This change replaces both the native binding and the managed engine with one nordocs cdylib plus a generated C# binding. The chosen generator is **interoptopus** (Rust→C#, annotate functions and types, emit a C# class surface). The FFI wraps the binding-agnostic `core-api` façade from `workspace-split`; it adds no engine logic of its own.

Depends on `workspace-split` (façade + the `nordocs-ffi` skeleton), `multi-format-export` (`CompiledDoc` + multi-format + `sys.inputs`), and `source-map-session` (jump operations).

## Goals / Non-Goals

**Goals:**
- A C# binding whose compile/markdown/preview surface is close enough to `ITypstCompiler` / `IPreviewRenderer` / `IMarkdownToTypstConverter` that swapping the implementation is mechanical.
- Multi-format compile returning a `CompileResult { format, buffers[] }`, matching the `Buffers[]` shape C# already consumes.
- A safe FFI boundary: no panic unwinds across it; errors are structured.
- An opaque session handle exposed to C# as `IDisposable`.
- Authoring/validation/catalogue operations available with the same structured results the CLI uses.

**Non-Goals:**
- A WASM build (a separate `nordocs-wasm` shell over the same core; designed-for but not built here).
- Reproducing the C# DI container or `ILogger` plumbing — those are app concerns above the binding.
- 100% signature identity with the reference; "painless swap" means close shape, not byte-identical APIs (the user accepted divergence where it simplifies the implementation).

## Decisions

### interoptopus over the core-api façade
interoptopus annotates plain Rust functions/types and generates C# P/Invoke. Because `core-api` already returns serialisable structured results with no I/O, the FFI functions are thin: marshal inputs, call the façade, marshal results. Alternatives considered: hand-rolled C ABI (more boilerplate, manual marshalling), uniffi (richer object model but community-maintained C# backend and a heavier interface-definition step). interoptopus was chosen for the Rust→C# focus and the flat-function fit.

### Session handle model
`CompiledDoc` is exposed as an opaque pointer. C# receives it from `ndoc_compile_session(...)`, calls accessor functions (`svg`, `png`, `jump_from_click`, `jump_from_cursor`, `page_count`, `page_size`), and must call `ndoc_session_free`. A hand-written C# wrapper class wraps the handle and implements `IDisposable`/finalizer so idiomatic `using` works, mirroring the reference's `using var compiler = ...`. interoptopus models the handle as an opaque type; the `IDisposable` ergonomics live in the thin C# wrapper, not the generated code.

### Multi-buffer CompileResult
`compile(...)` returns `CompileResult { format: enum, buffers: Vec<Vec<u8>> }`. For PDF, `buffers` has one entry; for SVG/PNG, one per page (or one when merged). This matches `TypstSharp.CompileResult.Buffers[]`, so existing C# call sites that index `Buffers[0]` keep working for PDF.

### Error marshalling
Every fallible export function wraps its body in `std::panic::catch_unwind` and converts both `Err(core::Error)` and caught panics into a structured FFI error (code + message string) surfaced to C# as a thrown exception by the wrapper. No `Result` or panic crosses the ABI directly. Strings cross as UTF-8 with explicit length; byte buffers as pointer+length with a matching `free`.

### Binding freshness in CI
The generated C# binding is checked into the repo. A CI step regenerates it and fails if it differs from the committed copy, so the binding can never silently drift from the Rust surface.

### Native packaging
nordocs produces a cdylib per target (e.g. `libnordocs.dylib`/`.so`/`.dll`). The reference copies a per-RID native file into the C# output; nordocs documents the equivalent per-platform artifacts so the consuming project's packaging can reference them. Cross-platform build/packaging detail beyond producing the artifacts is left to the consumer's pipeline.

## Risks / Trade-offs

- **Panic safety gaps** → A panic that escapes `catch_unwind` is UB across FFI. Mitigation: a single wrapper macro applied to every export; a test that forces a compile error and asserts a clean error return, not a crash.
- **Handle leaks / double-free** → C# forgetting `Dispose`, or freeing twice. Mitigation: `IDisposable` + finalizer in the wrapper; the Rust `free` tolerates being the sole owner and is documented as single-call; a smoke test exercises allocate/use/free.
- **Surface drift from the reference** → Divergence could make the swap less mechanical than hoped. Mitigation: the parity test enumerates the reference operations and asserts coverage; intentional divergences are documented.
- **interoptopus expressiveness** → Some result shapes (nested enums, maps) may need flattening for interoptopus. Mitigation: keep FFI DTOs flat and `#[repr(C)]`-friendly; convert from the richer `core-api` types at the boundary.
- **CI without .NET** → A full C# consumer test needs a .NET SDK in CI. Mitigation: make the Rust-side FFI smoke tests authoritative; gate the optional C# consumer test on toolchain availability.
