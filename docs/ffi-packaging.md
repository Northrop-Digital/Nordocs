# Packaging the nordocs native library for .NET

The `nordocs-ffi` crate builds a **cdylib** named `nordocs`. This single native
library is the artifact that replaces the reference's `libtypst_core.dylib` (the
`TypstSharp` native dependency) **and** the managed `Common.Typst` engine: it
embeds the Typst compiler, the Markdown→Typst converter, the fat-file/authoring
model, schema/validation, multi-format export, and the source-map session.

The C# side consumes it through two files committed under
`crates/nordocs-ffi/bindings/`:

| File | Origin | Role |
| --- | --- | --- |
| `NordocsFfi.g.cs` | **generated** (interoptopus) | flat P/Invoke `DllImport`s + value-type DTOs |
| `NordocsSession.cs` | **hand-written** | idiomatic wrapper: `byte[]`/`string` results, exceptions, `IDisposable` session |

Regenerate `NordocsFfi.g.cs` with:

```sh
cargo test -p nordocs-ffi generate_csharp_binding
```

CI runs that generator and then `git diff --exit-code` on the file, so the
checked-in binding can never drift from the Rust surface.

## Per-platform artifacts

Build the release cdylib with cargo. The output file name follows the platform
convention because the crate's `[lib] name` is `nordocs`:

| Host triple (example) | `DllImport` name | Built artifact (`target/release/`) | Ship as |
| --- | --- | --- | --- |
| `aarch64-apple-darwin` / `x86_64-apple-darwin` | `nordocs` | `libnordocs.dylib` | `libnordocs.dylib` |
| `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu` | `nordocs` | `libnordocs.so` | `libnordocs.so` |
| `x86_64-pc-windows-msvc` | `nordocs` | `nordocs.dll` | `nordocs.dll` |

```sh
# Native (host) build:
cargo build --release -p nordocs-ffi
# => target/release/libnordocs.dylib | libnordocs.so | nordocs.dll

# Cross / explicit target (install the target first with `rustup target add`):
cargo build --release -p nordocs-ffi --target aarch64-apple-darwin
# => target/aarch64-apple-darwin/release/libnordocs.dylib
```

The `[DllImport]` entries in `NordocsFfi.g.cs` reference the library by the name
`nordocs`; the .NET runtime resolves that to `libnordocs.dylib` /
`libnordocs.so` / `nordocs.dll` per platform, so no per-RID file renaming is
needed (this matches how the reference shipped one native file per RID, e.g.
`osx-arm64/native/libtypst_core.dylib`).

## Wiring into a C# project

1. Add `NordocsFfi.g.cs` and `NordocsSession.cs` to the consuming project (or a
   small `Nordocs.Ffi` class library that the app references).
2. Place the platform's `libnordocs.*` next to the executable (or under
   `runtimes/<rid>/native/` in a NuGet package), exactly as the reference placed
   `libtypst_core.dylib`.
3. Use the idiomatic wrapper:

   ```csharp
   using Nordocs.Ffi;

   // ITypstCompiler.CompileToPdf / CompileFileToPdf
   byte[] pdf = Typst.CompileToPdf("= Hello\n\nWorld");

   // IMarkdownToTypstConverter.Convert
   string typ = Typst.MarkdownToTypst("# Title\n\nBody");

   // Source-map session — mirrors `using var compiler = ...`
   using var session = Typst.CompileSession(typ);
   for (ulong p = 0; p < session.PageCount(); p++)
   {
       string svg = session.Svg(p);
       string jump = session.JumpFromClick(p, 100.0, 120.0);
   }
   ```

## Verifying the binding (optional, toolchain-gated)

A minimal consumer round-trip lives at
`crates/nordocs-ffi/bindings/consumer-test/`. It needs a .NET SDK and the built
cdylib; the Rust-side FFI smoke tests in `nordocs-ffi` remain authoritative. Run
it with:

```sh
cargo build --release -p nordocs-ffi
crates/nordocs-ffi/bindings/consumer-test/run.sh
```
