//! The canonical composed `.ndoc.typ` fat-file format.
//!
//! Ported from the C# reference (`FatFileService` / `ImageSectionParser` /
//! `StateBlockWriter`). A composed document is a single Typst-compilable file
//! built from four sections delimited by marker comments:
//!
//! ```text
//! /*===STATE-START===        <- Typst block comment: JSON prelude + input blocks
//! ...
//! ===STATE-END===*/
//! /*===IMAGES-START===       <- Typst block comment: base64 image blobs by hash
//! ...
//! ===IMAGES-END===*/
//! // ===TEMPLATE-START===    <- live Typst: theme + helpers + component defs
//! ...
//! // ===TEMPLATE-END===
//! // ===DOCUMENT-START===    <- live Typst: doc state + component tree
//! ...
//! ```
//!
//! Because STATE/IMAGES are block comments and TEMPLATE/DOCUMENT are live Typst,
//! the whole file compiles directly once the images referenced by
//! `image("images/{name}")` are materialised in the compiler's virtual
//! filesystem. We mirror the C# build pipeline: extract the base64 blobs (keyed
//! by content hash), map them to filenames via the STATE prelude's `images`
//! manifest, and inject each at `images/{name}` before compiling the source
//! verbatim.
//!
//! This is distinct from the four-section format ([`super`]) and the entry
//! format ([`super::ndoc`]); all three share the `.ndoc.typ` extension and are
//! told apart by their first non-empty line.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::{Error, Result};

const STATE_START: &str = "/*===STATE-START===";
const STATE_END: &str = "===STATE-END===*/";
const IMAGES_START: &str = "/*===IMAGES-START===";
const IMAGES_END: &str = "===IMAGES-END===*/";

/// The closing delimiter for one base64 blob in the IMAGES section.
const IMAGE_END_DELIM: &str = "---END---";

/// One entry in the STATE prelude `images` manifest: a logical filename and the
/// content hash that locates its bytes in the IMAGES section.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ImageRef {
    pub name: String,
    pub hash: String,
}

/// The leading JSON object of the STATE section.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Prelude {
    template_id: Option<String>,
    template_version: Option<String>,
    theme_id: Option<String>,
    #[serde(default)]
    nodes: Vec<PreludeNode>,
    #[serde(default)]
    images: Vec<ImageRef>,
}

/// One node in the prelude's `nodes` tree: a stable id, its component type, and
/// nested children.
#[derive(Deserialize)]
struct PreludeNode {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    children: Vec<PreludeNode>,
}

/// A parsed input value attached to a node or to the document.
///
/// Scalar inputs come from a block's YAML frontmatter; content inputs come from
/// its `<!-- start: name --> … <!-- end -->` blocks. The distinction is kept so
/// type checks can tell a YAML bool/number from authored markdown.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedInput {
    /// A scalar value from YAML frontmatter (string, number, bool, …).
    Scalar(serde_yaml_ng::Value),
    /// Markdown content from a `<!-- start: name --> … <!-- end -->` block.
    Content(String),
}

/// One node of the parsed document tree: id, component type, resolved inputs,
/// and children.
#[derive(Debug, Clone, PartialEq)]
pub struct DocNode {
    pub id: String,
    pub component_type: String,
    pub inputs: BTreeMap<String, ParsedInput>,
    pub children: Vec<DocNode>,
}

/// The fully parsed STATE section: template/theme identity, document-scope
/// global inputs, and the node tree with per-node inputs attached.
#[derive(Debug, Clone, PartialEq)]
pub struct DocState {
    pub template_id: String,
    pub template_version: String,
    pub theme_id: String,
    pub global_inputs: BTreeMap<String, ParsedInput>,
    pub nodes: Vec<DocNode>,
}

/// Whether `src` is a canonical composed fat file.
///
/// A composed document's first non-empty line is the `/*===STATE-START===`
/// marker. This distinguishes it from the entry format (`// ndoc document v1`)
/// and the four-section format (`// === STATE ===`), which also use `.ndoc.typ`.
pub fn is_composed(src: &str) -> bool {
    src.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line == STATE_START)
}

