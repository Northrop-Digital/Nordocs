//! Source-map session handle exports.
//!
//! [`ndoc_compile_session`] compiles a source into an opaque [`CompiledSession`]
//! (a retained [`CompiledDoc`](nordocs_core::compiler::CompiledDoc)) and returns
//! a raw pointer the caller owns. Accessor exports export SVG/PNG per page, query
//! page geometry, and run the bidirectional source map (click → source, cursor →
//! preview) without recompiling. The caller MUST release the handle with
//! [`ndoc_session_free`] exactly once; passing a null handle anywhere is a guarded
//! no-op, so a C# wrapper that nulls its field after `Dispose` is double-free
//! safe.
//!
//! Every export runs under the panic guard and reports failure via `out_err`.

use interoptopus::patterns::string::AsciiPointer;
use interoptopus::{ffi_function, ffi_type};

use nordocs_core::compiler::CompiledDoc;
use nordocs_core::{Error, Result};

use crate::convert::{arg_str, arg_str_opt, parse_vars, settle};
use crate::error::FfiError;
use crate::guard::run_guarded;
use crate::marshal::{ByteBuffer, FfiString};

/// An opaque, retained compiled document the source map operates over.
///
/// Constructed by [`ndoc_compile_session`] and released by [`ndoc_session_free`].
/// The C side never sees its fields.
#[ffi_type(opaque)]
pub struct CompiledSession {
    inner: CompiledDoc,
}

/// The size of a page in typographic points (`pt`).
///
/// Returned by [`ndoc_session_page_size`]; zeroed when the call fails.
#[ffi_type]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FfiPageSize {
    /// Page width in points.
    pub width_pt: f64,
    /// Page height in points.
    pub height_pt: f64,
}

impl FfiPageSize {
    fn zero() -> Self {
        Self {
            width_pt: 0.0,
            height_pt: 0.0,
        }
    }
}

/// Run a session-accessor body under the panic guard.
///
/// A raw `*const CompiledSession` is not `UnwindSafe` (the retained document
/// holds interior-mutable Typst caches), so the captured handle is asserted
/// unwind-safe: the accessors only read through it, and on a caught panic the
/// guard returns a structured error without touching the document again.
fn guarded_session<T, F>(body: F) -> std::result::Result<T, FfiError>
where
    F: FnOnce() -> Result<T>,
{
    run_guarded(std::panic::AssertUnwindSafe(body))
}

/// Borrow a live session from a raw handle, erroring on a null pointer.
///
/// # Safety
/// `handle` must be null or a pointer returned by [`ndoc_compile_session`] that
/// has not yet been freed.
unsafe fn session<'a>(handle: *const CompiledSession) -> Result<&'a CompiledSession> {
    handle
        .as_ref()
        .ok_or_else(|| Error::Validation("null session handle".to_string()))
}

/// Compile a source into an opaque session handle (multi-format + source map).
///
/// `vars` is a JSON object of `sys.inputs` values (or null/empty for none).
/// Returns a non-null handle the caller owns on success; on failure returns null
/// and sets `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_compile_session(
    source: AsciiPointer,
    vars: AsciiPointer,
    out_err: &mut FfiError,
) -> *mut CompiledSession {
    let result = guarded_session(move || {
        let source = arg_str(source)?;
        let vars = parse_vars(&arg_str_opt(vars).unwrap_or_default())?;
        nordocs_core::service::compile_session(&source, &vars)
    });
    settle(
        result.map(|inner| Box::into_raw(Box::new(CompiledSession { inner }))),
        out_err,
        std::ptr::null_mut(),
    )
}

/// Release a session handle returned by [`ndoc_compile_session`].
///
/// Single-call. Passing a null handle is a no-op, so a wrapper that nulls its
/// field after the first free is double-free safe. Panics during free are
/// swallowed so nothing unwinds across the ABI.
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_free(handle: *mut CompiledSession) {
    if handle.is_null() {
        return;
    }
    // SAFETY: caller upholds the single-call contract; the handle came from
    // Box::into_raw in ndoc_compile_session. AssertUnwindSafe because the box is
    // reclaimed and dropped exactly once here.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(unsafe { Box::from_raw(handle) });
    }));
}

/// Number of laid-out pages in the session. Returns 0 and sets `out_err` on a
/// null handle.
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_page_count(
    handle: *const CompiledSession,
    out_err: &mut FfiError,
) -> u64 {
    let result = guarded_session(move || {
        // SAFETY: see `session`'s contract; the guard catches any misuse panic.
        let session = unsafe { session(handle) }?;
        Ok(session.inner.page_count() as u64)
    });
    settle(result, out_err, 0)
}

/// Size of the page at 0-based `index`, in points. Zeroed (and `out_err` set) when
/// the handle is null or the index is out of range.
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_page_size(
    handle: *const CompiledSession,
    index: u64,
    out_err: &mut FfiError,
) -> FfiPageSize {
    let result = guarded_session(move || {
        // SAFETY: see `session`'s contract.
        let session = unsafe { session(handle) }?;
        let (width_pt, height_pt) = session
            .inner
            .page_size(index as usize)
            .ok_or_else(|| Error::Validation(format!("page index {index} out of range")))?;
        Ok(FfiPageSize {
            width_pt,
            height_pt,
        })
    });
    settle(result, out_err, FfiPageSize::zero())
}

