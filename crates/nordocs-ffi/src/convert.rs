//! Shared input/output marshalling helpers for the FFI exports.
//!
//! These convert the flat, C-friendly argument types ([`AsciiPointer`],
//! [`interoptopus::patterns::slice::FFISlice`]) into owned Rust values, and
//! settle a guarded [`Result`](nordocs_core::Result) into a concrete return
//! value plus the out-parameter [`FfiError`]. Keeping the conversions here means
//! every export marshals inputs and reports errors the same way.

use interoptopus::patterns::string::AsciiPointer;

use nordocs_core::{Error, Result};

use crate::error::FfiError;

/// Extract a required UTF-8 string argument, erroring on a null/invalid pointer.
///
/// A null pointer or non-UTF-8 bytes become an [`Error::Validation`] so the
/// boundary reports a structured error instead of reading invalid memory.
pub(crate) fn arg_str(ptr: AsciiPointer) -> Result<String> {
    ptr.as_str()
        .map(str::to_owned)
        .map_err(|_| Error::Validation("null or non-UTF-8 string argument".to_string()))
}

/// Extract an optional UTF-8 string argument, mapping a null pointer to `None`.
///
/// Used for genuinely optional inputs (e.g. an override theme); a present but
/// non-UTF-8 pointer still yields `None` rather than failing the whole call.
pub(crate) fn arg_str_opt(ptr: AsciiPointer) -> Option<String> {
    ptr.as_str().ok().map(str::to_owned)
}

/// Parse the `vars` JSON object into the ordered `sys.inputs` pairs the engine
/// consumes.
///
/// An empty or whitespace-only string means "no inputs". Otherwise the input
/// must be a JSON object whose values are coerced to strings (string values are
/// taken verbatim; non-string values are rendered with their JSON form). A
/// malformed object surfaces as [`Error::Json`].
pub(crate) fn parse_vars(json: &str) -> Result<Vec<(String, String)>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(trimmed)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| {
            let value = match v {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            (k, value)
        })
        .collect())
}

/// Settle a guarded result into a concrete value plus the out-parameter error.
///
/// On success, `out_err` is set to [`FfiError::ok`] and the value is returned; on
/// failure, `out_err` receives the structured error and `fallback` is returned.
pub(crate) fn settle<T>(
    result: std::result::Result<T, FfiError>,
    out_err: &mut FfiError,
    fallback: T,
) -> T {
    match result {
        Ok(value) => {
            *out_err = FfiError::ok();
            value
        }
        Err(error) => {
            *out_err = error;
            fallback
        }
    }
}

/// Settle a guarded unit result into just the out-parameter error.
pub(crate) fn settle_unit(result: std::result::Result<(), FfiError>, out_err: &mut FfiError) {
    *out_err = match result {
        Ok(()) => FfiError::ok(),
        Err(error) => error,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vars_empty_is_no_inputs() {
        assert!(parse_vars("").expect("empty parses").is_empty());
        assert!(parse_vars("   ").expect("blank parses").is_empty());
    }

    #[test]
    fn parse_vars_object_coerces_values_to_strings() {
        let pairs = parse_vars(r#"{"title":"Hi","count":3}"#).expect("object parses");
        assert!(pairs.contains(&("title".to_string(), "Hi".to_string())));
        assert!(pairs.contains(&("count".to_string(), "3".to_string())));
    }

    #[test]
    fn parse_vars_malformed_is_json_error() {
        let err = parse_vars("not json").expect_err("malformed must error");
        assert!(matches!(err, Error::Json(_)));
    }
}
