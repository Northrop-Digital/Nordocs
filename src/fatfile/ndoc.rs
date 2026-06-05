//! The `.ndoc.typ` document format: named entry sections with blake3 hashing.
//!
//! An ndoc document is a line-oriented text file that bundles named Typst
//! building blocks (components, templates) into a single managed file.  Each
//! entry is delimited by `NDOC-ENTRY` / `NDOC-END` marker comments.  The stored
//! hash in the start marker enables callers to detect changed entries without
//! re-reading entry content.
//!
//! Format (one entry):
//! ```text
//! // === NDOC-ENTRY: {name} kind={component|template} hash={64-char-blake3-hex} ===
//! {opaque Typst content}
//! // === NDOC-END: {name} ===
//! ```
//!
//! An empty document contains only the header line:
//! ```text
//! // ndoc document v1
//! ```

use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};

const DOCUMENT_HEADER: &str = "// ndoc document v1";
const ENTRY_PREFIX: &str = "// === NDOC-ENTRY: ";
const ENTRY_SUFFIX: &str = " ===";
const END_PREFIX: &str = "// === NDOC-END: ";

/// Whether an entry holds a component definition or a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Component,
    Template,
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryKind::Component => f.write_str("component"),
            EntryKind::Template => f.write_str("template"),
        }
    }
}

impl FromStr for EntryKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "component" => Ok(EntryKind::Component),
            "template" => Ok(EntryKind::Template),
            other => Err(Error::FatFile(format!("unknown entry kind: {other:?}"))),
        }
    }
}

/// A named entry inside an [`NdocDocument`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdocEntry {
    /// Unique name within the document.
    pub name: String,
    pub kind: EntryKind,
    /// Raw Typst source content (no surrounding delimiters).
    pub content: String,
    /// Blake3 hex digest stored in the file's start marker.
    ///
    /// Set by [`NdocDocument::compose`] at write time and read back verbatim by
    /// [`NdocDocument::parse`] without recomputing.  Use
    /// [`NdocEntry::is_content_changed`] to check whether the stored hash still
    /// agrees with the current content.
    pub hash: String,
}

impl NdocEntry {
    /// Returns `true` when the current `content` no longer matches the stored
    /// `hash`.  A freshly composed-and-parsed entry always returns `false`.
    pub fn is_content_changed(&self) -> bool {
        compute_entry_hash(&self.content) != self.hash
    }
}

/// An ndoc document: an ordered list of named [`NdocEntry`] values.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NdocDocument {
    pub entries: Vec<NdocEntry>,
}

impl NdocDocument {
    /// Create an empty document with no entries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialise the document to `.ndoc.typ` text.
    ///
    /// Hashes are recomputed from each entry's current content; the `hash`
    /// field stored on each [`NdocEntry`] is **not** used — it reflects what
    /// was last read from disk.
    pub fn compose(&self) -> String {
        let mut out = String::from(DOCUMENT_HEADER);
        out.push('\n');

        for entry in &self.entries {
            let hash = compute_entry_hash(&entry.content);
            out.push_str(ENTRY_PREFIX);
            out.push_str(&entry.name);
            out.push_str(&format!(" kind={} hash={hash}{ENTRY_SUFFIX}\n", entry.kind));
            out.push_str(&entry.content);
            out.push('\n');
            out.push_str(END_PREFIX);
            out.push_str(&entry.name);
            out.push_str(ENTRY_SUFFIX);
            out.push('\n');
        }

        out
    }

    /// Parse a `.ndoc.typ` source string into an [`NdocDocument`].
    ///
    /// The stored hash is read verbatim from each entry's start marker; it is
    /// not recomputed.  Returns an error if the source is not a valid ndoc
    /// document.
    pub fn parse(src: &str) -> Result<Self> {
        let mut lines = src.lines();

        let header = lines
            .next()
            .ok_or_else(|| Error::FatFile("empty document".to_string()))?;
        if header != DOCUMENT_HEADER {
            return Err(Error::FatFile(format!(
                "invalid document header: {header:?}"
            )));
        }

        let mut entries: Vec<NdocEntry> = Vec::new();

        loop {
            match lines.next() {
                None => break,
                Some(line) if line.starts_with(ENTRY_PREFIX) && line.ends_with(ENTRY_SUFFIX) => {
                    let (name, kind, hash) = parse_entry_header(line)?;
                    let end_marker = format!("{END_PREFIX}{name}{ENTRY_SUFFIX}");

                    let mut content_lines: Vec<&str> = Vec::new();
                    loop {
                        match lines.next() {
                            None => {
                                return Err(Error::FatFile(format!(
                                    "missing NDOC-END marker for entry {name:?}"
                                )));
                            }
                            Some(l) if l == end_marker => break,
                            Some(l) => content_lines.push(l),
                        }
                    }

                    // Reconstruct content: join lines with \n.
                    // A single empty line (content="") round-trips as vec![""] -> "".
                    // A trailing \n in content produces a trailing "" element -> correct.
                    let content = content_lines.join("\n");

                    entries.push(NdocEntry {
                        name,
                        kind,
                        content,
                        hash,
                    });
                }
                Some("") => {
                    // Trailing blank lines after the last entry are ignored.
                }
                Some(line) => {
                    return Err(Error::FatFile(format!(
                        "unexpected line in document: {line:?}"
                    )));
                }
            }
        }

        Ok(NdocDocument { entries })
    }
}

/// Parse the `kind=` and `hash=` attributes from an `NDOC-ENTRY` marker line.
///
/// Expected format: `// === NDOC-ENTRY: {name} kind={kind} hash={hash} ===`
fn parse_entry_header(line: &str) -> Result<(String, EntryKind, String)> {
    let inner = line
        .strip_prefix(ENTRY_PREFIX)
        .and_then(|s| s.strip_suffix(ENTRY_SUFFIX))
        .ok_or_else(|| Error::FatFile(format!("malformed NDOC-ENTRY marker: {line:?}")))?;

    let mut parts = inner.splitn(3, ' ');

    let name = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::FatFile(format!("missing entry name in: {inner:?}")))?
        .to_string();

    let kind_token = parts
        .next()
        .ok_or_else(|| Error::FatFile(format!("missing kind= in: {inner:?}")))?;
    let kind_str = kind_token
        .strip_prefix("kind=")
        .ok_or_else(|| Error::FatFile(format!("malformed kind token: {kind_token:?}")))?;
    let kind: EntryKind = kind_str.parse()?;

    let hash_token = parts
        .next()
        .ok_or_else(|| Error::FatFile(format!("missing hash= in: {inner:?}")))?;
    let hash = hash_token
        .strip_prefix("hash=")
        .ok_or_else(|| Error::FatFile(format!("malformed hash token: {hash_token:?}")))?
        .to_string();

    Ok((name, kind, hash))
}

/// Compute a 64-character lowercase blake3 hex digest of `content`.
pub fn compute_entry_hash(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}