/// Export the page at 0-based `page` to an SVG string. Empty (and `out_err` set)
/// on failure.
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_svg(
    handle: *const CompiledSession,
    page: u64,
    out_err: &mut FfiError,
) -> FfiString {
    let result = guarded_session(move || {
        // SAFETY: see `session`'s contract.
        let session = unsafe { session(handle) }?;
        session.inner.to_svg(page as usize)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Render the page at 0-based `page` to PNG bytes at `dpi`. Empty buffer (and
/// `out_err` set) on failure.
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_png(
    handle: *const CompiledSession,
    page: u64,
    dpi: f32,
    out_err: &mut FfiError,
) -> ByteBuffer {
    let result = guarded_session(move || {
        // SAFETY: see `session`'s contract.
        let session = unsafe { session(handle) }?;
        session.inner.to_png(page as usize, dpi)
    });
    settle(
        result.map(ByteBuffer::from_vec),
        out_err,
        ByteBuffer::empty(),
    )
}

/// Map a click on a rendered page back to its source location (backward map).
///
/// `page` is 0-based; `x_pt`/`y_pt` are page-local coordinates in points. Returns
/// a JSON-serialised `Jump` (or `null` when the click resolves to no target).
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_jump_from_click(
    handle: *const CompiledSession,
    page: u64,
    x_pt: f64,
    y_pt: f64,
    out_err: &mut FfiError,
) -> FfiString {
    let result = guarded_session(move || {
        // SAFETY: see `session`'s contract.
        let session = unsafe { session(handle) }?;
        let jump =
            nordocs_core::service::jump_from_click(&session.inner, page as usize, x_pt, y_pt);
        Ok(serde_json::to_string(&jump)?)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Map a source cursor to the on-page positions it produced (forward map).
///
/// `file` is a virtual source path (the composed source is `main.typ`); `offset`
/// is a byte offset into it. Returns a JSON array of `Position`s (empty array when
/// the file is unknown or the cursor maps to no glyphs).
#[ffi_function]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ndoc_session_jump_from_cursor(
    handle: *const CompiledSession,
    file: AsciiPointer,
    offset: u64,
    out_err: &mut FfiError,
) -> FfiString {
    let result = guarded_session(move || {
        // SAFETY: see `session`'s contract.
        let session = unsafe { session(handle) }?;
        let file = arg_str(file)?;
        let positions = session.inner.jump_from_cursor(&file, offset as usize);
        Ok(serde_json::to_string(&positions)?)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ndoc_error_free, FfiErrorCode};
    use crate::marshal::{ndoc_byte_buffer_free, ndoc_string_free};
    use std::ffi::CString;

    fn ptr(s: &CString) -> AsciiPointer<'_> {
        AsciiPointer::from_slice_with_nul(s.as_bytes_with_nul()).expect("ascii")
    }

    fn read_string(s: &FfiString) -> String {
        // SAFETY: from_string stores valid UTF-8 of length `len`.
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(s.data, s.len as usize))
                .to_string()
        }
    }

    #[test]
    fn session_lifecycle_allocate_export_jump_free() {
        let source = CString::new("#set page(width: 120pt, height: 80pt, margin: 10pt)\nJump")
            .expect("source");
        let mut err = FfiError::ok();
        let handle = ndoc_compile_session(ptr(&source), AsciiPointer::default(), &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok, "session must compile");
        assert!(!handle.is_null(), "handle must be non-null");

        // Page geometry.
        let mut err = FfiError::ok();
        assert_eq!(ndoc_session_page_count(handle, &mut err), 1);
        assert_eq!(err.code, FfiErrorCode::Ok);
        let mut err = FfiError::ok();
        let size = ndoc_session_page_size(handle, 0, &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert!(
            (size.width_pt - 120.0).abs() < 0.01,
            "width: {}",
            size.width_pt
        );

        // SVG export.
        let mut err = FfiError::ok();
        let svg = ndoc_session_svg(handle, 0, &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert!(read_string(&svg).starts_with("<svg"), "must be an SVG");
        ndoc_string_free(svg);

        // PNG export.
        let mut err = FfiError::ok();
        let png = ndoc_session_png(handle, 0, 144.0, &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert!(png.len > 8);
        ndoc_byte_buffer_free(png);

        // Forward source map: a cursor inside the "Jump" text maps to a position.
        let file = CString::new("main.typ").expect("file");
        let mut err = FfiError::ok();
        let positions = ndoc_session_jump_from_cursor(handle, ptr(&file), 52, &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);
        let _: serde_json::Value =
            serde_json::from_str(&read_string(&positions)).expect("valid json array");
        ndoc_string_free(positions);

        // Backward source map: clicking empty space resolves to JSON null.
        let mut err = FfiError::ok();
        let jump = ndoc_session_jump_from_click(handle, 0, 115.0, 75.0, &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);
        assert_eq!(read_string(&jump), "null", "empty-space click is null");
        ndoc_string_free(jump);

        // Free once, then prove a second (nulled) free is a guarded no-op.
        ndoc_session_free(handle);
        ndoc_session_free(std::ptr::null_mut());
    }

    #[test]
    fn null_handle_reports_structured_error() {
        let mut err = FfiError::ok();
        let count = ndoc_session_page_count(std::ptr::null(), &mut err);
        assert_eq!(count, 0);
        assert_eq!(
            err.code,
            FfiErrorCode::Validation,
            "null handle is rejected"
        );
        ndoc_error_free(err);
    }

    #[test]
    fn compile_session_invalid_source_returns_null() {
        let source = CString::new("#this_function_does_not_exist()").expect("source");
        let mut err = FfiError::ok();
        let handle = ndoc_compile_session(ptr(&source), AsciiPointer::default(), &mut err);
        assert_eq!(err.code, FfiErrorCode::Compile);
        assert!(handle.is_null(), "failed compile yields a null handle");
        ndoc_error_free(err);
        // Freeing a null handle is safe.
        ndoc_session_free(handle);
    }
}
