//! High-level compile/export wrapper.
//!
//! Wraps [`typst::compile`] + [`typst_pdf::pdf`] into a single `.typ` -> PDF
//! bytes operation over our [`NorthdocWorld`]. Diagnostics are flattened into
//! [`Error::Compile`].

use crate::error::{Error, Result};
use crate::typst_world::NorthdocWorld;

/// Compile a composed `.typ` source string into PDF bytes.
///
/// Builds a fresh [`NorthdocWorld`], runs the Typst compiler, and exports to
/// PDF. Compilation warnings are currently discarded; surface them once the
/// CLI grows a diagnostics channel.
pub fn compile_to_pdf(main_source: &str) -> Result<Vec<u8>> {
    compile_to_pdf_with_options(main_source, &typst_pdf::PdfOptions::default())
}

fn compile_to_pdf_with_options(
    main_source: &str,
    pdf_options: &typst_pdf::PdfOptions,
) -> Result<Vec<u8>> {
    let world = NorthdocWorld::new(main_source.to_owned());

    let compiled = typst::compile::<typst::layout::PagedDocument>(&world);
    let document = compiled.output.map_err(|diags| {
        let msg = diags
            .iter()
            .map(|d| d.message.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        Error::Compile(msg)
    })?;

    let pdf = typst_pdf::pdf(&document, pdf_options).map_err(|diags| {
        let msg = diags
            .iter()
            .map(|d| d.message.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        Error::Compile(msg)
    })?;

    // Keep the incremental cache bounded between invocations.
    NorthdocWorld::evict_cache(5);

    Ok(pdf)
}

#[cfg(test)]
mod tests {
    use super::{compile_to_pdf, compile_to_pdf_with_options};
    use crate::error::Error;

    #[test]
    fn compile_to_pdf_happy_path() {
        let result = compile_to_pdf("Hello, Typst!").expect("valid source should compile to PDF");
        assert!(!result.is_empty(), "PDF bytes should be non-empty");
    }

    #[test]
    fn compile_to_pdf_invalid_source() {
        let result = compile_to_pdf("#panic(\"forced compile error\")");
        match result {
            Err(Error::Compile(msg)) => {
                assert!(!msg.is_empty(), "compile error message should be non-empty")
            }
            other => panic!("expected Error::Compile, got {:?}", other),
        }
    }

    #[test]
    fn compile_to_pdf_empty_source_succeeds() {
        let pdf = compile_to_pdf("").expect("empty source compiles to a blank PDF");
        assert!(!pdf.is_empty(), "blank PDF must produce non-empty bytes");
    }

    #[test]
    fn compile_to_pdf_export_error_maps_to_compile_error() {
        // PDF/UA-1 requires a document title. A plain document without
        // `#set document(title: ...)` fails at typst_pdf::pdf(), exercising
        // the PDF export error closure.
        let standards = typst_pdf::PdfStandards::new(&[typst_pdf::PdfStandard::Ua_1])
            .expect("PDF/UA-1 standards creation should succeed");
        let options = typst_pdf::PdfOptions {
            standards,
            ..typst_pdf::PdfOptions::default()
        };
        let result = compile_to_pdf_with_options("Hello!", &options);
        match result {
            Err(Error::Compile(msg)) => {
                assert!(
                    !msg.is_empty(),
                    "PDF/UA-1 violation should produce a non-empty error message"
                );
            }
            other => panic!(
                "expected Error::Compile from PDF/UA-1 title violation, got {:?}",
                other
            ),
        }
    }
}
