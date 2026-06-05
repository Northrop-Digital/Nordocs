//! Schema-based validation for `.ndoc.typ` and `.md` documents.
//!
//! The two public entry points validate against the built-in catalogue and
//! return a [`ValidationResult`] listing every violation found. Neither
//! function fails fast on the first error — all violations are collected
//! before returning.

use std::path::Path;

use crate::error::Result;
use crate::fatfile::ndoc::NdocDocument;
use crate::schema::{Catalogue, ConstraintKind};

/// A single schema violation found during validation.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// Identifies the location of the problem (file path, entry name, …).
    pub location: String,
    /// Human-readable description of the violation.
    pub message: String,
}

/// The output of a validation run.
///
/// An empty `violations` list means the document is valid.
#[derive(Debug, Default, Clone)]
pub struct ValidationResult {
    pub violations: Vec<Violation>,
}

impl ValidationResult {
    /// `true` when no violations were found.
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Validate a `.ndoc.typ` fat-file document against the built-in catalogue.
///
/// Reads and parses the document at `path`. If parsing fails (e.g. a missing
/// or malformed document header) the parse error is recorded as a
/// [`Violation`] rather than propagated as an `Err`. Structural constraints
/// that survive parsing — currently entry-name uniqueness — are checked
/// without fail-fast behaviour.
///
/// Returns `Err` only for I/O failures.
pub fn validate_ndoc_file(path: &Path) -> Result<ValidationResult> {
    let src = std::fs::read_to_string(path)?;
    let mut violations = Vec::new();

    let doc = match NdocDocument::parse(&src) {
        Ok(d) => d,
        Err(e) => {
            violations.push(Violation {
                location: path.display().to_string(),
                message: e.to_string(),
            });
            return Ok(ValidationResult { violations });
        }
    };

    let catalogue = Catalogue::from_builtins();

    for rule in &catalogue.document_constraints.rules {
        match rule.kind {
            ConstraintKind::EntryNameUniqueness => {
                let mut seen = std::collections::HashSet::new();
                for entry in &doc.entries {
                    if !seen.insert(entry.name.as_str()) {
                        violations.push(Violation {
                            location: format!("{}:{}", path.display(), entry.name),
                            message: format!(
                                "duplicate entry name '{}': {}",
                                entry.name, rule.description
                            ),
                        });
                    }
                }
            }
            // HeaderPresence and ValidEntryKind are enforced by NdocDocument::parse;
            // reaching this point means those constraints are already satisfied.
            // MarkdownFrontmatterValid applies only to .md files.
            ConstraintKind::HeaderPresence
            | ConstraintKind::ValidEntryKind
            | ConstraintKind::MarkdownFrontmatterValid => {}
        }
    }

    Ok(ValidationResult { violations })
}

/// Validate a `.md` Markdown file.
///
/// Checks that the optional YAML frontmatter block (if present) parses
/// without errors. Returns an unsupported-file-type [`Violation`] for any
/// path whose extension is not `md`.
///
/// Returns `Err` only for I/O failures.
pub fn validate_markdown_file(path: &Path) -> Result<ValidationResult> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension != "md" {
        return Ok(ValidationResult {
            violations: vec![Violation {
                location: path.display().to_string(),
                message: format!(
                    "unsupported file type '{}': expected a '.ndoc.typ' or '.md' file",
                    path.display()
                ),
            }],
        });
    }

    let src = std::fs::read_to_string(path)?;
    let mut violations = Vec::new();

    let catalogue = Catalogue::from_builtins();

    if catalogue
        .document_constraints
        .rules
        .iter()
        .any(|r| r.kind == ConstraintKind::MarkdownFrontmatterValid)
    {
        if let Some(fm_str) = extract_frontmatter(&src) {
            if let Err(e) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(fm_str) {
                violations.push(Violation {
                    location: format!("{}:frontmatter", path.display()),
                    message: format!("invalid YAML frontmatter: {e}"),
                });
            }
        }
    }

    Ok(ValidationResult { violations })
}

/// Extract the raw YAML content between a leading `---` frontmatter fence.
///
/// Returns `None` if the source does not begin with a `---` delimiter.
fn extract_frontmatter(src: &str) -> Option<&str> {
    let body = src
        .strip_prefix("---\n")
        .or_else(|| src.strip_prefix("---\r\n"))?;

    let end = body
        .find("\n---\n")
        .or_else(|| body.find("\n---\r\n"))
        .or_else(|| body.find("\n---"));

    end.map(|pos| &body[..pos])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fatfile::ndoc::{compute_entry_hash, EntryKind, NdocDocument, NdocEntry};

    fn write_temp(suffix: &str, content: &str) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new()
            .suffix(suffix)
            .tempfile()
            .expect("temp file");
        std::fs::write(file.path(), content).expect("write temp file");
        file
    }

    // --- validate_ndoc_file ---

    #[test]
    fn validate_valid_ndoc_file_no_violations() {
        let doc = NdocDocument::new();
        let raw = doc.compose();
        let file = write_temp(".ndoc.typ", &raw);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            result.is_valid(),
            "empty ndoc document must have no violations"
        );
    }

    #[test]
    fn validate_ndoc_file_invalid_header_returns_violation() {
        let file = write_temp(".ndoc.typ", "not a valid ndoc document\n");
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "malformed header must produce at least one violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("invalid document header"),
            "violation message must describe the bad header"
        );
    }

    #[test]
    fn validate_ndoc_file_duplicate_entry_names_returns_violation() {
        let mut doc = NdocDocument::new();
        doc.entries.push(NdocEntry {
            name: "mycomp".to_string(),
            kind: EntryKind::Component,
            content: "first".to_string(),
            hash: compute_entry_hash("first"),
        });
        doc.entries.push(NdocEntry {
            name: "mycomp".to_string(),
            kind: EntryKind::Component,
            content: "second".to_string(),
            hash: compute_entry_hash("second"),
        });
        let raw = doc.compose();
        let file = write_temp(".ndoc.typ", &raw);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "duplicate entry names must produce at least one violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("duplicate entry name"),
            "violation message must name the constraint"
        );
    }

    // --- validate_markdown_file ---

    #[test]
    fn validate_valid_markdown_file_no_violations() {
        let content = "---\ntitle: Test Doc\n---\n\n# Heading\n\nParagraph.\n";
        let file = write_temp(".md", content);
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(result.is_valid(), "valid markdown must have no violations");
    }

    #[test]
    fn validate_markdown_file_no_frontmatter_no_violations() {
        let content = "# Simple Heading\n\nJust a paragraph.\n";
        let file = write_temp(".md", content);
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(
            result.is_valid(),
            "markdown without frontmatter must have no violations"
        );
    }

    #[test]
    fn validate_markdown_file_invalid_frontmatter_returns_violation() {
        // Deliberately unclosed flow sequence to force a YAML parse error.
        let content = "---\ntitle: [unclosed bracket\n---\n\n# Heading\n";
        let file = write_temp(".md", content);
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "invalid YAML frontmatter must produce at least one violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("invalid YAML frontmatter"),
            "violation message must describe the frontmatter problem"
        );
    }

    #[test]
    fn validate_unsupported_extension_returns_violation() {
        let file = write_temp(".txt", "some content\n");
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "unsupported extension must produce a violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("unsupported file type"),
            "violation message must flag the unsupported type"
        );
    }
}