/// Extract the IMAGES section into a `hash -> bytes` map.
///
/// Mirrors the C# `ImageSectionParser.ExtractImages`: within the
/// `/*===IMAGES-START===` … `===IMAGES-END===*/` block each blob is framed by a
/// `---{hash}---` delimiter line, base64 payload lines, and a closing
/// `---END---` line. A missing section yields an empty map.
///
/// # Errors
///
/// Returns [`Error::FatFile`] on a duplicate hash entry or undecodable base64.
pub fn extract_image_blobs(src: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    use base64::Engine as _;

    let Some(body) = section_body(src, IMAGES_START, IMAGES_END) else {
        return Ok(BTreeMap::new());
    };

    let mut map = BTreeMap::new();
    let mut current_hash: Option<String> = None;
    let mut payload = String::new();

    for raw in body.lines() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with("---")
            && line.ends_with("---")
            && line != IMAGE_END_DELIM
            && line.len() > 6
        {
            let hash = &line[3..line.len() - 3];
            if map.contains_key(hash) {
                return Err(Error::FatFile(format!(
                    "duplicate hash entry in IMAGES section: {hash}"
                )));
            }
            current_hash = Some(hash.to_string());
            payload.clear();
        } else if line == IMAGE_END_DELIM {
            if let Some(hash) = current_hash.take() {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(payload.trim())
                    .map_err(|e| Error::FatFile(format!("invalid base64 for image {hash}: {e}")))?;
                map.insert(hash, bytes);
                payload.clear();
            }
        } else if current_hash.is_some() && !line.trim().is_empty() {
            payload.push_str(line.trim());
        }
    }

    Ok(map)
}

/// Parse the STATE prelude's `images` manifest (logical name -> content hash).
///
/// # Errors
///
/// Returns [`Error::FatFile`] if the STATE markers are absent or the leading
/// JSON prelude fails to deserialise.
pub fn image_manifest(src: &str) -> Result<Vec<ImageRef>> {
    Ok(parse_prelude(src)?.images)
}

/// Parse the STATE section's leading JSON prelude.
///
/// The prelude is the leading JSON object; `<document-input>` / per-node
/// `<component-input>` blocks follow it. A streaming deserialiser reads exactly
/// the first JSON value and ignores the trailing block text.
fn parse_prelude(src: &str) -> Result<Prelude> {
    let body = section_body(src, STATE_START, STATE_END)
        .ok_or_else(|| Error::FatFile("missing STATE section markers".into()))?;

    serde_json::Deserializer::from_str(body)
        .into_iter::<Prelude>()
        .next()
        .ok_or_else(|| Error::FatFile("STATE section has no JSON prelude".into()))?
        .map_err(|e| Error::FatFile(format!("invalid STATE prelude JSON: {e}")))
}

/// Parse the full STATE section into a [`DocState`]: the node tree from the JSON
/// prelude, global inputs from the `<document-input>` block, and per-node inputs
/// from each `<component-input>` block (joined to nodes by id).
///
/// Ported from the C# `FatFileService.ParseStateBody` / `StateBlockChunker` /
/// `TaggedBlockParser`.
///
/// # Errors
///
/// Returns [`Error::FatFile`] if the STATE markers are missing, the prelude JSON
/// is invalid, a tagged block is malformed, or input YAML fails to parse.
pub fn parse_state(src: &str) -> Result<DocState> {
    let prelude = parse_prelude(src)?;
    let body = section_body(src, STATE_START, STATE_END)
        .ok_or_else(|| Error::FatFile("missing STATE section markers".into()))?;

    // Global (document-scope) inputs from the single <document-input> block. The
    // `templateId` anchor key is an internal marker, not a global input.
    let mut global_inputs = BTreeMap::new();
    if let Some(doc_input) = first_tag_body(body, "document-input") {
        let (yaml, _content) = parse_tagged_block(&doc_input)?;
        global_inputs = parse_flat_yaml(&yaml)?;
        global_inputs.remove("templateId");
    }

    // Per-node inputs from each <component-input> block, keyed by joined id.
    let mut inputs_by_id: BTreeMap<String, BTreeMap<String, ParsedInput>> = BTreeMap::new();
    for chunk in component_input_chunks(body)? {
        let (yaml, content_blocks) = parse_tagged_block(&chunk.body)?;
        let mut inputs = parse_flat_yaml(&yaml)?;
        for (name, markdown) in content_blocks {
            inputs.insert(name, ParsedInput::Content(markdown));
        }
        inputs_by_id.insert(chunk.joined_id(), inputs);
    }

    let nodes = build_nodes(&prelude.nodes, &inputs_by_id);

    Ok(DocState {
        template_id: prelude.template_id.unwrap_or_default(),
        template_version: prelude.template_version.unwrap_or_default(),
        theme_id: prelude.theme_id.unwrap_or_default(),
        global_inputs,
        nodes,
    })
}

