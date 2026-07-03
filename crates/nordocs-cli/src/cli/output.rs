//! JSON output envelope for `--json` mode.
//!
//! Every subcommand that supports `--json` calls [`emit_json_result`] on success.
//! [`emit_json_error`] is called by `main.rs` when a top-level error occurs with
//! `--json` active.

use serde::Serialize;

/// Structured envelope emitted to stdout when `--json` is active.
///
/// Success shape:  `{"status":"ok","data":<command-specific or null>}`
/// Error shape:    `{"status":"error","message":"<actionable text>"}`
#[derive(Debug, Serialize)]
pub struct JsonEnvelope {
    /// `"ok"` on success, `"error"` on failure.
    pub status: &'static str,
    /// Command-specific payload; present on success, absent on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Human-readable description; present on error, absent on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Emit a success JSON envelope to stdout.
///
/// `data` is the command-specific payload (`null` for commands with no structured output).
pub fn emit_json_result(data: Option<serde_json::Value>) {
    let envelope = JsonEnvelope {
        status: "ok",
        data,
        message: None,
    };
    println!(
        "{}",
        serde_json::to_string(&envelope)
            .unwrap_or_else(|_| r#"{"status":"ok","data":null}"#.to_owned())
    );
}

/// Emit an error JSON envelope to stdout.
///
/// Called when `--json` is active and the command encountered an error. The
/// caller is responsible for exiting with a non-zero code after this returns.
pub fn emit_json_error(message: &str) {
    let envelope = JsonEnvelope {
        status: "error",
        data: None,
        message: Some(message.to_owned()),
    };
    println!(
        "{}",
        serde_json::to_string(&envelope).unwrap_or_else(|_| {
            format!(
                r#"{{"status":"error","message":"{}"}}"#,
                message.replace('\\', r"\\").replace('"', r#"\""#)
            )
        })
    );
}
