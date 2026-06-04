//! Markdown -> Typst conversion.
//!
//! Replaces the C# Markdig-based converter. Uses `comrak` (CommonMark + full
//! GFM: tables, task lists, strikethrough, footnotes) whose arena-backed typed
//! AST suits the recursive block/inline walk the C# converter performed.
//!
//! This skeleton wires the parser and exposes the conversion entrypoint; the
//! node-by-node Typst emission is filled in as the pipeline is ported.

use comrak::{Arena, Options};

use crate::error::Result;

/// Convert a Markdown string into an equivalent Typst markup string.
///
/// The current skeleton parses with full GFM extensions enabled and returns the
/// input wrapped as a Typst raw block placeholder. Real AST-walk emission lands
/// as the converter is ported from C#.
pub fn markdown_to_typst(markdown: &str) -> Result<String> {
    let arena = Arena::new();
    let options = gfm_options();
    let _root = comrak::parse_document(&arena, markdown, &options);

    // TODO(port): walk `_root` and emit Typst markup per node kind.
    Ok(format!("// converted from markdown\n{markdown}"))
}

/// Comrak options with the GFM extensions the C# converter relied on.
fn gfm_options() -> Options {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.autolink = true;
    options
}
