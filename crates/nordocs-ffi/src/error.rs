//! Structured FFI error marshalling.
//!
//! No `Result` and no panic may cross the ABI. Every fallible export instead
//! reports an [`FfiError`] — a flat `(code, message)` pair — produced by the
//! panic guard in [`crate::guard`]. [`FfiErrorCode`] mirrors the
//! [`nordocs_core::Error`] variants one-for-one, plus a [`FfiErrorCode::Panic`]
//! sentinel for a caught unwind.

use interoptopus::{ffi_function, ffi_type};

use crate::marshal::{ndoc_string_free, FfiString};

/// The category of an FFI failure.
///
/// `Ok` is the zero value (success); the remaining variants map one-for-one onto
/// [`nordocs_core::Error`], and `Panic` marks a Rust panic caught at the
/// boundary.
#[ffi_type]
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FfiErrorCode {
    /// No error.
    Ok = 0,
    /// `nordocs_core::Error::Authoring`.
    Authoring = 1,
    /// `nordocs_core::Error::Validation`.
    Validation = 2,
    /// `nordocs_core::Error::Schema`.
    Schema = 3,
    /// `nordocs_core::Error::Markdown`.
    Markdown = 4,
    /// `nordocs_core::Error::Compile`.
    Compile = 5,
    /// `nordocs_core::Error::FatFile`.
    FatFile = 6,
    /// `nordocs_core::Error::Io`.
    Io = 7,
    /// `nordocs_core::Error::Json`.
    Json = 8,
    /// A Rust panic was caught at the FFI boundary.
    Panic = 99,
}

impl From<&nordocs_core::Error> for FfiErrorCode {
    fn from(error: &nordocs_core::Error) -> Self {
        use nordocs_core::Error::*;
        match error {
            Authoring(_) => Self::Authoring,
            Validation(_) => Self::Validation,
            Schema(_) => Self::Schema,
            Markdown(_) => Self::Markdown,
            Compile(_) => Self::Compile,
            FatFile(_) => Self::FatFile,
            Io(_) => Self::Io,
            Json(_) => Self::Json,
        }
    }
}

/// A structured FFI error: a category code plus an owned UTF-8 message.
///
/// On success, exports leave this as [`FfiError::ok`] (code [`FfiErrorCode::Ok`],
/// empty message). On failure, `message` is a human-readable description owned by
/// the caller, who MUST release it with [`ndoc_error_free`] (or
/// [`ndoc_string_free`] on the `message` field). The empty `Ok` value needs no
/// free.
#[ffi_type]
#[repr(C)]
#[derive(Debug)]
pub struct FfiError {
    /// The failure category (`Ok` on success).
    pub code: FfiErrorCode,
    /// Owned, UTF-8, human-readable message (empty on success).
    pub message: FfiString,
}

impl FfiError {
    /// The success sentinel: [`FfiErrorCode::Ok`] with an empty message.
    pub fn ok() -> Self {
        Self {
            code: FfiErrorCode::Ok,
            message: FfiString::empty(),
        }
    }

    /// Build from a [`nordocs_core::Error`], capturing its `Display` message.
    pub fn from_core(error: &nordocs_core::Error) -> Self {
        Self {
            code: FfiErrorCode::from(error),
            message: FfiString::from_string(error.to_string()),
        }
    }

    /// Build a [`FfiErrorCode::Panic`] error with the caught panic message.
    pub fn panic(message: String) -> Self {
        Self {
            code: FfiErrorCode::Panic,
            message: FfiString::from_string(message),
        }
    }
}

/// Free the message owned by an [`FfiError`].
///
/// Single-call, like the other free exports. Passing an `Ok` error (empty
/// message) is a no-op.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_error_free(error: FfiError) {
    ndoc_string_free(error.message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_core_variant_to_a_distinct_code() {
        let cases = [
            (
                nordocs_core::Error::Authoring("a".into()),
                FfiErrorCode::Authoring,
            ),
            (
                nordocs_core::Error::Validation("v".into()),
                FfiErrorCode::Validation,
            ),
            (
                nordocs_core::Error::Schema("s".into()),
                FfiErrorCode::Schema,
            ),
            (
                nordocs_core::Error::Markdown("m".into()),
                FfiErrorCode::Markdown,
            ),
            (
                nordocs_core::Error::Compile("c".into()),
                FfiErrorCode::Compile,
            ),
            (
                nordocs_core::Error::FatFile("f".into()),
                FfiErrorCode::FatFile,
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(FfiErrorCode::from(&err), expected);
        }
    }

    #[test]
    fn ok_sentinel_has_empty_message() {
        let ok = FfiError::ok();
        assert_eq!(ok.code, FfiErrorCode::Ok);
        assert!(ok.message.data.is_null());
        ndoc_error_free(ok);
    }

    #[test]
    fn from_core_captures_code_and_message() {
        let err = FfiError::from_core(&nordocs_core::Error::Compile("boom".into()));
        assert_eq!(err.code, FfiErrorCode::Compile);
        assert!(err.message.len > 0);
        ndoc_error_free(err);
    }
}
