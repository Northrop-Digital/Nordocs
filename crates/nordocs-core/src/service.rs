//! Binding-agnostic service façade.
//!
//! Every operation a front end performs is available here as a function that
//! takes owned inputs and returns structured data, with **no** stdout/stderr,
//! `process::exit`, or file-viewer side effects. The `ndoc` CLI
//! (`nordocs-cli`) and the .NET FFI (`nordocs-ffi`) are thin adapters that read
//! files/stdin, call these functions, and render the result (`--json` envelope,
//! human text, exit codes) — they perform no engine logic of their own.
//!
//! This mirrors the C# reference split: `Common.Typst` (engine, structured
//! results) vs `Common.Typst.CLI` (executable). The function names track the
//! reference interfaces where practical (`ITypstCompiler.CompileToPdf` /
//! `CompileFileToPdf`, `IMarkdownToTypstConverter.Convert`).
//!
//! Authoring, validation, and catalogue introspection already live as
//! structured-result operations in [`crate::authoring`], [`crate::validation`],
//! [`crate::schema`], and [`crate::item`]; this module adds the compile/render
//! entry points and re-documents the surface as the canonical `core-api`.

use std::path::Path;

use crate::compiler::{CompiledDoc, Jump};
use crate::error::Result;

/// Component / document live-preview rendering (the `IPreviewRenderer` surface).
///
/// Re-exported from [`crate::preview`] so bindings reach the whole `core-api`
/// through this one façade module: compose a component or document into live
/// Typst and compile it to PDF, with no I/O side effects.
pub use crate::preview::{render_component_preview, render_document_preview};

/// Compile raw Typst markup to PDF bytes (content form).
///
/// Mirrors the reference `ITypstCompiler.CompileToPdf`. A thin façade over
/// [`crate::compiler::compile_to_pdf`].
pub fn compile_to_pdf(source: &str) -> Result<Vec<u8>> {
    crate::compiler::compile_to_pdf(source)
}

/// Compile raw Typst markup to PDF bytes with `sys.inputs` injected.
///
/// Mirrors the reference `TypstCompiler.SetSysInputs` followed by
/// `CompileToPdf`: `sys_inputs` is an ordered list of `(key, value)` pairs the
/// document can read as `sys.inputs.<key>`. Passing an empty slice is identical
/// to [`compile_to_pdf`].
pub fn compile_to_pdf_with_inputs(
    source: &str,
    sys_inputs: &[(String, String)],
) -> Result<Vec<u8>> {
    crate::compiler::compile(source, sys_inputs)?.to_pdf()
}

/// Compile raw Typst markup into a retained [`CompiledDoc`] (multi-format).
///
/// The returned document can be exported to PDF, SVG, or PNG without
/// recompiling. `sys_inputs` is injected as `sys.inputs` before compilation.
/// This is the façade entry point bindings (CLI, FFI) use when they need a
/// format other than PDF or want to emit several formats from one compile.
///
/// Because the world is retained alongside the laid-out pages, the returned
/// [`CompiledDoc`] also exposes the bidirectional source map — click → source
/// ([`CompiledDoc::jump_from_click`]) and cursor → preview
/// ([`CompiledDoc::jump_from_cursor`]), plus page geometry
/// ([`CompiledDoc::page_size`]) — so a binding can drive click-to-source over a
/// single compiled session.
pub fn compile_session(source: &str, sys_inputs: &[(String, String)]) -> Result<CompiledDoc> {
    crate::compiler::compile(source, sys_inputs)
}

/// Compile the document at `path` to PDF bytes (path form).
///
/// Mirrors the reference `ITypstCompiler.CompileFileToPdf`. Routes by the
/// file's shape:
/// - `*.md` → Markdown is converted to Typst, then compiled.
/// - `*.ndoc.typ` → a canonical composed fat file renders directly (its
///   embedded images resolved); an entry-format archive has its entries
///   concatenated before compilation (see [`document_archive_to_pdf`]).
/// - any other Typst file (`*.ncmp.typ`, `*.ndoct.typ`, bare `*.typ`) → its raw
///   contents are compiled verbatim.
pub fn compile_file_to_pdf(path: &Path) -> Result<Vec<u8>> {
    let name = path.to_string_lossy();
    let src = std::fs::read_to_string(path)?;
    if name.ends_with(".md") {
        let typst_source = crate::markdown::markdown_to_typst(&src)?;
        crate::compiler::compile_to_pdf(&typst_source)
    } else if name.ends_with(".ndoc.typ") {
        document_archive_to_pdf(&src)
    } else {
        crate::compiler::compile_to_pdf(&src)
    }
}

