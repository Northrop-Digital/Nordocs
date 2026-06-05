//! Markdown -> Typst conversion.
//!
//! Replaces the C# Markdig-based converter. Uses `comrak` (CommonMark + full
//! GFM: tables, task lists, strikethrough, footnotes) whose arena-backed typed
//! AST suits the recursive block/inline walk the C# converter performed.
//!
//! Conversion is a two-phase process:
//! 1. Parse with comrak and collect footnote definitions by name.
//! 2. Walk the AST and emit Typst markup, substituting `#footnote[...]` at
//!    each `FootnoteReference` site.

use std::collections::HashMap;

use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};
use comrak::{Arena, Options};

use crate::error::Result;

/// Convert a Markdown string into an equivalent Typst markup string.
///
/// Returns an empty string for empty or whitespace-only input. All supported
/// CommonMark and GFM constructs (tables, task lists, strikethrough, footnotes)
/// are translated; unknown constructs are silently dropped.
pub fn markdown_to_typst(markdown: &str) -> Result<String> {
    if markdown.trim().is_empty() {
        return Ok(String::new());
    }

    let arena = Arena::new();
    let options = gfm_options();
    let root = comrak::parse_document(&arena, markdown, &options);

    let footnotes = collect_footnotes(root);

    let mut out = String::new();
    render_document(root, &footnotes, &mut out);

    Ok(out.trim_end().to_owned())
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

/// First pass: collect all `FootnoteDefinition` nodes at the document root and
/// render their paragraph body to a map keyed by footnote name.
fn collect_footnotes<'a>(root: &'a AstNode<'a>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for child in root.children() {
        let value = child.data.borrow().value.clone();
        if let NodeValue::FootnoteDefinition(fd) = value {
            let mut body = String::new();
            for def_child in child.children() {
                let def_value = def_child.data.borrow().value.clone();
                if let NodeValue::Paragraph = def_value {
                    // Footnote bodies do not resolve nested footnote refs.
                    render_inlines(def_child, &HashMap::new(), &mut body);
                }
            }
            map.insert(fd.name.clone(), body.trim_end().to_owned());
        }
    }
    map
}

/// Render the `Document` root, inserting a blank line between top-level blocks
/// and skipping `FootnoteDefinition` nodes (consumed by the first pass).
fn render_document<'a>(
    root: &'a AstNode<'a>,
    footnotes: &HashMap<String, String>,
    out: &mut String,
) {
    let mut first = true;
    for child in root.children() {
        if matches!(child.data.borrow().value, NodeValue::FootnoteDefinition(_)) {
            continue;
        }
        if !first {
            out.push('\n');
        }
        render_block(child, footnotes, out, 0);
        first = false;
    }
}

