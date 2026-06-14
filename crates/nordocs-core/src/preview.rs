//! Component and document preview composition.
//!
//! Ports the C# reference `PreviewRenderer` (over `FatFileService.Compose` plus
//! `DocumentGenerator` / `StateBlockWriter`) into the binding-agnostic core. A
//! preview is produced by composing the live Typst that a single component (or a
//! whole document) needs to render — the TEMPLATE section (theme + the
//! `image-or-placeholder` runtime helper + component definitions) followed by the
//! DOCUMENT section (the `doc` state update plus the component-invocation tree) —
//! and compiling it with the embedded compiler.
//!
//! These functions are I/O-free: they take owned content (sources, schemas,
//! input values) and return PDF bytes, so the `ndoc` CLI and the `.NET` FFI can
//! both drive live preview without any file-system or process side effects.
//!
//! Unlike the persisted four-section fat file ([`crate::fatfile`]) or the
//! canonical composed reader ([`crate::fatfile::composed`]), the preview
//! composition only emits the *live* TEMPLATE/DOCUMENT Typst the compiler needs —
//! the inert STATE/IMAGES sections used for round-tripping are not required for a
//! one-shot render and are omitted.

use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use crate::error::Result;
use crate::model::{Document, InputKind, InputValue, Node, NodeId};
use crate::schema::ComponentSchema;

/// The minimal theme used when a component preview supplies no theme of its own.
///
/// Mirrors the reference `PreviewRenderer.MinimalTheme`: a frontmatter header
/// (stripped before composition) plus the `theme` / `doc` state declarations the
/// composed document expects to exist.
const MINIMAL_THEME: &str = "/*---\nthemeId: preview\nversion: \"0.0.0\"\n---*/\n\n\
     #let theme = state(\"theme\", (:))\n#let doc = state(\"doc\", (:))";

/// Runtime helper injected into every composed preview's TEMPLATE section.
///
/// Ported verbatim from the reference `FatFileService.Compose`: component authors
/// call `image-or-placeholder(name, ..)` unconditionally; an empty/none `name`
/// renders a dashed placeholder instead of an embedded image.
const IMAGE_HELPER: &str = "// --- Runtime helper: image-or-placeholder ---\n\
     #let image-or-placeholder(\n\
     \x20 name,\n\
     \x20 width: auto,\n\
     \x20 height: auto,\n\
     \x20 placeholder-width: auto,\n\
     \x20 placeholder-height: auto,\n\
     \x20 ..args,\n\
     ) = {\n\
     \x20 if name == none or name == \"\" {\n\
     \x20\x20\x20 rect(\n\
     \x20\x20\x20\x20\x20 width: placeholder-width,\n\
     \x20\x20\x20\x20\x20 height: placeholder-height,\n\
     \x20\x20\x20\x20\x20 stroke: (dash: \"dashed\", paint: luma(180), thickness: 1pt),\n\
     \x20\x20\x20\x20\x20 radius: 12pt,\n\
     \x20\x20\x20 )[\n\
     \x20\x20\x20\x20\x20 #align(center + horizon)[\n\
     \x20\x20\x20\x20\x20\x20\x20 #text(size: 10pt, fill: luma(150))[Photo]\n\
     \x20\x20\x20\x20\x20 ]\n\
     \x20\x20\x20 ]\n\
     \x20 } else {\n\
     \x20\x20\x20 image(\"images/\" + name, width: width, height: height, ..args)\n\
     \x20 }\n\
     }";

