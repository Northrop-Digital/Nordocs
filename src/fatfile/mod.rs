//! The self-contained `.ndoc.typ` "fat file".
//!
//! A fat file bundles everything needed to render a document into one `.typ`:
//!
//! - `STATE` — JSON prelude: document inputs, per-node component inputs, and the
//!   images manifest.
//! - `TEMPLATE` — the theme variables plus component function definitions.
//! - `DOCUMENT` — `doc.update(...)` plus the component call tree.
//! - `IMAGES` — embedded image payloads referenced by the manifest.
//!
//! This module composes those sections into a single source string, extracts
//! them back out, and hashes for change detection. Ported from the C#
//! `FatFileService`.

use crate::error::Result;

/// The four logical sections of a fat file, kept as raw strings during compose.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FatFileSections {
    pub state: String,
    pub template: String,
    pub document: String,
    pub images: String,
}

/// Marker comments that delimit sections inside the composed `.typ`.
pub mod markers {
    pub const STATE: &str = "// === STATE ===";
    pub const TEMPLATE: &str = "// === TEMPLATE ===";
    pub const DOCUMENT: &str = "// === DOCUMENT ===";
    pub const IMAGES: &str = "// === IMAGES ===";
}

/// Compose the four sections into a single `.ndoc.typ` source string.
pub fn compose(sections: &FatFileSections) -> Result<String> {
    let mut out = String::new();
    out.push_str(markers::STATE);
    out.push('\n');
    out.push_str(&sections.state);
    out.push_str("\n\n");
    out.push_str(markers::TEMPLATE);
    out.push('\n');
    out.push_str(&sections.template);
    out.push_str("\n\n");
    out.push_str(markers::DOCUMENT);
    out.push('\n');
    out.push_str(&sections.document);
    out.push_str("\n\n");
    out.push_str(markers::IMAGES);
    out.push('\n');
    out.push_str(&sections.images);
    out.push('\n');
    Ok(out)
}

/// A stable content hash of the composed fat file, used for change detection.
///
/// Uses the standard-library hasher for the skeleton; swap for a cryptographic
/// digest if cross-run stability across toolchains is required.
pub fn content_hash(composed: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    composed.hash(&mut hasher);
    hasher.finish()
}