/// Render a block node, appending Typst markup to `out`.
///
/// `depth` tracks nesting level for list indentation.
fn render_block<'a>(
    node: &'a AstNode<'a>,
    footnotes: &HashMap<String, String>,
    out: &mut String,
    depth: usize,
) {
    let value = node.data.borrow().value.clone();
    match value {
        NodeValue::Heading(h) => {
            out.push_str(&"=".repeat(h.level as usize));
            out.push(' ');
            render_inlines(node, footnotes, out);
            out.push('\n');
        }
        NodeValue::Paragraph => {
            render_inlines(node, footnotes, out);
            out.push('\n');
        }
        NodeValue::BlockQuote => {
            out.push_str("#quote(block: true)[\n");
            for child in node.children() {
                let child_val = child.data.borrow().value.clone();
                match child_val {
                    NodeValue::Paragraph => {
                        render_inlines(child, footnotes, out);
                        out.push('\n');
                    }
                    _ => render_block(child, footnotes, out, depth),
                }
            }
            out.push_str("]\n");
        }
        NodeValue::List(list_meta) => {
            render_list(node, list_meta.list_type, footnotes, out, depth);
        }
        NodeValue::CodeBlock(cb) => {
            if cb.info.is_empty() {
                out.push_str("```\n");
            } else {
                out.push_str("```");
                out.push_str(&cb.info);
                out.push('\n');
            }
            out.push_str(&cb.literal);
            if !cb.literal.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        NodeValue::HtmlBlock(html) => {
            out.push_str(&html.literal);
        }
        NodeValue::ThematicBreak => {
            out.push_str("#line(length: 100%)\n");
        }
        NodeValue::Table(table_meta) => {
            render_table(node, &table_meta, footnotes, out);
        }
        // These are rendered by their parent; skip if visited directly.
        NodeValue::Item(_)
        | NodeValue::TaskItem(_)
        | NodeValue::TableRow(_)
        | NodeValue::TableCell
        | NodeValue::FootnoteDefinition(_) => {}
        _ => {
            for child in node.children() {
                render_block(child, footnotes, out, depth);
            }
        }
    }
}

/// Render a list (ordered or unordered), handling task items and nested lists.
///
/// Each `Item` paragraph gets the list marker prepended. `TaskItem` nodes get a
/// checkbox symbol (`☑` or `☐`) instead of the bullet. Nested lists are
/// rendered at `depth + 1` with two-space indentation per level.
fn render_list<'a>(
    node: &'a AstNode<'a>,
    list_type: ListType,
    footnotes: &HashMap<String, String>,
    out: &mut String,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    let is_ordered = list_type == ListType::Ordered;

    for child in node.children() {
        let child_val = child.data.borrow().value.clone();
        match child_val {
            NodeValue::TaskItem(checked) => {
                let checkbox = if checked.is_some() { "☑" } else { "☐" };
                let mut wrote_prefix = false;
                for item_child in child.children() {
                    let ic_val = item_child.data.borrow().value.clone();
                    match ic_val {
                        NodeValue::Paragraph => {
                            if !wrote_prefix {
                                out.push_str(&indent);
                                out.push_str("- ");
                                out.push_str(checkbox);
                                out.push(' ');
                                wrote_prefix = true;
                            }
                            render_inlines(item_child, footnotes, out);
                            out.push('\n');
                        }
                        NodeValue::List(nl) => {
                            render_list(item_child, nl.list_type, footnotes, out, depth + 1);
                        }
                        _ => {}
                    }
                }
            }
            NodeValue::Item(_) => {
                let marker = if is_ordered { "+" } else { "-" };
                for item_child in child.children() {
                    let ic_val = item_child.data.borrow().value.clone();
                    match ic_val {
                        NodeValue::Paragraph => {
                            out.push_str(&indent);
                            out.push_str(marker);
                            out.push(' ');
                            render_inlines(item_child, footnotes, out);
                            out.push('\n');
                        }
                        NodeValue::List(nl) => {
                            render_list(item_child, nl.list_type, footnotes, out, depth + 1);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// Render a GFM table, mapping column alignments to Typst `align` arguments.
fn render_table<'a>(
    node: &'a AstNode<'a>,
    table_meta: &comrak::nodes::NodeTable,
    footnotes: &HashMap<String, String>,
    out: &mut String,
) {
    let col_count = table_meta.num_columns;
    if col_count == 0 {
        return;
    }

    let alignments: Vec<&str> = table_meta
        .alignments
        .iter()
        .map(|a| match a {
            TableAlignment::Center => "center",
            TableAlignment::Right => "right",
            _ => "left",
        })
        .collect();

    out.push_str("#table(columns: ");
    out.push_str(&col_count.to_string());
    out.push_str(", align: (");
    out.push_str(&alignments.join(", "));
    out.push_str("),\n");

    for row in node.children() {
        if !matches!(row.data.borrow().value, NodeValue::TableRow(_)) {
            continue;
        }
        for cell in row.children() {
            if !matches!(cell.data.borrow().value, NodeValue::TableCell) {
                continue;
            }
            out.push_str("  [");
            render_inlines(cell, footnotes, out);
            out.push_str("],");
        }
        out.push('\n');
    }

    out.push_str(")\n");
}

/// Render the inline children of `node`, appending to `out`.
fn render_inlines<'a>(
    node: &'a AstNode<'a>,
    footnotes: &HashMap<String, String>,
    out: &mut String,
) {
    for child in node.children() {
        render_inline(child, footnotes, out);
    }
}

/// Render a single inline node, appending to `out`.
fn render_inline<'a>(node: &'a AstNode<'a>, footnotes: &HashMap<String, String>, out: &mut String) {
    let value = node.data.borrow().value.clone();
    match value {
        NodeValue::Text(text) => {
            out.push_str(&escape_typst(&text));
        }
        NodeValue::SoftBreak => {
            out.push('\n');
        }
        NodeValue::LineBreak => {
            out.push_str(" \\\n");
        }
        NodeValue::Code(c) => {
            out.push('`');
            out.push_str(&c.literal);
            out.push('`');
        }
        NodeValue::HtmlInline(html) => {
            out.push_str(&html);
        }
        NodeValue::Emph => {
            out.push('_');
            render_inlines(node, footnotes, out);
            out.push('_');
        }
        NodeValue::Strong => {
            out.push('*');
            render_inlines(node, footnotes, out);
            out.push('*');
        }
        NodeValue::Strikethrough => {
            out.push_str("#strike[");
            render_inlines(node, footnotes, out);
            out.push(']');
        }
        NodeValue::Link(link) => {
            out.push_str("#link(\"");
            out.push_str(&link.url);
            out.push_str("\")[");
            render_inlines(node, footnotes, out);
            out.push(']');
        }
        NodeValue::Image(img) => {
            out.push_str("#image(\"");
            out.push_str(&img.url);
            out.push_str("\")");
        }
        NodeValue::FootnoteReference(fr) => {
            if let Some(body) = footnotes.get(&fr.name) {
                out.push_str("#footnote[");
                out.push_str(body);
                out.push(']');
            }
        }
        _ => {
            render_inlines(node, footnotes, out);
        }
    }
}

/// Escape characters that have special meaning in Typst markup.
///
/// Applied to all literal `Text` nodes so that user content cannot accidentally
/// trigger Typst math, headings, emphasis, or other markup.
fn escape_typst(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('#', "\\#")
        .replace('@', "\\@")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('$', "\\$")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(md: &str) -> String {
        markdown_to_typst(md).expect("conversion should not fail")
    }

    #[test]
    fn markdown_to_typst_empty() {
        assert_eq!(convert(""), "");
        assert_eq!(convert("   \n\t  "), "");
    }

    #[test]
    fn markdown_to_typst_headings() {
        assert_eq!(convert("# H1"), "= H1");
        assert_eq!(convert("## H2"), "== H2");
        assert_eq!(convert("### H3"), "=== H3");
        assert_eq!(convert("#### H4"), "==== H4");
    }

    #[test]
    fn markdown_to_typst_bold_italic() {
        assert_eq!(convert("**bold**"), "*bold*");
        assert_eq!(convert("*italic*"), "_italic_");
    }

    #[test]
    fn markdown_to_typst_escape() {
        // $ and @ are never special in CommonMark; they land as literal Text nodes.
        assert_eq!(convert("$10"), "\\$10");
        assert_eq!(convert("user@host"), "user\\@host");
        // # in the middle of a word (not at line start) is literal text.
        assert_eq!(convert("foo#bar"), "foo\\#bar");
    }

    #[test]
    fn markdown_to_typst_link() {
        assert_eq!(
            convert("[text](https://example.com)"),
            "#link(\"https://example.com\")[text]"
        );
    }

    #[test]
    fn markdown_to_typst_image() {
        assert_eq!(convert("![alt](img.png)"), "#image(\"img.png\")");
    }

    #[test]
    fn markdown_to_typst_code_inline() {
        assert_eq!(convert("`code`"), "`code`");
    }

    #[test]
    fn markdown_to_typst_code_block_with_lang() {
        let input = "```rust\nfn main() {}\n```";
        let expected = "```rust\nfn main() {}\n```";
        assert_eq!(convert(input), expected);
    }

    #[test]
    fn markdown_to_typst_code_block_no_lang() {
        let input = "```\nplain\n```";
        let expected = "```\nplain\n```";
        assert_eq!(convert(input), expected);
    }

    #[test]
    fn markdown_to_typst_unordered_list() {
        assert_eq!(convert("- a\n- b"), "- a\n- b");
    }

    #[test]
    fn markdown_to_typst_ordered_list() {
        assert_eq!(convert("1. a\n2. b"), "+ a\n+ b");
    }

    #[test]
    fn markdown_to_typst_nested_list() {
        let input = "- parent\n  - child";
        let expected = "- parent\n  - child";
        assert_eq!(convert(input), expected);
    }

    #[test]
    fn markdown_to_typst_blockquote() {
        assert_eq!(convert("> text"), "#quote(block: true)[\ntext\n]");
    }

    #[test]
    fn markdown_to_typst_thematic_break() {
        assert_eq!(convert("---"), "#line(length: 100%)");
    }

    #[test]
    fn markdown_to_typst_strikethrough() {
        assert_eq!(convert("~~text~~"), "#strike[text]");
    }

    #[test]
    fn markdown_to_typst_task_list() {
        assert_eq!(convert("- [x] Done"), "- ☑ Done");
        assert_eq!(convert("- [ ] Todo"), "- ☐ Todo");
    }

    #[test]
    fn markdown_to_typst_footnote() {
        let input = "See[^1].\n\n[^1]: Note.";
        assert_eq!(convert(input), "See#footnote[Note.].");
    }
}