/// Render a `.ndoc.typ` document *source string* (composed or entry-format) to
/// PDF bytes.
///
/// Shared by the CLI `build` and `preview` commands: a canonical composed fat
/// file renders directly (resolving its embedded images); an entry-format
/// archive has its entries concatenated before compilation. Keeping this in the
/// core means the routing rule lives in exactly one place.
pub fn document_archive_to_pdf(src: &str) -> Result<Vec<u8>> {
    if crate::fatfile::composed::is_composed(src) {
        crate::fatfile::composed::render_to_pdf(src)
    } else {
        let doc = crate::fatfile::ndoc::NdocDocument::parse(src)?;
        let typst_source = doc
            .entries
            .iter()
            .map(|e| e.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        crate::compiler::compile_to_pdf(&typst_source)
    }
}

/// Render a `.ndoc.typ` document *source string* (composed or entry-format)
/// into a retained [`CompiledDoc`].
///
/// The session variant of [`document_archive_to_pdf`]: it applies the identical
/// composed-vs-entry routing but retains the laid-out document so a binding can
/// emit PDF, SVG, or PNG (and any per-page split) without recompiling. Used by
/// the CLI `build` command when a non-PDF `--format` is requested.
pub fn document_archive_to_session(src: &str) -> Result<CompiledDoc> {
    if crate::fatfile::composed::is_composed(src) {
        crate::fatfile::composed::render_to_session(src)
    } else {
        let doc = crate::fatfile::ndoc::NdocDocument::parse(src)?;
        let typst_source = doc
            .entries
            .iter()
            .map(|e| e.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        crate::compiler::compile(&typst_source, &[])
    }
}

/// Map a click on a rendered page back to its source location (point form).
///
/// A binding-agnostic wrapper over [`CompiledDoc::jump_from_click`] that takes
/// page-local coordinates in typographic points (`pt`) as plain `f64`s, so a
/// front end (the FFI, a future WASM shell) need not depend on `typst`'s
/// geometry types to drive click-to-source over a compiled session. `page_index`
/// is 0-based. Returns `None` when the page is out of range or the click lands on
/// content carrying no source span.
pub fn jump_from_click(doc: &CompiledDoc, page_index: usize, x_pt: f64, y_pt: f64) -> Option<Jump> {
    let point =
        typst::layout::Point::new(typst::layout::Abs::pt(x_pt), typst::layout::Abs::pt(y_pt));
    doc.jump_from_click(page_index, point)
}

/// Convert Markdown content to Typst markup.
///
/// Mirrors the reference `IMarkdownToTypstConverter.Convert`. A thin façade over
/// [`crate::markdown::markdown_to_typst`].
pub fn markdown_to_typst(markdown: &str) -> Result<String> {
    crate::markdown::markdown_to_typst(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_to_pdf_produces_pdf_bytes() {
        let pdf = compile_to_pdf("Hello via the façade").expect("compile");
        assert_eq!(&pdf[..5], b"%PDF-");
    }

    #[test]
    fn markdown_to_typst_round_trips() {
        let typ = markdown_to_typst("# Title\n\nBody").expect("convert");
        assert!(typ.contains("Title"), "expected heading text in: {typ}");
    }

    #[test]
    fn compile_file_to_pdf_routes_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "# Heading\n\nParagraph text.").expect("write md");
        let pdf = compile_file_to_pdf(&path).expect("compile md file");
        assert_eq!(&pdf[..5], b"%PDF-");
    }

    #[test]
    fn compile_to_pdf_with_inputs_injects_value() {
        let pdf = compile_to_pdf_with_inputs(
            "#sys.inputs.heading",
            &[("heading".to_string(), "Hello".to_string())],
        )
        .expect("compile with injected sys.inputs");
        assert_eq!(&pdf[..5], b"%PDF-");
    }

    #[test]
    fn compile_to_pdf_with_inputs_missing_key_errors() {
        let result = compile_to_pdf_with_inputs("#sys.inputs.absent", &[]);
        assert!(
            matches!(result, Err(crate::error::Error::Compile(_))),
            "reading an unset sys.inputs key should fail to compile"
        );
    }

    #[test]
    fn compile_session_exports_multiple_formats() {
        let doc = compile_session("#set page(width: 90pt, height: 60pt)\nHi", &[])
            .expect("session compiles");
        assert_eq!(doc.page_count(), 1);
        let svg = doc.to_svg(0).expect("svg export");
        assert!(svg.starts_with("<svg"));
        let pdf = doc.to_pdf().expect("pdf export");
        assert_eq!(&pdf[..5], b"%PDF-");
    }

    #[test]
    fn document_archive_to_session_exports_svg() {
        // An entry-format archive routes through the concatenation path and the
        // retained document exports to SVG without recompiling.
        let src = "// ndoc document v1\n\
            // === NDOC-ENTRY: main kind=component hash=0000000000000000000000000000000000000000000000000000000000000000 ===\n\
            #set page(width: 90pt, height: 60pt)\nHello\n\
            // === NDOC-END: main ===\n";
        let doc = document_archive_to_session(src).expect("archive compiles to a session");
        assert_eq!(doc.page_count(), 1);
        let svg = doc.to_svg(0).expect("svg export");
        assert!(
            svg.starts_with("<svg"),
            "expected an SVG document: {svg:.20}"
        );
    }

    #[test]
    fn jump_from_click_point_form_resolves_or_misses() {
        // A small fixed-size page with text in the top-left corner. A click well
        // clear of the glyphs resolves to no jump; the wrapper accepts plain
        // point coordinates without exposing typst geometry types.
        let doc = compile_session(
            "#set page(width: 120pt, height: 80pt, margin: 10pt)\nJump",
            &[],
        )
        .expect("fixture compiles");
        assert!(
            jump_from_click(&doc, 0, 115.0, 75.0).is_none(),
            "a click on empty space resolves to no jump target"
        );
        assert!(
            jump_from_click(&doc, 99, 1.0, 1.0).is_none(),
            "an out-of-range page resolves to no jump target"
        );
    }

    #[test]
    fn compile_file_to_pdf_routes_raw_typst() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("c.ncmp.typ");
        std::fs::write(&path, "Raw typst body").expect("write typ");
        let pdf = compile_file_to_pdf(&path).expect("compile raw typst file");
        assert_eq!(&pdf[..5], b"%PDF-");
    }
}
