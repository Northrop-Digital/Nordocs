//! Node-tree persistence for the four-section authoring fat file.
//!
//! The `doc` command group stores its [`Document`] model as comment-prefixed
//! serde-JSON inside the existing `// === STATE ===` section of the
//! four-section fat file ([`crate::fatfile`]). Because every line is prefixed
//! with `// `, the persisted state is inert Typst and survives compilation.
//!
//! Persistence deliberately reuses the existing `// === STATE ===` marker
//! rather than introducing a `// === DOC-STATE ===` marker: the entry-format
//! parser ([`crate::fatfile::ndoc::NdocDocument::parse`]) rejects any
//! unrecognised top-level line, so a new marker would collide with it. The
//! `doc` commands therefore always route through this four-section reader,
//! never through `NdocDocument::parse`.
//!
//! [`read_document`] extracts the STATE section, strips the `// ` prefix, and
//! deserialises the JSON. [`write_document`] does the inverse and persists via
//! the transactional [`super::ndoc::atomic_write`] (temp file + rename), so a
//! failed write never leaves a half-written document. Other sections
//! (TEMPLATE / DOCUMENT / IMAGES) are preserved across writes when the target
//! already exists, so writing the node tree never clobbers embedded images.

use std::path::Path;

use crate::error::{Error, Result};
use crate::fatfile::{self, FatFileSections};
use crate::model::{Document, ImageRef};

const STATE_LINE_PREFIX: &str = "// ";

/// Outcome of embedding an image into a four-section document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageEmbed {
    /// The image was newly recorded in the manifest and its bytes embedded.
    Added,
    /// An identical `{name, hash}` entry was already present; nothing changed.
    AlreadyPresent,
}

/// Embed `image_bytes` (named `image_name`) into the document at `path`.
///
/// Records a `{name, hash}` entry in the STATE-section image manifest and the
/// base64 bytes in the IMAGES section, keyed by the blake3 `hash` of the bytes.
/// Embedding is idempotent on identical content: re-embedding the same name and
/// content is a no-op that reports [`ImageEmbed::AlreadyPresent`]. Bytes shared
/// by several names are stored once (deduped by hash). The whole operation is
/// transactional via [`write_document`]'s atomic write.
///
/// # Errors
///
/// - [`Error::FatFile`] if `path` is not a four-section authoring document.
/// - [`Error::Authoring`] if a different image is already recorded under
///   `image_name` (same name, different content).
/// - [`Error::Io`] / [`Error::Json`] on read/parse/write failure.
pub fn embed_image(path: &Path, image_name: &str, image_bytes: &[u8]) -> Result<ImageEmbed> {
    let src = std::fs::read_to_string(path)?;
    if !fatfile::is_four_section(&src) {
        return Err(Error::FatFile(format!(
            "not a four-section authoring document: {}",
            path.display()
        )));
    }

    let mut sections = fatfile::extract(&src)?;
    let mut doc: Document = serde_json::from_str(&decode_state(&sections.state))?;
    let hash = blake3::hash(image_bytes).to_hex().to_string();

    if let Some(existing) = doc.images.iter().find(|img| img.name == image_name) {
        if existing.hash == hash {
            return Ok(ImageEmbed::AlreadyPresent);
        }
        return Err(Error::Authoring(format!(
            "image '{image_name}' is already embedded with different content"
        )));
    }

    doc.images.push(ImageRef {
        name: image_name.to_string(),
        hash: hash.clone(),
    });

    let mut bytes_by_hash = fatfile::parse_images_section(&sections.images);
    bytes_by_hash
        .entry(hash)
        .or_insert_with(|| image_bytes.to_vec());
    sections.images = fatfile::write_images_section(&bytes_by_hash);

    sections.state = encode_state(&serde_json::to_string_pretty(&doc)?);

    let composed = fatfile::compose(&sections)?;
    super::ndoc::atomic_write(path, &composed)?;
    Ok(ImageEmbed::Added)
}