/// Render a single component to PDF for live preview.
///
/// Mirrors the reference `IPreviewRenderer.RenderComponentPreview`: schema inputs
/// are seeded with type defaults, overridden by any supplied `input_values`, and
/// the component is invoked from a one-node document composed against `theme_code`
/// (or the [`MINIMAL_THEME`] when `None`). The composed source is compiled with
/// the embedded compiler and returned as PDF bytes.
///
/// # Errors
///
/// Returns [`crate::error::Error::Markdown`] if a content input fails to convert,
/// or [`crate::error::Error::Compile`] if the composed document does not compile.
pub fn render_component_preview(
    component_source: &str,
    schema: &ComponentSchema,
    input_values: &[(String, Value)],
    theme_code: Option<&str>,
) -> Result<Vec<u8>> {
    // Seed every declared input with its type default, then apply overrides.
    let mut inputs: BTreeMap<String, InputValue> = BTreeMap::new();
    for input in &schema.inputs {
        inputs.insert(
            input.name.clone(),
            InputValue {
                kind: input.kind,
                value: default_for_kind(input.kind),
            },
        );
    }
    for (name, value) in input_values {
        let kind = schema
            .inputs
            .iter()
            .find(|i| &i.name == name)
            .map(|i| i.kind)
            .unwrap_or(InputKind::String);
        inputs.insert(
            name.clone(),
            InputValue {
                kind,
                value: value.clone(),
            },
        );
    }

    let node = Node {
        id: NodeId::mint(&schema.name, &HashSet::new()),
        component: schema.name.clone(),
        inputs,
        children: Vec::new(),
    };

    let state = Document {
        template: "preview".to_string(),
        inputs: BTreeMap::new(),
        nodes: vec![node],
        images: Vec::new(),
    };

    let mut component_sources = BTreeMap::new();
    component_sources.insert(schema.name.clone(), component_source.to_string());
    let mut component_schemas = BTreeMap::new();
    component_schemas.insert(schema.name.clone(), schema.clone());

    let source = compose_preview(
        &state,
        theme_code.unwrap_or(MINIMAL_THEME),
        &component_sources,
        &component_schemas,
    )?;
    crate::compiler::compile_to_pdf(&source)
}

/// Render a complete document to PDF for live preview.
///
/// Mirrors the reference `IPreviewRenderer.RenderDocumentPreview`: the document
/// `state`, its `theme_code`, and the `component_sources` / `component_schemas`
/// maps are composed into a single Typst source and compiled with the embedded
/// compiler.
///
/// # Errors
///
/// Returns [`crate::error::Error::Markdown`] if a content input fails to convert,
/// or [`crate::error::Error::Compile`] if the composed document does not compile.
pub fn render_document_preview(
    state: &Document,
    theme_code: &str,
    component_sources: &BTreeMap<String, String>,
    component_schemas: &BTreeMap<String, ComponentSchema>,
) -> Result<Vec<u8>> {
    let source = compose_preview(state, theme_code, component_sources, component_schemas)?;
    crate::compiler::compile_to_pdf(&source)
}

/// The default value for an input of the given kind (no author-supplied value).
///
/// Mirrors the reference `GetDefaultForType`.
fn default_for_kind(kind: InputKind) -> Value {
    match kind {
        InputKind::String | InputKind::Content | InputKind::Image => Value::String(String::new()),
        InputKind::Number => Value::Number(0.into()),
        InputKind::Boolean => Value::Bool(false),
        InputKind::Color => Value::String("#000000".to_string()),
    }
}

/// Compose the live TEMPLATE + DOCUMENT Typst the embedded compiler renders.
///
/// Ports the live-Typst portion of the reference `FatFileService.Compose`: the
/// TEMPLATE section carries the (frontmatter-stripped) theme, the forward `doc`
/// state declaration, the `image-or-placeholder` helper, and every component
/// definition; the DOCUMENT section carries the `doc.update` of the global inputs
/// followed by the component-invocation tree.
fn compose_preview(
    state: &Document,
    theme_code: &str,
    component_sources: &BTreeMap<String, String>,
    component_schemas: &BTreeMap<String, ComponentSchema>,
) -> Result<String> {
    let mut out = String::new();

    // TEMPLATE section.
    out.push_str("// ===TEMPLATE-START===\n\n");
    out.push_str("// --- Theme ---\n\n");
    out.push_str(strip_frontmatter(theme_code).trim());
    out.push_str("\n\n// Forward-declare doc state so components can reference doc.get()\n");
    out.push_str("#let doc = state(\"doc\", (:))\n\n");
    out.push_str(IMAGE_HELPER);
    out.push('\n');
    for (component_id, source) in component_sources {
        out.push_str(&format!("\n// --- Component: {component_id} ---\n\n"));
        out.push_str(strip_frontmatter(source).trim());
        out.push('\n');
    }
    out.push_str("\n// ===TEMPLATE-END===\n\n");

    // DOCUMENT section.
    out.push_str("// ===DOCUMENT-START===\n\n");
    out.push_str(&generate_document_state(&state.inputs)?);
    out.push_str("\n// --- Document tree ---\n\n");
    out.push_str(&generate_typst_code(&state.nodes, component_schemas)?);
    out.push('\n');

    Ok(out)
}

