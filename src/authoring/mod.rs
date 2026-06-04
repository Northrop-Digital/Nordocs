//! Transactional document authoring over fat files.
//!
//! Ports the C# `DocumentAuthoringService`: a transactional read-validate-write
//! cycle over `.ndoc.typ` fat files backing the `doc` CLI command group
//! (new / outline / add / remove / set / patch / search / batch-add / schema).
//!
//! Each mutation reads the current fat file, applies the change to the in-memory
//! [`Document`] model, validates the result, and only then re-composes and
//! writes — so a failed validation never leaves a half-written document.

use crate::error::Result;
use crate::model::Document;

/// A single authoring transaction against a document.
///
/// Skeleton: holds the working document; concrete mutation methods (add_node,
/// remove_node, set_input, patch, ...) are added as the command group is ported.
pub struct AuthoringTransaction {
    document: Document,
}

impl AuthoringTransaction {
    /// Begin a transaction over an already-parsed document.
    pub fn new(document: Document) -> Self {
        Self { document }
    }

    /// Borrow the working document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Validate the working document, returning an error on the first problem.
    ///
    /// Skeleton always succeeds; real structural/input validation (the C#
    /// `DocumentNodeTreeValidator` / `ItemValidator`) is wired in next.
    pub fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// Commit the transaction, consuming it and returning the final document.
    pub fn commit(self) -> Result<Document> {
        self.validate()?;
        Ok(self.document)
    }
}
