//! Snapshot tests for fat-file composition.

use nordocs_core::fatfile::{compose, FatFileSections};

/// Composing a minimal fat file produces a stable, section-delimited `.typ`.
#[test]
fn compose_minimal_fat_file() {
    let sections = FatFileSections {
        state: "{ \"inputs\": {} }".to_string(),
        template: "#let theme = (brand: blue)".to_string(),
        document: "#doc.update(())".to_string(),
        images: "// no images".to_string(),
    };

    let composed = compose(&sections).expect("compose succeeds");
    insta::assert_snapshot!(composed);
}