/// Read the [`Document`] persisted in a four-section fat file's STATE section.
///
/// # Errors
///
/// - [`Error::Io`] if `path` is missing or unreadable.
/// - [`Error::FatFile`] if `path` is not a four-section authoring file.
/// - [`Error::Json`] if the STATE section is not a valid serialised document.
pub fn read_document(path: &Path) -> Result<Document> {
    let src = std::fs::read_to_string(path)?;
    if !fatfile::is_four_section(&src) {
        return Err(Error::FatFile(format!(
            "not a four-section authoring document: {}",
            path.display()
        )));
    }
    let sections = fatfile::extract(&src)?;
    let json = decode_state(&sections.state);
    Ok(serde_json::from_str(&json)?)
}

/// Atomically persist `doc` into a four-section fat file's STATE section.
///
/// If `path` already exists and is a four-section file, its TEMPLATE / DOCUMENT
/// / IMAGES sections are preserved; only STATE is rewritten. Otherwise the other
/// sections start empty.
///
/// # Errors
///
/// - [`Error::Json`] if `doc` cannot be serialised.
/// - [`Error::FatFile`] if composition fails.
/// - [`Error::Io`] if the atomic write fails.
pub fn write_document(path: &Path, doc: &Document) -> Result<()> {
    let mut sections = match std::fs::read_to_string(path) {
        Ok(src) if fatfile::is_four_section(&src) => fatfile::extract(&src)?,
        _ => FatFileSections::default(),
    };

    let json = serde_json::to_string_pretty(doc)?;
    sections.state = encode_state(&json);

    let composed = fatfile::compose(&sections)?;
    super::ndoc::atomic_write(path, &composed)
}

