//! The self-contained `.ndoc.typ` fat file and the `ndoc` document format.
//!
//! Two distinct formats live here:
//!
//! - The original **fat file** (this module) — four fixed sections
//!   (STATE / TEMPLATE / DOCUMENT / IMAGES) for the render pipeline.
//! - The **ndoc document** ([`ndoc`]) — a variable list of named entries
//!   (components/templates) for the P2 document authoring commands.
//!
//! Both formats target `.ndoc.typ` files but use distinct marker prefixes
//! (`// === STATE ===` vs `// === NDOC-ENTRY: ...`) so they cannot be confused
//! during parsing.

pub mod ndoc;

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
// TODO: P3 — replace DefaultHasher with blake3 for cross-run stability.
pub fn content_hash(composed: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    composed.hash(&mut hasher);
    hasher.finish()
}
