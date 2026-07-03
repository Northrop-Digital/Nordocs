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

pub mod composed;
pub mod ndoc;

use crate::error::{Error, Result};

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

/// Whether `src` is a four-section authoring fat file (STATE/TEMPLATE/DOCUMENT/IMAGES).
///
/// Detection is marker-based, not extension-based: both the four-section
/// authoring format and the entry format ([`ndoc::NdocDocument`]) share the
/// `.ndoc.typ` extension. A four-section file's first non-empty line is the
/// `// === STATE ===` marker, whereas an entry-format file opens with the
/// `// ndoc document v1` header. This is the single point that decides which
/// parser a `.ndoc.typ` should be routed through.
pub fn is_four_section(src: &str) -> bool {
    src.lines()
        .map(str::trim_end)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line == markers::STATE)
}

/// Extract the four logical sections from a composed fat file.
///
/// Section bodies are the lines between consecutive markers, with the single
/// trailing blank separator line emitted by [`compose`] removed so that a
/// `compose` -> `extract` round-trip is lossless. Markers must appear in the
/// canonical order STATE, TEMPLATE, DOCUMENT, IMAGES.
///
/// # Errors
///
/// Returns [`Error::FatFile`] if the source is not a four-section fat file or a
/// required section marker is missing or out of order.
pub fn extract(src: &str) -> Result<FatFileSections> {
    let markers = [
        markers::STATE,
        markers::TEMPLATE,
        markers::DOCUMENT,
        markers::IMAGES,
    ];

    let lines: Vec<&str> = src.lines().collect();
    let mut marker_positions = Vec::with_capacity(markers.len());
    let mut search_from = 0usize;
    for marker in markers {
        let pos = lines[search_from..]
            .iter()
            .position(|line| line.trim_end() == marker)
            .map(|offset| search_from + offset)
            .ok_or_else(|| {
                Error::FatFile(format!(
                    "missing or out-of-order section marker: {marker:?}"
                ))
            })?;
        marker_positions.push(pos);
        search_from = pos + 1;
    }

    let body = |start_marker: usize, end: usize| -> String {
        let mut section: Vec<&str> = lines[start_marker + 1..end].to_vec();
        // compose() places a single blank separator line before each following
        // marker; drop it so round-tripping preserves the original body exactly.
        if section.last() == Some(&"") {
            section.pop();
        }
        section.join("\n")
    };

    Ok(FatFileSections {
        state: body(marker_positions[0], marker_positions[1]),
        template: body(marker_positions[1], marker_positions[2]),
        document: body(marker_positions[2], marker_positions[3]),
        images: body(marker_positions[3], lines.len()),
    })
}

/// Prefix on every IMAGES-section data line, keeping the section inert Typst.
const IMAGE_LINE_PREFIX: &str = "// ";

/// Parse the IMAGES section body into a `hash -> bytes` map.
///
/// Each non-blank data line has the shape `// {hash} {base64}` (the leading
/// `// ` keeps the section inert Typst). Lines without that shape, or whose
/// base64 payload does not decode, are skipped so a partially hand-edited
/// section never aborts a read; well-formed entries are still recovered.
pub fn parse_images_section(images: &str) -> std::collections::BTreeMap<String, Vec<u8>> {
    use base64::Engine as _;

    let mut map = std::collections::BTreeMap::new();
    for line in images.lines() {
        let Some(rest) = line.trim_end().strip_prefix(IMAGE_LINE_PREFIX) else {
            continue;
        };
        let Some((hash, payload)) = rest.split_once(' ') else {
            continue;
        };
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(payload.trim()) {
            map.insert(hash.to_string(), bytes);
        }
    }
    map
}

/// Render a `hash -> bytes` map back into an IMAGES section body.
///
/// Entries are emitted in `hash` order (the map is a `BTreeMap`) so the section
/// is deterministic across runs, which keeps snapshot output stable.
pub fn write_images_section(images: &std::collections::BTreeMap<String, Vec<u8>>) -> String {
    use base64::Engine as _;

    images
        .iter()
        .map(|(hash, bytes)| {
            let payload = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!("{IMAGE_LINE_PREFIX}{hash} {payload}")
        })
        .collect::<Vec<_>>()
        .join("\n")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_extract_round_trips_multiline_sections() {
        let sections = FatFileSections {
            state: "// {\"template\":\"x\"}\n// line two".to_string(),
            template: "#let a = 1".to_string(),
            document: String::new(),
            images: "// img-1 base64==".to_string(),
        };
        let composed = compose(&sections).expect("compose");
        let extracted = extract(&composed).expect("extract");
        assert_eq!(extracted, sections);
    }

    #[test]
    fn extract_errors_on_missing_marker() {
        let src = "// === STATE ===\nbody\n";
        let err = extract(src).expect_err("missing markers must error");
        assert!(
            err.to_string().contains("missing or out-of-order"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn is_four_section_classifies_state_format() {
        let composed = compose(&FatFileSections::default()).expect("compose");
        assert!(is_four_section(&composed));
    }

    #[test]
    fn is_four_section_rejects_entry_format() {
        let entry = ndoc::NdocDocument::new().compose();
        assert!(!is_four_section(&entry));
    }

    #[test]
    fn images_section_round_trips_bytes_by_hash() {
        let mut images = std::collections::BTreeMap::new();
        images.insert("aaaa".to_string(), vec![0u8, 1, 2, 255]);
        images.insert("bbbb".to_string(), b"PNG\x89".to_vec());

        let body = write_images_section(&images);
        for line in body.lines() {
            assert!(
                line.starts_with("// "),
                "image line must be inert: {line:?}"
            );
        }
        let parsed = parse_images_section(&body);
        assert_eq!(parsed, images);
    }

    #[test]
    fn images_section_skips_malformed_lines() {
        let body = "// not-a-valid-line\n// aaaa AAEC/w==\nrandom text";
        let parsed = parse_images_section(body);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.get("aaaa"), Some(&vec![0u8, 1, 2, 255]));
    }

    #[test]
    fn empty_images_section_parses_to_empty_map() {
        assert!(parse_images_section("").is_empty());
    }
}