/// Emit the `#doc.update((..))` block that seeds document-scope global inputs.
///
/// Mirrors the reference `DocumentGenerator.GenerateDocumentState`. Global inputs
/// are always formatted as scalars (never markdown content).
fn generate_document_state(global_inputs: &BTreeMap<String, InputValue>) -> Result<String> {
    let mut out = String::from("// Document context\n#doc.update((\n");
    for (name, input) in global_inputs {
        out.push_str(&format!(
            "  {name}: {},\n",
            format_value(&input.value, false)?
        ));
    }
    out.push_str("))\n");
    Ok(out)
}

/// Emit the component-invocation tree, one top-level node per page.
///
/// Mirrors the reference `DocumentGenerator.GenerateTypstCode`: top-level nodes
/// are separated by `#pagebreak()`.
fn generate_typst_code(
    nodes: &[Node],
    component_schemas: &BTreeMap<String, ComponentSchema>,
) -> Result<String> {
    let mut out = String::new();
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push_str("\n#pagebreak()\n\n");
        }
        generate_node(&mut out, node, component_schemas, 0)?;
    }
    Ok(out.trim_end().to_string())
}

/// Recursively emit one node as a `#component(inputs)[body]` invocation.
///
/// Mirrors the reference `DocumentGenerator.GenerateNode`: a body (`[]` or a
/// nested child block) is appended only when the component's schema declares
/// `has_body`.
fn generate_node(
    out: &mut String,
    node: &Node,
    component_schemas: &BTreeMap<String, ComponentSchema>,
    indent: usize,
) -> Result<()> {
    let prefix = " ".repeat(indent);
    let has_body = component_schemas
        .get(&node.component)
        .is_some_and(|s| s.has_body);

    out.push_str(&format!("{prefix}#{}(", node.component));
    append_inputs(out, &node.inputs)?;
    out.push(')');

    if has_body {
        if node.children.is_empty() {
            out.push_str("[]");
        } else {
            out.push_str("[\n");
            for (i, child) in node.children.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                generate_node(out, child, component_schemas, indent + 2)?;
            }
            out.push('\n');
            out.push_str(&format!("{prefix}]"));
        }
    }

    out.push('\n');
    Ok(())
}

/// Emit the named-argument list for a component invocation.
///
/// Content-typed inputs route through the Markdown converter and are wrapped in
/// `[..]`; scalar inputs are formatted as literals. Mirrors the reference
/// `DocumentGenerator.AppendInputs`.
fn append_inputs(out: &mut String, inputs: &BTreeMap<String, InputValue>) -> Result<()> {
    for (i, (name, input)) in inputs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let is_content = input.kind == InputKind::Content;
        out.push_str(&format!(
            "{name}: {}",
            format_value(&input.value, is_content)?
        ));
    }
    Ok(())
}

/// Format a JSON input value as a Typst literal.
///
/// Mirrors the reference `DocumentGenerator.FormatValue`: content strings convert
/// through Markdown and become `[..]` content blocks; other strings become quoted
/// Typst strings; numbers/bools become literals; null becomes `[]` (content) or
/// `none` (scalar).
fn format_value(value: &Value, is_content: bool) -> Result<String> {
    Ok(match value {
        Value::Null => {
            if is_content {
                "[]".to_string()
            } else {
                "none".to_string()
            }
        }
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) if is_content => format_content_value(s)?,
        Value::String(s) => format_string_value(s),
        // Arrays / objects have no first-class Typst literal here; fall back to a
        // quoted string of their JSON form, matching the reference's catch-all.
        other => format_string_value(&other.to_string()),
    })
}

/// Convert a Markdown content string to a Typst `[..]` content block.
fn format_content_value(s: &str) -> Result<String> {
    if s.is_empty() {
        return Ok("[]".to_string());
    }
    let typst = crate::markdown::markdown_to_typst(s)?;
    if typst.is_empty() {
        Ok("[]".to_string())
    } else {
        Ok(format!("[{typst}]"))
    }
}