/// Comment-prefix each line of `json` so the STATE block is inert Typst.
fn encode_state(json: &str) -> String {
    json.lines()
        .map(|line| {
            if line.is_empty() {
                "//".to_string()
            } else {
                format!("{STATE_LINE_PREFIX}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip the `// ` comment prefix from each STATE line, recovering the JSON.
fn decode_state(state: &str) -> String {
    state
        .lines()
        .map(|line| {
            line.strip_prefix(STATE_LINE_PREFIX)
                .or_else(|| line.strip_prefix("//"))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{InputKind, InputValue, Node, NodeId};
    use std::collections::BTreeMap;

    fn sample_document() -> Document {
        let mut doc_inputs = BTreeMap::new();
        doc_inputs.insert(
            "title".to_string(),
            InputValue {
                kind: InputKind::String,
                value: serde_json::json!("My Doc"),
            },
        );

        let mut node_inputs = BTreeMap::new();
        node_inputs.insert(
            "body".to_string(),
            InputValue {
                kind: InputKind::Content,
                value: serde_json::json!("Hello **world**"),
            },
        );
        node_inputs.insert(
            "cover".to_string(),
            InputValue {
                kind: InputKind::Image,
                value: serde_json::json!("logo.png"),
            },
        );

        Document {
            template: "article".to_string(),
            inputs: doc_inputs,
            nodes: vec![Node {
                id: NodeId("section-aabb".to_string()),
                component: "section".to_string(),
                inputs: BTreeMap::new(),
                children: vec![Node {
                    id: NodeId("para-0001".to_string()),
                    component: "paragraph".to_string(),
                    inputs: node_inputs,
                    children: Vec::new(),
                }],
            }],
            images: Vec::new(),
        }
    }

    #[test]
    fn write_then_read_round_trips_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        let doc = sample_document();

        write_document(&path, &doc).expect("write");
        let read_back = read_document(&path).expect("read");

        assert_eq!(read_back, doc);
    }

    #[test]
    fn snapshot_composed_state_fat_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        let composed = std::fs::read_to_string(&path).expect("read composed");
        insta::assert_snapshot!(composed);
    }

    #[test]
    fn state_block_lines_are_comment_prefixed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        let src = std::fs::read_to_string(&path).expect("read raw");
        let state_body: Vec<&str> = src
            .lines()
            .skip_while(|l| *l != fatfile::markers::STATE)
            .skip(1)
            .take_while(|l| !l.is_empty())
            .collect();
        assert!(!state_body.is_empty(), "STATE block must not be empty");
        for line in state_body {
            assert!(
                line.starts_with("//"),
                "every STATE line must be a comment: {line:?}"
            );
        }
    }

    #[test]
    fn read_rejects_entry_format_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("entry.ndoc.typ");
        std::fs::write(&path, "// ndoc document v1\n").expect("write entry file");

        let err = read_document(&path).expect_err("entry-format must be rejected");
        assert!(
            err.to_string().contains("four-section"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn read_missing_file_is_io_error() {
        let err = read_document(Path::new("/nonexistent/doc.ndoc.typ"))
            .expect_err("missing file must error");
        assert!(matches!(err, Error::Io(_)), "unexpected error: {err}");
    }

    #[test]
    fn write_preserves_other_sections() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");

        let seeded = FatFileSections {
            state: super::encode_state("{}"),
            template: "#let tpl = 1".to_string(),
            document: String::new(),
            images: "// logo base64==".to_string(),
        };
        let composed = fatfile::compose(&seeded).expect("compose seed");
        std::fs::write(&path, &composed).expect("seed file");

        write_document(&path, &sample_document()).expect("write");
        let src = std::fs::read_to_string(&path).expect("read raw");
        let sections = fatfile::extract(&src).expect("extract");

        assert_eq!(sections.template, "#let tpl = 1");
        assert_eq!(sections.images, "// logo base64==");
    }

    #[test]
    fn embed_image_records_manifest_and_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        let outcome = embed_image(&path, "logo.png", b"PNGDATA").expect("embed");
        assert_eq!(outcome, ImageEmbed::Added);

        let doc = read_document(&path).expect("read back");
        assert_eq!(doc.images.len(), 1);
        assert_eq!(doc.images[0].name, "logo.png");
        let expected_hash = blake3::hash(b"PNGDATA").to_hex().to_string();
        assert_eq!(doc.images[0].hash, expected_hash);

        let src = std::fs::read_to_string(&path).expect("read raw");
        let bytes = fatfile::parse_images_section(&fatfile::extract(&src).expect("extract").images);
        assert_eq!(bytes.get(&expected_hash), Some(&b"PNGDATA".to_vec()));
    }

    #[test]
    fn embed_image_is_idempotent_on_identical_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        assert_eq!(
            embed_image(&path, "logo.png", b"SAME").expect("first embed"),
            ImageEmbed::Added
        );
        assert_eq!(
            embed_image(&path, "logo.png", b"SAME").expect("second embed"),
            ImageEmbed::AlreadyPresent
        );

        let doc = read_document(&path).expect("read back");
        assert_eq!(doc.images.len(), 1, "identical re-embed must not duplicate");
    }

    #[test]
    fn embed_image_dedupes_shared_content_across_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        embed_image(&path, "a.png", b"SHARED").expect("embed a");
        embed_image(&path, "b.png", b"SHARED").expect("embed b");

        let doc = read_document(&path).expect("read back");
        assert_eq!(doc.images.len(), 2, "two manifest entries");

        let src = std::fs::read_to_string(&path).expect("read raw");
        let bytes = fatfile::parse_images_section(&fatfile::extract(&src).expect("extract").images);
        assert_eq!(bytes.len(), 1, "shared content stored once");
    }

    #[test]
    fn embed_image_rejects_name_with_different_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        embed_image(&path, "logo.png", b"ONE").expect("first embed");
        let err = embed_image(&path, "logo.png", b"TWO").expect_err("conflict must error");
        assert!(
            matches!(err, Error::Authoring(_)),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn embed_image_rejects_entry_format_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("entry.ndoc.typ");
        std::fs::write(&path, "// ndoc document v1\n").expect("write entry file");

        let err = embed_image(&path, "logo.png", b"X").expect_err("entry format rejected");
        assert!(
            err.to_string().contains("four-section"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn composed_state_document_compiles_to_non_empty_pdf() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        write_document(&path, &sample_document()).expect("write");

        let src = std::fs::read_to_string(&path).expect("read raw");
        let pdf = crate::compiler::compile_to_pdf(&src).expect("compile STATE fat file");
        assert!(!pdf.is_empty(), "composed STATE file must compile to a PDF");
    }
}