/// Recursively materialise the prelude node tree into [`DocNode`]s, attaching
/// each node's parsed inputs (empty when no `<component-input>` block matched).
fn build_nodes(
    prelude_nodes: &[PreludeNode],
    inputs_by_id: &BTreeMap<String, BTreeMap<String, ParsedInput>>,
) -> Vec<DocNode> {
    prelude_nodes
        .iter()
        .map(|n| DocNode {
            id: n.id.clone(),
            component_type: n.node_type.clone(),
            inputs: inputs_by_id.get(&n.id).cloned().unwrap_or_default(),
            children: build_nodes(&n.children, inputs_by_id),
        })
        .collect()
}

/// Deserialise a flat YAML map into scalar inputs. An empty body yields an empty
/// map.
fn parse_flat_yaml(yaml: &str) -> Result<BTreeMap<String, ParsedInput>> {
    if yaml.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    let map: BTreeMap<String, serde_yaml_ng::Value> = serde_yaml_ng::from_str(yaml)
        .map_err(|e| Error::FatFile(format!("invalid input YAML: {e}")))?;
    Ok(map
        .into_iter()
        .map(|(k, v)| (k, ParsedInput::Scalar(v)))
        .collect())
}

/// One `<component-input componentId="…" instance="…">` block.
struct ComponentChunk {
    component_id: String,
    instance: String,
    body: String,
}

impl ComponentChunk {
    /// The `{componentId}-{instance}` form used as the node id.
    fn joined_id(&self) -> String {
        format!("{}-{}", self.component_id, self.instance)
    }
}

/// Extract every `<component-input …>…</component-input>` block in order.
fn component_input_chunks(body: &str) -> Result<Vec<ComponentChunk>> {
    const OPEN: &str = "<component-input";
    const CLOSE: &str = "</component-input>";

    let mut chunks = Vec::new();
    let mut cursor = 0;
    while let Some(rel) = body[cursor..].find(OPEN) {
        let start = cursor + rel + OPEN.len();
        let gt = body[start..]
            .find('>')
            .ok_or_else(|| Error::FatFile("unclosed <component-input> opening tag".into()))?;
        let attrs = &body[start..start + gt];
        let inner_start = start + gt + 1;
        let close = body[inner_start..]
            .find(CLOSE)
            .ok_or_else(|| Error::FatFile("unclosed <component-input> block".into()))?;
        let inner = &body[inner_start..inner_start + close];

        let component_id = attr_value(attrs, "componentId").ok_or_else(|| {
            Error::FatFile("<component-input> is missing its componentId attribute".into())
        })?;
        let instance = attr_value(attrs, "instance").ok_or_else(|| {
            Error::FatFile("<component-input> is missing its instance attribute".into())
        })?;

        chunks.push(ComponentChunk {
            component_id,
            instance,
            body: inner.to_string(),
        });
        cursor = inner_start + close + CLOSE.len();
    }
    Ok(chunks)
}

/// Read the body of the first `<{tag}…>…</{tag}>` block, or `None` if absent.
fn first_tag_body(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let gt = body[start..].find('>')? + start + 1;
    let close_rel = body[gt..].find(&close)?;
    Some(body[gt..gt + close_rel].to_string())
}

/// Read the quoted value of `key="value"` from a tag's attribute string.
fn attr_value(attrs: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = attrs.find(&needle)? + needle.len();
    let end = attrs[start..].find('"')? + start;
    Some(attrs[start..end].to_string())
}

/// Split a tagged block into its leading `---`-fenced YAML frontmatter and any
/// `<!-- start: name --> … <!-- end -->` content blocks beneath it.
///
/// Ported from the C# `TaggedBlockParser`.
fn parse_tagged_block(block: &str) -> Result<(String, Vec<(String, String)>)> {
    let norm = block.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = norm.trim_start_matches([' ', '\t', '\n']);

    if !trimmed.starts_with("---") {
        return Err(Error::FatFile(
            "tagged block is missing its opening '---' YAML frontmatter fence".into(),
        ));
    }
    let after_open = trimmed.find('\n').ok_or_else(|| {
        Error::FatFile("tagged block's opening '---' fence is not followed by a newline".into())
    })? + 1;

    let close = find_closing_fence(&trimmed[after_open..]).ok_or_else(|| {
        Error::FatFile("tagged block is missing its closing '---' YAML frontmatter fence".into())
    })?;
    let yaml = trimmed[after_open..after_open + close]
        .trim_end_matches('\n')
        .to_string();

    // Step past the closing fence line to the start of the trailing body.
    let rest = &trimmed[after_open + close..];
    let body_start = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
    let content = parse_content_blocks(&rest[body_start..])?;

    Ok((yaml, content))
}

