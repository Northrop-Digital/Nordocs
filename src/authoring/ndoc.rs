//! Transactional helper functions for `.ndoc.typ` document authoring.
//!
//! Each function performs a read-validate-write cycle:
//!
//! 1. Read the target file from disk.
//! 2. Parse it into an [`NdocDocument`].
//! 3. Apply the mutation to the in-memory model.
//! 4. Recompose to text.
//! 5. Write to a sibling temp file, then `rename` atomically to the target.
//!
//! The atomic rename ensures no partial writes are ever visible to callers.

use std::path::Path;

use crate::error::{Error, Result};
use crate::fatfile::ndoc::{compute_entry_hash, EntryKind, NdocDocument, NdocEntry};

/// Create a new, empty `.ndoc.typ` document at `path`.
///
/// # Errors
///
/// Returns `Error::Io` if `path` already exists or if any write fails.
pub fn create_document(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("document already exists: {}", path.display()),
        )));
    }
    let doc = NdocDocument::new();
    atomic_write(path, &doc.compose())
}

/// Add a named entry to an existing document at `path`.
///
/// Content is accepted as a `&str`; the caller is responsible for reading from
/// a file or stdin before calling this function.
///
/// # Errors
///
/// - `Error::Io` if `path` does not exist or if any write fails.
/// - `Error::Authoring("duplicate-entry …")` if an entry named `name` already
///   exists in the document.
pub fn add_entry(path: &Path, name: &str, kind: EntryKind, content: &str) -> Result<()> {
    let src = read_document(path)?;
    let mut doc = NdocDocument::parse(&src)?;

    if doc.entries.iter().any(|e| e.name == name) {
        return Err(Error::Authoring(format!(
            "duplicate-entry: an entry named {name:?} already exists in {}",
            path.display()
        )));
    }

    doc.entries.push(NdocEntry {
        name: name.to_string(),
        kind,
        content: content.to_string(),
        hash: compute_entry_hash(content),
    });

    atomic_write(path, &doc.compose())
}

/// Replace the content of a named entry in an existing document at `path`.
///
/// # Errors
///
/// - `Error::Io` if `path` does not exist or if any write fails.
/// - `Error::Authoring("entry-not-found …")` if no entry named `name` exists
///   in the document.
pub fn edit_entry(path: &Path, name: &str, content: &str) -> Result<()> {
    let src = read_document(path)?;
    let mut doc = NdocDocument::parse(&src)?;

    let entry = doc
        .entries
        .iter_mut()
        .find(|e| e.name == name)
        .ok_or_else(|| {
            Error::Authoring(format!(
                "entry-not-found: no entry named {name:?} in {}",
                path.display()
            ))
        })?;

    entry.content = content.to_string();
    entry.hash = compute_entry_hash(content);

    atomic_write(path, &doc.compose())
}

/// Read a document file into a `String`.
///
/// Maps the underlying `io::Error` to `Error::Io` so callers see a consistent
/// error type when the file is missing or unreadable.
fn read_document(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

/// Write `content` to a sibling temp file, then rename atomically to `target`.
///
/// On any failure the temp file is removed and the original target is left
/// untouched.  The temp file is placed in the same directory as `target` to
/// guarantee `rename` stays on the same filesystem.
pub(crate) fn atomic_write(target: &Path, content: &str) -> Result<()> {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("ndoc"))
        .to_string_lossy();
    let tmp_path = dir.join(format!(".{}.{}.tmp", file_name, std::process::id()));

    if let Err(e) = std::fs::write(&tmp_path, content) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(Error::Io(e));
    }

    if let Err(e) = std::fs::rename(&tmp_path, target) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(Error::Io(e));
    }

    Ok(())
}
