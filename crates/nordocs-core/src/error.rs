//! Typed library errors.
//!
//! Application/CLI code uses `anyhow` for ergonomic context chaining; the
//! library layers surface these typed variants so callers (and the eventual
//! AgentTools surface) can match on failure categories. Mirrors the C#
//! `AuthoringError` / `ValidationError` / `FileError` split.

use thiserror::Error;

/// Convenience alias for results returned by nordocs library functions.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level typed error for the nordocs library.
#[derive(Debug, Error)]
pub enum Error {
    /// A document authoring operation failed (compose, patch, set, etc.).
    #[error("authoring error: {0}")]
    Authoring(String),

    /// Document structure or input validation failed.
    #[error("validation error: {0}")]
    Validation(String),

    /// Schema parsing or resolution failed.
    #[error("schema error: {0}")]
    Schema(String),

    /// Markdown -> Typst conversion failed.
    #[error("markdown conversion error: {0}")]
    Markdown(String),

    /// Typst compilation or PDF export failed.
    #[error("compile error: {0}")]
    Compile(String),

    /// Fat-file compose/extract failed.
    #[error("fat-file error: {0}")]
    FatFile(String),

    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
