//! Compiler and markdown exports (the `ITypstCompiler` /
//! `IMarkdownToTypstConverter` surface).
//!
//! Mirrors the reference `ITypstCompiler.CompileToPdf` / `CompileFileToPdf` and
//! `IMarkdownToTypstConverter.Convert`, plus a multi-format [`ndoc_compile`] that
//! returns a [`CompileResult`] of one byte buffer per page (SVG/PNG) or a single
//! buffer (PDF), matching `TypstSharp.CompileResult.Buffers[]`. Every export runs
//! its body under the panic guard and reports failure through the out-parameter
//! [`FfiError`].

use std::path::Path;

use interoptopus::patterns::string::AsciiPointer;
use interoptopus::{ffi_function, ffi_type};

use crate::convert::{arg_str, arg_str_opt, parse_vars, settle};
use crate::error::FfiError;
use crate::guard::run_guarded;
use crate::marshal::{ndoc_byte_buffer_free, ByteBuffer, FfiString};

/// The output format requested from [`ndoc_compile`].
///
/// Mirrors the formats the engine's [`CompiledDoc`](nordocs_core::compiler::CompiledDoc)
/// exports. `Pdf` yields a single buffer; `Svg`/`Png` yield one buffer per page.
#[ffi_type]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FfiFormat {
    /// A single PDF document buffer.
    Pdf = 0,
    /// One SVG document buffer per page.
    Svg = 1,
    /// One PNG image buffer per page.
    Png = 2,
}

/// A multi-format compile result: the chosen `format` plus its output buffers.
///
/// `buffers` points to `len` [`ByteBuffer`]s (one per page for SVG/PNG; one for
/// PDF). The caller owns the whole result and MUST release it exactly once with
/// [`ndoc_compile_result_free`], which frees every inner buffer and the backing
/// array. `buffers` is null when `len == 0`.
#[ffi_type]
#[repr(C)]
pub struct CompileResult {
    /// The format the buffers are encoded in (echoes the requested format).
    pub format: FfiFormat,
    /// Pointer to the first [`ByteBuffer`], or null when `len == 0`.
    pub buffers: *mut ByteBuffer,
    /// Number of buffers.
    pub len: u64,
    /// Backing array capacity; needed to free safely.
    pub capacity: u64,
}

impl CompileResult {
    /// An empty result for `format` (null array, zero length).
    fn empty(format: FfiFormat) -> Self {
        Self {
            format,
            buffers: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    /// Move a `Vec<ByteBuffer>` out across the boundary, transferring ownership.
    fn from_buffers(format: FfiFormat, mut buffers: Vec<ByteBuffer>) -> Self {
        if buffers.is_empty() {
            return Self::empty(format);
        }
        let ptr = buffers.as_mut_ptr();
        let len = buffers.len() as u64;
        let capacity = buffers.capacity() as u64;
        std::mem::forget(buffers);
        Self {
            format,
            buffers: ptr,
            len,
            capacity,
        }
    }

    /// Reclaim the backing array and free every inner buffer.
    ///
    /// # Safety
    /// Must be called at most once, with the `(len, capacity)` produced by
    /// [`CompileResult::from_buffers`].
    unsafe fn into_owned(self) {
        if self.buffers.is_null() {
            return;
        }
        let buffers = Vec::from_raw_parts(self.buffers, self.len as usize, self.capacity as usize);
        for buffer in buffers {
            ndoc_byte_buffer_free(buffer);
        }
    }
}

/// Compile raw Typst markup to PDF bytes, with optional `sys.inputs`.
///
/// `vars` is a JSON object of `sys.inputs` values (or null/empty for none).
/// Mirrors the reference `ITypstCompiler.CompileToPdf`. On failure returns the
/// empty buffer and sets `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_compile_to_pdf(
    source: AsciiPointer,
    vars: AsciiPointer,
    out_err: &mut FfiError,
) -> ByteBuffer {
    let result = run_guarded(move || {
        let source = arg_str(source)?;
        let vars = parse_vars(&arg_str_opt(vars).unwrap_or_default())?;
        nordocs_core::service::compile_to_pdf_with_inputs(&source, &vars)
    });
    settle(
        result.map(ByteBuffer::from_vec),
        out_err,
        ByteBuffer::empty(),
    )
}

/// Compile the document at `path` to PDF bytes (path form).
///
/// Mirrors the reference `ITypstCompiler.CompileFileToPdf`; routing by file shape
/// (`.md`, `.ndoc.typ`, raw Typst) is the engine's. On failure returns the empty
/// buffer and sets `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_compile_file_to_pdf(
    path: AsciiPointer,
    out_err: &mut FfiError,
) -> ByteBuffer {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        nordocs_core::service::compile_file_to_pdf(Path::new(&path))
    });
    settle(
        result.map(ByteBuffer::from_vec),
        out_err,
        ByteBuffer::empty(),
    )
}