/// Offset (within `s`) of a line that trims to exactly `---`, or `None`.
fn find_closing_fence(s: &str) -> Option<usize> {
    let mut cursor = 0;
    loop {
        let line_end = s[cursor..]
            .find('\n')
            .map(|i| cursor + i)
            .unwrap_or(s.len());
        if s[cursor..line_end].trim() == "---" {
            return Some(cursor);
        }
        if line_end >= s.len() {
            return None;
        }
        cursor = line_end + 1;
    }
}

/// Parse `<!-- start: name --> … <!-- end -->` content blocks from a body.
fn parse_content_blocks(body: &str) -> Result<Vec<(String, String)>> {
    const START: &str = "<!-- start:";
    const END: &str = "<!-- end -->";

    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(rel) = body[cursor..].find(START) {
        let after = cursor + rel + START.len();
        let name_end = body[after..]
            .find("-->")
            .ok_or_else(|| Error::FatFile("unterminated content start marker".into()))?;
        let name = body[after..after + name_end].trim().to_string();

        let content_start = after + name_end + "-->".len();
        let end_rel = body[content_start..].find(END).ok_or_else(|| {
            Error::FatFile(format!("content block '{name}' has no <!-- end --> marker"))
        })?;
        let raw = &body[content_start..content_start + end_rel];
        blocks.push((name, trim_block_boundaries(raw)));
        cursor = content_start + end_rel + END.len();
    }
    Ok(blocks)
}

/// Strip the single newline adjacent to each content-block delimiter.
fn trim_block_boundaries(raw: &str) -> String {
    let start = raw.strip_prefix('\n').unwrap_or(raw);
    start.strip_suffix('\n').unwrap_or(start).to_string()
}

/// Resolve `image("images/{name}")` references into `(name, bytes)` pairs by
/// joining the STATE manifest to the IMAGES blobs via content hash.
///
/// Manifest entries whose hash is absent from the IMAGES section are skipped,
/// matching the C# pipeline (which omits blobs it cannot find rather than
/// failing the build).
pub fn resolve_images(src: &str) -> Result<Vec<(String, Vec<u8>)>> {
    let blobs = extract_image_blobs(src)?;
    let manifest = image_manifest(src)?;
    Ok(manifest
        .into_iter()
        .filter_map(|img| blobs.get(&img.hash).map(|bytes| (img.name, bytes.clone())))
        .collect())
}

/// Render a composed fat file to PDF bytes.
///
/// Materialises the manifest images into the compiler's virtual filesystem at
/// `images/{name}`, then compiles the source verbatim — the STATE and IMAGES
/// sections are inert block comments, so the live TEMPLATE/DOCUMENT sections
/// drive the render.
pub fn render_to_pdf(src: &str) -> Result<Vec<u8>> {
    let images = resolve_images(src)?;
    crate::compiler::compile_to_pdf_with_images(src, &images)
}

