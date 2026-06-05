//! Snapshot tests for `markdown_to_typst`.
//!
//! These tests freeze the rendered Typst output for representative Markdown
//! constructs. Run `cargo insta review` after updating the implementation to
//! review and accept new snapshots.

use northdoc::markdown::markdown_to_typst;

fn convert(md: &str) -> String {
    markdown_to_typst(md).expect("conversion should not fail")
}

#[test]
fn snapshot_paragraphs() {
    let md = "Hello, world.\n\nAnother paragraph.";
    insta::assert_snapshot!(convert(md));
}

#[test]
fn snapshot_headings() {
    let md = "# Heading 1\n\n## Heading 2\n\n### Heading 3\n\n#### Heading 4";
    insta::assert_snapshot!(convert(md));
}

#[test]
fn snapshot_table() {
    let md = "| Left | Center | Right |\n|:-----|:------:|------:|\n| l1   |   c1   |    r1 |\n| l2   |   c2   |    r2 |";
    insta::assert_snapshot!(convert(md));
}

#[test]
fn snapshot_tasklist() {
    let md = "- [x] Done\n- [ ] Todo\n- [x] Also done";
    insta::assert_snapshot!(convert(md));
}

#[test]
fn snapshot_strikethrough() {
    let md = "Normal text and ~~struck through~~ and back to normal.";
    insta::assert_snapshot!(convert(md));
}

#[test]
fn snapshot_footnotes() {
    let md = "First reference[^a] and second[^b].\n\n[^a]: Footnote alpha.\n[^b]: Footnote beta.";
    insta::assert_snapshot!(convert(md));
}
