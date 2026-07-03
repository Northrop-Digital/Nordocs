//! Snapshot and unit tests for `fatfile::ndoc`.
//!
//! Snapshot tests freeze the composed `.ndoc.typ` text for empty, single-entry,
//! and multi-entry documents.  Unit tests verify round-trip parse/compose
//! symmetry and hash-based change detection.

use nordocs_core::fatfile::ndoc::{compute_entry_hash, EntryKind, NdocDocument, NdocEntry};

fn component(name: &str, content: &str) -> NdocEntry {
    NdocEntry {
        name: name.to_string(),
        kind: EntryKind::Component,
        content: content.to_string(),
        hash: compute_entry_hash(content),
    }
}

fn template_entry(name: &str, content: &str) -> NdocEntry {
    NdocEntry {
        name: name.to_string(),
        kind: EntryKind::Template,
        content: content.to_string(),
        hash: compute_entry_hash(content),
    }
}

// ---------------------------------------------------------------------------
// Snapshot tests — freeze the `.ndoc.typ` text format
// ---------------------------------------------------------------------------

#[test]
fn compose_empty_document() {
    let doc = NdocDocument::new();
    insta::assert_snapshot!(doc.compose());
}

#[test]
fn compose_single_entry() {
    let mut doc = NdocDocument::new();
    doc.entries.push(component("hero", "#let hero = ()"));
    insta::assert_snapshot!(doc.compose());
}

#[test]
fn compose_multi_entry() {
    let mut doc = NdocDocument::new();
    doc.entries.push(component("hero", "#let hero = ()"));
    doc.entries.push(template_entry("page", "#let page = ()"));
    insta::assert_snapshot!(doc.compose());
}

// ---------------------------------------------------------------------------
// Round-trip unit tests — parse(compose(doc)) == doc
// ---------------------------------------------------------------------------

#[test]
fn round_trip_single_entry() {
    let mut doc = NdocDocument::new();
    doc.entries.push(component("hero", "#let hero = ()"));
    let composed = doc.compose();
    let parsed = NdocDocument::parse(&composed).expect("parse should succeed");
    assert_eq!(doc, parsed);
}

#[test]
fn round_trip_multi_entry() {
    let mut doc = NdocDocument::new();
    doc.entries.push(component("hero", "#let hero = ()"));
    doc.entries.push(template_entry("page", "#let page = ()"));
    let composed = doc.compose();
    let parsed = NdocDocument::parse(&composed).expect("parse should succeed");
    assert_eq!(doc, parsed);
}

// ---------------------------------------------------------------------------
// Hash change detection unit tests (REQ-006)
// ---------------------------------------------------------------------------

#[test]
fn hash_stability_unchanged() {
    let entry = component("hero", "#let hero = ()");
    assert!(
        !entry.is_content_changed(),
        "is_content_changed() must return false when content matches stored hash"
    );
}

#[test]
fn hash_changes_after_edit() {
    let mut entry = component("hero", "#let hero = ()");
    entry.content = "#let hero = (updated: true)".to_string();
    assert!(
        entry.is_content_changed(),
        "is_content_changed() must return true after content is modified"
    );
}