/// Return the text strictly between `start` and the first `end` after it, or
/// `None` if either marker is missing.
fn section_body<'a>(src: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_idx = src.find(start)? + start.len();
    let rel_end = src[start_idx..].find(end)?;
    Some(&src[start_idx..start_idx + rel_end])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny but valid SVG so the embedded compiler can rasterise it.
    const TINY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>"##;

    /// Build a minimal but complete composed document referencing one image.
    fn composed_doc() -> String {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(TINY_SVG.as_bytes());
        format!(
            "/*===STATE-START===\n\
             {{\n  \"templateId\": \"t\",\n  \"themeId\": \"th\",\n  \"nodes\": [],\n  \"images\": [\n    {{ \"name\": \"logo.svg\", \"hash\": \"deadbeef\" }}\n  ]\n}}\n\n\
             <document-input>\n---\ntemplateId: t\n---\n</document-input>\n\
             ===STATE-END===*/\n\
             /*===IMAGES-START===\n---deadbeef---\n{b64}\n---END---\n===IMAGES-END===*/\n\
             // ===TEMPLATE-START===\n// ===TEMPLATE-END===\n\
             // ===DOCUMENT-START===\n#image(\"images/logo.svg\", width: 10pt)\n"
        )
    }

    #[test]
    fn is_composed_detects_state_start_marker() {
        assert!(is_composed(&composed_doc()));
    }

    #[test]
    fn is_composed_rejects_entry_and_four_section_formats() {
        assert!(!is_composed("// ndoc document v1\n"));
        assert!(!is_composed("// === STATE ===\nbody\n"));
        assert!(!is_composed(""));
    }

    #[test]
    fn extract_image_blobs_decodes_hash_keyed_base64() {
        let blobs = extract_image_blobs(&composed_doc()).expect("blobs parse");
        assert_eq!(blobs.len(), 1);
        assert_eq!(
            blobs.get("deadbeef").map(Vec::as_slice),
            Some(TINY_SVG.as_bytes())
        );
    }

    #[test]
    fn extract_image_blobs_empty_when_no_section() {
        assert!(extract_image_blobs("// ===DOCUMENT-START===\n#[]")
            .expect("no images section is not an error")
            .is_empty());
    }

    #[test]
    fn extract_image_blobs_decodes_payload_split_across_multiple_lines() {
        use base64::Engine as _;
        // 200 bytes encodes to a base64 payload longer than one line; split it so
        // the decoder must reassemble multiple payload lines before decoding.
        let original: Vec<u8> = (0..200u32).map(|i| (i % 251) as u8).collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&original);
        let wrapped: String = b64
            .as_bytes()
            .chunks(76)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        let src = format!(
            "/*===IMAGES-START===\n---largehash---\n{wrapped}\n---END---\n===IMAGES-END===*/\n"
        );
        let blobs = extract_image_blobs(&src).expect("multi-line blob decodes");
        assert_eq!(
            blobs.get("largehash").map(Vec::as_slice),
            Some(original.as_slice())
        );
    }

    #[test]
    fn extract_image_blobs_rejects_duplicate_hash() {
        let src = "/*===IMAGES-START===\n\
                   ---aaaa---\nAA==\n---END---\n\
                   ---aaaa---\nAQ==\n---END---\n\
                   ===IMAGES-END===*/\n";
        let err = extract_image_blobs(src).expect_err("duplicate hash must error");
        assert!(
            err.to_string().contains("duplicate hash"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn image_manifest_reads_name_and_hash() {
        let manifest = image_manifest(&composed_doc()).expect("manifest parse");
        assert_eq!(
            manifest,
            vec![ImageRef {
                name: "logo.svg".to_string(),
                hash: "deadbeef".to_string(),
            }]
        );
    }

    #[test]
    fn image_manifest_errors_without_state_section() {
        let err = image_manifest("no markers here").expect_err("missing STATE must error");
        assert!(err.to_string().contains("STATE"), "unexpected error: {err}");
    }

    #[test]
    fn resolve_images_joins_manifest_to_blobs_by_hash() {
        let images = resolve_images(&composed_doc()).expect("resolve");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].0, "logo.svg");
        assert_eq!(images[0].1.as_slice(), TINY_SVG.as_bytes());
    }

    /// A composed STATE section with a document-input block, a parent node with
    /// a scalar input and a content input, and one child node.
    fn composed_with_state() -> String {
        "/*===STATE-START===\n\
         {\n  \"templateId\": \"default\",\n  \"templateVersion\": \"1.0\",\n  \"themeId\": \"th\",\n  \"nodes\": [\n    { \"id\": \"heading-aaaa\", \"type\": \"heading\", \"children\": [\n      { \"id\": \"paragraph-bbbb\", \"type\": \"paragraph\" }\n    ] }\n  ]\n}\n\n\
         <document-input>\n---\ntemplateId: default\ntitle: \"My Doc\"\n---\n</document-input>\n\n\
         <component-input componentId=\"heading\" instance=\"aaaa\">\n---\nlevel: 1\n---\n\n<!-- start: text -->\nHello **world**\n<!-- end -->\n</component-input>\n\n\
         <component-input componentId=\"paragraph\" instance=\"bbbb\">\n---\ntext: \"Body\"\n---\n</component-input>\n\
         ===STATE-END===*/\n\
         // ===DOCUMENT-START===\n= Doc\n"
            .to_string()
    }

    #[test]
    fn parse_state_reads_identity_and_global_inputs() {
        let state = parse_state(&composed_with_state()).expect("state parses");
        assert_eq!(state.template_id, "default");
        assert_eq!(state.template_version, "1.0");
        assert_eq!(state.theme_id, "th");
        // templateId anchor is excluded; only `title` is a global input.
        assert_eq!(state.global_inputs.len(), 1);
        assert_eq!(
            state.global_inputs.get("title"),
            Some(&ParsedInput::Scalar(serde_yaml_ng::Value::String(
                "My Doc".to_string()
            )))
        );
    }

    #[test]
    fn parse_state_builds_node_tree_with_inputs() {
        let state = parse_state(&composed_with_state()).expect("state parses");
        assert_eq!(state.nodes.len(), 1);
        let heading = &state.nodes[0];
        assert_eq!(heading.id, "heading-aaaa");
        assert_eq!(heading.component_type, "heading");
        // scalar `level` plus content `text`.
        assert_eq!(
            heading.inputs.get("level"),
            Some(&ParsedInput::Scalar(serde_yaml_ng::Value::Number(1.into())))
        );
        assert_eq!(
            heading.inputs.get("text"),
            Some(&ParsedInput::Content("Hello **world**".to_string()))
        );
        // child node attached, with its own scalar input.
        assert_eq!(heading.children.len(), 1);
        let para = &heading.children[0];
        assert_eq!(para.component_type, "paragraph");
        assert_eq!(
            para.inputs.get("text"),
            Some(&ParsedInput::Scalar(serde_yaml_ng::Value::String(
                "Body".to_string()
            )))
        );
    }

    #[test]
    fn parse_state_errors_on_malformed_prelude() {
        let src = "/*===STATE-START===\n{ broken json\n===STATE-END===*/\n";
        let err = parse_state(src).expect_err("malformed prelude must error");
        assert!(
            err.to_string().contains("prelude"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_content_blocks_extracts_named_blocks_in_order() {
        let body = "<!-- start: intro -->\nfirst\n<!-- end -->\n<!-- start: body -->\nsecond\n<!-- end -->\n";
        let blocks = parse_content_blocks(body).expect("blocks parse");
        assert_eq!(
            blocks,
            vec![
                ("intro".to_string(), "first".to_string()),
                ("body".to_string(), "second".to_string()),
            ]
        );
    }

    #[test]
    fn parse_content_blocks_empty_body_yields_no_blocks() {
        let blocks = parse_content_blocks("just some markdown, no markers\n").expect("parse");
        assert!(blocks.is_empty());
    }

    #[test]
    fn parse_tagged_block_normalises_crlf_line_endings() {
        let block = "---\r\nlevel: 1\r\n---\r\n<!-- start: text -->\r\nHello\r\n<!-- end -->\r\n";
        let (yaml, content) = parse_tagged_block(block).expect("tagged block parses");
        assert_eq!(yaml, "level: 1");
        assert_eq!(content, vec![("text".to_string(), "Hello".to_string())]);
    }

    #[test]
    fn attr_value_reads_key_regardless_of_attribute_order() {
        assert_eq!(
            attr_value("componentId=\"heading\" instance=\"aaaa\"", "instance").as_deref(),
            Some("aaaa")
        );
        assert_eq!(
            attr_value("instance=\"aaaa\" componentId=\"heading\"", "instance").as_deref(),
            Some("aaaa")
        );
    }

    #[test]
    fn parse_state_bare_component_without_input_block_has_empty_inputs() {
        let src = "/*===STATE-START===\n\
             {\n  \"templateId\": \"default\",\n  \"themeId\": \"th\",\n  \"nodes\": [\n    { \"id\": \"table-of-contents-1234\", \"type\": \"table-of-contents\" }\n  ]\n}\n\n\
             <document-input>\n---\ntemplateId: default\n---\n</document-input>\n\
             ===STATE-END===*/\n\
             // ===DOCUMENT-START===\n= Doc\n";
        let state = parse_state(src).expect("state parses");
        assert_eq!(state.nodes.len(), 1);
        let toc = &state.nodes[0];
        assert_eq!(toc.component_type, "table-of-contents");
        assert!(
            toc.inputs.is_empty(),
            "a bare component with no <component-input> block round-trips with empty inputs"
        );
    }

    #[test]
    fn render_to_pdf_resolves_embedded_image() {
        let pdf = render_to_pdf(&composed_doc()).expect("composed doc compiles to PDF");
        assert!(!pdf.is_empty(), "PDF bytes should be non-empty");
        assert_eq!(&pdf[..5], b"%PDF-", "output should be a PDF");
    }
}