/// Compile to a multi-format [`CompileResult`].
///
/// `vars` is a JSON object of `sys.inputs` values (or null/empty for none).
/// `dpi` is the PNG render resolution (ignored for PDF/SVG). For SVG/PNG the
/// result holds one buffer per page; for PDF, a single buffer. On failure returns
/// an empty result of the requested format and sets `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_compile(
    source: AsciiPointer,
    vars: AsciiPointer,
    format: FfiFormat,
    dpi: f32,
    out_err: &mut FfiError,
) -> CompileResult {
    // Build owned byte vectors inside the guard so a mid-way export error leaves
    // no leaked (already-forgotten) ByteBuffers; convert to buffers only once the
    // whole set succeeds.
    let result = run_guarded(move || -> nordocs_core::Result<Vec<Vec<u8>>> {
        let source = arg_str(source)?;
        let vars = parse_vars(&arg_str_opt(vars).unwrap_or_default())?;
        let doc = nordocs_core::service::compile_session(&source, &vars)?;
        match format {
            FfiFormat::Pdf => Ok(vec![doc.to_pdf()?]),
            FfiFormat::Svg => (0..doc.page_count())
                .map(|p| doc.to_svg(p).map(String::into_bytes))
                .collect(),
            FfiFormat::Png => (0..doc.page_count()).map(|p| doc.to_png(p, dpi)).collect(),
        }
    });
    settle(
        result.map(|raw| {
            CompileResult::from_buffers(format, raw.into_iter().map(ByteBuffer::from_vec).collect())
        }),
        out_err,
        CompileResult::empty(format),
    )
}

