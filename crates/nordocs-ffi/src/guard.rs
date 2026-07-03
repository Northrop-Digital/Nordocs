//! The FFI panic guard.
//!
//! Every fallible export runs its body through [`run_guarded`], which executes
//! it under [`std::panic::catch_unwind`] and converts **both** an
//! `Err(nordocs_core::Error)` and a caught panic into a structured
//! [`FfiError`]. No `Result` and no panic ever cross the ABI: the export hands
//! back the success value (or a default) plus an out-parameter [`FfiError`].

use std::any::Any;
use std::panic::{catch_unwind, UnwindSafe};

use crate::error::FfiError;

/// Run an export body under the panic guard.
///
/// Returns `Ok(value)` when `body` succeeds, or `Err(FfiError)` when it returns
/// an `Err` (mapped via [`FfiError::from_core`]) or panics (mapped via
/// [`FfiError::panic`], with the unwind payload's message extracted where
/// possible). The returned [`FfiError`] owns a heap message that must be freed.
///
/// Exports added in later task groups call this, translate `Ok`/`Err` into their
/// concrete return value plus an out-parameter `FfiError`, and never let a panic
/// escape.
#[allow(dead_code)] // Consumed by the compile/markdown/preview/session exports (task groups 2–5).
pub(crate) fn run_guarded<T, F>(body: F) -> Result<T, FfiError>
where
    F: FnOnce() -> nordocs_core::Result<T> + UnwindSafe,
{
    match catch_unwind(body) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(core_error)) => Err(FfiError::from_core(&core_error)),
        Err(payload) => Err(FfiError::panic(panic_message(payload))),
    }
}

/// Best-effort extraction of a human-readable message from an unwind payload.
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic at the FFI boundary (non-string payload)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ndoc_error_free, FfiErrorCode};

    #[test]
    fn forced_compile_error_becomes_a_clean_structured_error() {
        // A genuine compile failure through the real embedded compiler: calling
        // an undefined function. The guard must surface this as a structured
        // error, not crash or abort.
        let result = run_guarded(|| {
            nordocs_core::service::compile_to_pdf("#this_function_does_not_exist()")
        });
        let error = result.expect_err("invalid Typst must fail to compile");
        assert_eq!(error.code, FfiErrorCode::Compile);
        assert!(error.message.len > 0, "expected a non-empty error message");
        ndoc_error_free(error);
    }

    #[test]
    fn caught_panic_becomes_a_panic_coded_error() {
        // Silence the default panic hook so the deliberate panic does not spam
        // test output; catch_unwind still intercepts the unwind.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = run_guarded::<(), _>(|| panic!("boom from inside the boundary"));
        std::panic::set_hook(previous);

        let error = result.expect_err("a panic must be caught, not propagated");
        assert_eq!(error.code, FfiErrorCode::Panic);
        assert!(
            error.message.len > 0,
            "expected the panic message to be captured"
        );
        ndoc_error_free(error);
    }

    #[test]
    fn success_passes_the_value_through() {
        let value = run_guarded(|| Ok(7u32)).expect("ok body yields its value");
        assert_eq!(value, 7);
    }
}
