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

    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|diags| {
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
    use super::compile_to_pdf;
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
}