/// Convert Markdown content to Typst markup.
///
/// Mirrors the reference `IMarkdownToTypstConverter.Convert`. On failure returns
/// the empty string and sets `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_markdown_to_typst(
    markdown: AsciiPointer,
    out_err: &mut FfiError,
) -> FfiString {
    let result = run_guarded(move || {
        let markdown = arg_str(markdown)?;
        nordocs_core::service::markdown_to_typst(&markdown)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Free a [`CompileResult`] returned by [`ndoc_compile`].
///
/// Single-call: frees every inner [`ByteBuffer`] and the backing array. Passing
/// an empty result (null `buffers`) is a no-op. Panics during free are swallowed
/// so nothing unwinds across the ABI.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_compile_result_free(result: CompileResult) {
    // SAFETY: caller upholds the single-call ownership contract; AssertUnwindSafe
    // because the raw pointer is consumed exactly once here.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        result.into_owned()
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FfiErrorCode;
    use std::ffi::CString;

    /// Build a NUL-terminated [`AsciiPointer`] from `s`, kept alive by the
    /// returned `CString` (which the caller must hold for the pointer's use).
    fn cstr(s: &str) -> CString {
        CString::new(s).expect("no interior NUL in test input")
    }

    #[test]
    fn compile_to_pdf_round_trips_across_the_boundary() {
        let source = cstr("Hello from the FFI boundary");
        let mut err = FfiError::ok();
        let buffer = ndoc_compile_to_pdf(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok, "compile must succeed");
        assert!(buffer.len >= 5, "PDF must be non-trivial");
        // SAFETY: reading the bytes we just received ownership of.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) };
        assert_eq!(&bytes[..5], b"%PDF-", "must be a PDF");
        ndoc_byte_buffer_free(buffer);
    }

    #[test]
    fn compile_to_pdf_with_vars_injects_sys_inputs() {
        let source = cstr("#sys.inputs.heading");
        let vars = cstr(r#"{"heading":"Hi"}"#);
        let mut err = FfiError::ok();
        let buffer = ndoc_compile_to_pdf(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::from_slice_with_nul(vars.as_bytes_with_nul()).expect("ascii"),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert!(buffer.len > 0);
        ndoc_byte_buffer_free(buffer);
    }

    #[test]
    fn compile_invalid_source_reports_structured_error() {
        let source = cstr("#this_function_does_not_exist()");
        let mut err = FfiError::ok();
        let buffer = ndoc_compile_to_pdf(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Compile, "must be a compile error");
        assert!(err.message.len > 0, "error message must be non-empty");
        assert!(buffer.data.is_null(), "failure yields the empty buffer");
        ndoc_byte_buffer_free(buffer);
        crate::error::ndoc_error_free(err);
    }

    #[test]
    fn compile_multi_svg_yields_one_buffer_per_page() {
        let source = cstr("#set page(width: 90pt, height: 60pt)\nSVG body");
        let mut err = FfiError::ok();
        let result = ndoc_compile(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            FfiFormat::Svg,
            144.0,
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert_eq!(result.format, FfiFormat::Svg);
        assert_eq!(result.len, 1, "single-page fixture yields one buffer");
        // SAFETY: the result owns `len` buffers we may read before freeing.
        let buffers = unsafe { std::slice::from_raw_parts(result.buffers, result.len as usize) };
        let first = unsafe { std::slice::from_raw_parts(buffers[0].data, buffers[0].len as usize) };
        assert!(first.starts_with(b"<svg"), "buffer must be an SVG document");
        ndoc_compile_result_free(result);
    }

    #[test]
    fn compile_multi_pdf_yields_single_buffer() {
        let source = cstr("PDF body");
        let mut err = FfiError::ok();
        let result = ndoc_compile(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            FfiFormat::Pdf,
            72.0,
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert_eq!(result.len, 1, "PDF is a single buffer");
        ndoc_compile_result_free(result);
    }

    #[test]
    fn compile_file_to_pdf_round_trips_a_temp_file() {
        // The path form of the PDF round-trip: write raw Typst to disk, compile
        // it through the boundary, and assert real PDF bytes come back.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.typ");
        std::fs::write(&path, "Compiled from a file path").expect("write source");
        let path_c = cstr(&path.to_string_lossy());
        let mut err = FfiError::ok();
        let buffer = ndoc_compile_file_to_pdf(
            AsciiPointer::from_slice_with_nul(path_c.as_bytes_with_nul()).expect("ascii"),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok, "file compile must succeed");
        assert!(buffer.len >= 5, "PDF must be non-trivial");
        // SAFETY: reading the bytes we just received ownership of.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) };
        assert_eq!(&bytes[..5], b"%PDF-", "must be a PDF");
        ndoc_byte_buffer_free(buffer);
    }

    #[test]
    fn compile_file_to_pdf_missing_file_reports_structured_error() {
        let path = cstr("/nonexistent/path/missing.typ");
        let mut err = FfiError::ok();
        let buffer = ndoc_compile_file_to_pdf(
            AsciiPointer::from_slice_with_nul(path.as_bytes_with_nul()).expect("ascii"),
            &mut err,
        );
        assert_ne!(err.code, FfiErrorCode::Ok, "a missing file must fail");
        assert!(err.message.len > 0, "error message must be non-empty");
        assert!(buffer.data.is_null(), "failure yields the empty buffer");
        ndoc_byte_buffer_free(buffer);
        crate::error::ndoc_error_free(err);
    }

    #[test]
    fn compile_multi_png_yields_one_buffer_per_page() {
        let source = cstr("#set page(width: 90pt, height: 60pt)\nPNG body");
        let mut err = FfiError::ok();
        let result = ndoc_compile(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            FfiFormat::Png,
            96.0,
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert_eq!(result.format, FfiFormat::Png);
        assert_eq!(result.len, 1, "single-page fixture yields one buffer");
        // SAFETY: the result owns `len` buffers we may read before freeing.
        let buffers = unsafe { std::slice::from_raw_parts(result.buffers, result.len as usize) };
        let first = unsafe { std::slice::from_raw_parts(buffers[0].data, buffers[0].len as usize) };
        assert_eq!(
            &first[..4],
            b"\x89PNG",
            "buffer must carry the PNG signature"
        );
        ndoc_compile_result_free(result);
    }

    #[test]
    fn compile_multi_multipage_yields_one_buffer_per_page() {
        // A two-page fixture proves the buffer count tracks the page count, not a
        // hard-coded 1 (the single-page tests above cannot catch that regression).
        let source = cstr("#set page(width: 90pt, height: 60pt)\nPage one\n#pagebreak()\nPage two");
        let mut err = FfiError::ok();
        let result = ndoc_compile(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            FfiFormat::Svg,
            72.0,
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert_eq!(result.len, 2, "two pages yield two SVG buffers");
        // SAFETY: the result owns `len` buffers we may read before freeing.
        let buffers = unsafe { std::slice::from_raw_parts(result.buffers, result.len as usize) };
        for buffer in buffers {
            let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) };
            assert!(bytes.starts_with(b"<svg"), "each page is an SVG document");
        }
        ndoc_compile_result_free(result);
    }

    #[test]
    fn compile_multi_invalid_source_yields_empty_result_and_error() {
        // The forced-error path through the multi-format export: a mid-compile
        // failure must leave an empty (null, zero-length) result and a structured
        // error, with nothing leaked or unwound across the boundary.
        let source = cstr("#this_function_does_not_exist()");
        let mut err = FfiError::ok();
        let result = ndoc_compile(
            AsciiPointer::from_slice_with_nul(source.as_bytes_with_nul()).expect("ascii"),
            AsciiPointer::default(),
            FfiFormat::Svg,
            72.0,
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Compile);
        assert!(err.message.len > 0, "error message must be non-empty");
        assert_eq!(result.len, 0, "failure yields an empty result");
        assert!(result.buffers.is_null(), "no buffers on failure");
        ndoc_compile_result_free(result);
        crate::error::ndoc_error_free(err);
    }

    #[test]
    fn markdown_to_typst_round_trips_across_the_boundary() {
        let md = cstr("# Title\n\nBody text");
        let mut err = FfiError::ok();
        let typ = ndoc_markdown_to_typst(
            AsciiPointer::from_slice_with_nul(md.as_bytes_with_nul()).expect("ascii"),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert!(typ.len > 0, "converted Typst must be non-empty");
        // SAFETY: from_string stores valid UTF-8 of length `len`.
        let text = unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(typ.data, typ.len as usize))
        };
        assert!(text.contains("Title"), "expected heading text: {text}");
        ndoc_string_free(typ);
    }

    use crate::marshal::ndoc_string_free;
}