/// Format a string as a quoted, backslash/quote-escaped Typst string literal.
fn format_string_value(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Strip a leading `/*--- … ---*/` frontmatter block and any embedded
/// `/*===IMAGES-START=== … ===IMAGES-END===*/` section from a Typst source.
///
/// Mirrors the reference `FatFileService.StripFrontmatter`: theme and component
/// sources carry a schema/frontmatter header that must not leak into the composed
/// preview.
fn strip_frontmatter(src: &str) -> String {
    let trimmed = src.trim_start();
    let mut body = trimmed.to_string();
    if let Some(rest) = trimmed.strip_prefix("/*---") {
        if let Some(end) = rest.find("---*/") {
            body = rest[end + "---*/".len()..].to_string();
        }
    }
    if let Some(start) = body.find("/*===IMAGES-START===") {
        if let Some(end_rel) = body[start..].find("===IMAGES-END===*/") {
            let end = start + end_rel + "===IMAGES-END===*/".len();
            body.replace_range(start..end, "");
        }
    }
    body.trim_start_matches(['\r', '\n']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::InputSchema;

    /// A leaf component (no body) that renders its scalar + content inputs.
    fn greeting_schema() -> ComponentSchema {
        ComponentSchema {
            name: "greeting".to_string(),
            has_body: false,
            allowed_children: Vec::new(),
            inputs: vec![
                InputSchema {
                    name: "name".to_string(),
                    kind: InputKind::String,
                    required: true,
                },
                InputSchema {
                    name: "body".to_string(),
                    kind: InputKind::Content,
                    required: true,
                },
            ],
        }
    }

    const GREETING_SOURCE: &str =
        "/*---\ncomponentId: greeting\n---*/\n#let greeting(name: \"\", body: []) = [Hello #name: #body]";

    #[test]
    fn render_component_preview_produces_pdf_bytes() {
        let pdf = render_component_preview(
            GREETING_SOURCE,
            &greeting_schema(),
            &[
                ("name".to_string(), Value::String("World".to_string())),
                (
                    "body".to_string(),
                    Value::String("Hello **bold**".to_string()),
                ),
            ],
            None,
        )
        .expect("component preview compiles");
        assert!(!pdf.is_empty(), "preview PDF must be non-empty");
        assert_eq!(&pdf[..5], b"%PDF-", "preview output must be a PDF");
    }

    #[test]
    fn render_component_preview_uses_schema_defaults_when_no_inputs() {
        // No overrides: every input falls back to its type default and must still
        // compose into a compilable document.
        let pdf = render_component_preview(GREETING_SOURCE, &greeting_schema(), &[], None)
            .expect("default-seeded preview compiles");
        assert_eq!(&pdf[..5], b"%PDF-");
    }

    #[test]
    fn render_document_preview_produces_pdf_bytes() {
        let mut node_inputs = BTreeMap::new();
        node_inputs.insert(
            "title".to_string(),
            InputValue {
                kind: InputKind::String,
                value: Value::String("Intro".to_string()),
            },
        );

        let mut global_inputs = BTreeMap::new();
        global_inputs.insert(
            "title".to_string(),
            InputValue {
                kind: InputKind::String,
                value: Value::String("My Document".to_string()),
            },
        );

        let state = Document {
            template: "default".to_string(),
            inputs: global_inputs,
            nodes: vec![Node {
                id: NodeId("section-aabb".to_string()),
                component: "section".to_string(),
                inputs: node_inputs,
                children: Vec::new(),
            }],
            images: Vec::new(),
        };

        let theme = "#let theme = state(\"theme\", (:))";
        let mut sources = BTreeMap::new();
        sources.insert(
            "section".to_string(),
            "#let section(title: \"\", body) = [== #title #body]".to_string(),
        );
        let mut schemas = BTreeMap::new();
        schemas.insert("section".to_string(), {
            let mut schema = ComponentSchema::new("section");
            schema.has_body = true;
            schema
        });

        let pdf = render_document_preview(&state, theme, &sources, &schemas)
            .expect("document preview compiles");
        assert!(!pdf.is_empty(), "document preview PDF must be non-empty");
        assert_eq!(&pdf[..5], b"%PDF-", "document preview output must be a PDF");
    }

    #[test]
    fn format_value_routes_content_and_scalars() {
        assert_eq!(format_value(&Value::Bool(true), false).unwrap(), "true");
        assert_eq!(format_value(&Value::Null, false).unwrap(), "none");
        assert_eq!(format_value(&Value::Null, true).unwrap(), "[]");
        assert_eq!(
            format_value(&Value::String("a\"b".to_string()), false).unwrap(),
            "\"a\\\"b\""
        );
        assert!(format_value(&Value::String("**x**".to_string()), true)
            .unwrap()
            .starts_with('['));
    }

    #[test]
    fn strip_frontmatter_removes_leading_block() {
        let stripped = strip_frontmatter("/*---\nid: x\n---*/\n#let a = 1");
        assert_eq!(stripped, "#let a = 1");
    }
}
