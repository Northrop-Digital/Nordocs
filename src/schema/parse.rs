//! File-backed schema parsing.
//!
//! Turns the YAML frontmatter of `.ncmp.typ` (component) and `.ndoct.typ`
//! (template) files into the [`ComponentSchema`](super::ComponentSchema) and
//! [`TemplateSchema`](super::TemplateSchema) types. The frontmatter is a YAML
//! block fenced inside a Typst block comment:
//!
//! ```typst
//! /*---
//! componentId: section-title
//! inputs:
//!   - name: title
//!     type: string
//! ---*/
//! ```
//!
//! The parser is isolated here so the on-disk file format can evolve without
//! touching the commands that consume the schema types. Every parse failure
//! surfaces as a typed [`Error::Schema`] rather than a panic.

use std::path::Path;

use serde::Deserialize;

use super::{ComponentSchema, InputSchema, TemplateSchema};
use crate::error::{Error, Result};
use crate::model::InputKind;

/// Opening fence of the frontmatter block comment.
const FENCE_OPEN: &str = "/*---";
/// Closing fence of the frontmatter block comment.
const FENCE_CLOSE: &str = "---*/";

/// File extension (suffix) identifying component files.
const COMPONENT_SUFFIX: &str = ".ncmp.typ";
/// File extension (suffix) identifying template files.
const TEMPLATE_SUFFIX: &str = ".ndoct.typ";

/// One declared input as authored in the frontmatter YAML.
///
/// `kind` reads the `type:` key; `required` defaults to `true` when omitted,
/// matching the authoring convention that inputs are mandatory unless a default
/// or `required: false` is supplied. Unknown YAML keys are ignored so the file
/// format may carry presentation metadata (labels, descriptions, defaults)
/// without breaking the parser.
#[derive(Debug, Deserialize)]
struct InputFile {
    name: String,
    #[serde(rename = "type")]
    kind: InputKind,
    #[serde(default = "default_required")]
    required: bool,
}

/// One declared content-typed input (the `content:` list in the frontmatter).
///
/// Content inputs carry no `type:` key — they are markdown content by
/// definition — so they parse separately from scalar [`InputFile`] entries and
/// are folded in as [`InputKind::Content`].
#[derive(Debug, Deserialize)]
struct ContentInputFile {
    name: String,
    #[serde(default = "default_required")]
    required: bool,
}

/// The frontmatter shape of a component file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentFile {
    component_id: String,
    #[serde(default)]
    inputs: Vec<InputFile>,
    #[serde(default)]
    content: Vec<ContentInputFile>,
    #[serde(default = "default_has_body")]
    has_body: bool,
    #[serde(default)]
    allowed_children: Vec<String>,
}

/// The frontmatter shape of a template file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateFile {
    template_id: String,
    #[serde(default)]
    document_inputs: Vec<InputFile>,
    #[serde(default)]
    allowed_components: Vec<String>,
}

fn default_required() -> bool {
    true
}

fn default_has_body() -> bool {
    true
}

impl From<InputFile> for InputSchema {
    fn from(input: InputFile) -> Self {
        InputSchema {
            name: input.name,
            kind: input.kind,
            required: input.required,
        }
    }
}

impl From<ContentInputFile> for InputSchema {
    fn from(input: ContentInputFile) -> Self {
        InputSchema {
            name: input.name,
            kind: InputKind::Content,
            required: input.required,
        }
    }
}

/// Parse a component schema from raw `.ncmp.typ` source text.
pub fn parse_component_str(content: &str) -> Result<ComponentSchema> {
    let yaml = extract_frontmatter(content).ok_or_else(|| {
        Error::Schema("component file has no '/*--- ... ---*/' frontmatter".into())
    })?;
    let file: ComponentFile = deserialize(yaml, "component")?;
    let inputs = file
        .inputs
        .into_iter()
        .map(InputSchema::from)
        .chain(file.content.into_iter().map(InputSchema::from))
        .collect();
    Ok(ComponentSchema {
        name: file.component_id,
        inputs,
        has_body: file.has_body,
        allowed_children: file.allowed_children,
    })
}

/// Parse a template schema from raw `.ndoct.typ` source text.
pub fn parse_template_str(content: &str) -> Result<TemplateSchema> {
    let yaml = extract_frontmatter(content).ok_or_else(|| {
        Error::Schema("template file has no '/*--- ... ---*/' frontmatter".into())
    })?;
    let file: TemplateFile = deserialize(yaml, "template")?;
    Ok(TemplateSchema {
        name: file.template_id,
        document_inputs: file
            .document_inputs
            .into_iter()
            .map(InputSchema::from)
            .collect(),
        allowed_components: file.allowed_components,
    })
}

/// Parse a component schema from a file on disk.
pub fn parse_component_file(path: &Path) -> Result<ComponentSchema> {
    let content = read_with_context(path)?;
    parse_component_str(&content).map_err(|e| Error::Schema(format!("{}: {e}", path.display())))
}

/// Parse a template schema from a file on disk.
pub fn parse_template_file(path: &Path) -> Result<TemplateSchema> {
    let content = read_with_context(path)?;
    parse_template_str(&content).map_err(|e| Error::Schema(format!("{}: {e}", path.display())))
}

/// Load every component (`*.ncmp.typ`) in a directory, in stable path order.
///
/// Errors if the directory cannot be read or any component file fails to parse.
pub fn load_components_from_dir(dir: &Path) -> Result<Vec<ComponentSchema>> {
    let mut out = Vec::new();
    for path in sorted_files_with_suffix(dir, COMPONENT_SUFFIX)? {
        out.push(parse_component_file(&path)?);
    }
    Ok(out)
}

/// Load every template (`*.ndoct.typ`) in a directory, in stable path order.
///
/// Errors if the directory cannot be read or any template file fails to parse.
pub fn load_templates_from_dir(dir: &Path) -> Result<Vec<TemplateSchema>> {
    let mut out = Vec::new();
    for path in sorted_files_with_suffix(dir, TEMPLATE_SUFFIX)? {
        out.push(parse_template_file(&path)?);
    }
    Ok(out)
}

/// Extract the YAML text between the `/*---` and `---*/` fences.
///
/// Returns `None` when either fence is absent. The fences and the blank lines
/// immediately adjacent to them are trimmed; interior YAML is preserved.
fn extract_frontmatter(content: &str) -> Option<&str> {
    let after_open = content.find(FENCE_OPEN)? + FENCE_OPEN.len();
    let rest = &content[after_open..];
    let close = rest.find(FENCE_CLOSE)?;
    Some(rest[..close].trim_matches(['\n', '\r']))
}

/// Deserialize YAML into `T`, mapping any parse failure to [`Error::Schema`].
fn deserialize<T: for<'de> Deserialize<'de>>(yaml: &str, kind: &str) -> Result<T> {
    serde_yaml_ng::from_str(yaml)
        .map_err(|e| Error::Schema(format!("failed to parse {kind} frontmatter: {e}")))
}

/// Read a file, wrapping I/O failures with the offending path.
fn read_with_context(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::Schema(format!("cannot read '{}': {e}", path.display())))
}

/// Collect files in `dir` whose name ends with `suffix`, sorted by path.
///
/// Sorting by the full path yields a deterministic, OS-independent enumeration
/// order so listings and snapshots are stable across machines.
fn sorted_files_with_suffix(dir: &Path, suffix: &str) -> Result<Vec<std::path::PathBuf>> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Error::Schema(format!("cannot read directory '{}': {e}", dir.display())))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| Error::Schema(format!("cannot read directory entry: {e}")))?;
        let path = entry.path();
        let is_match = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(suffix));
        if is_match {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("create fixture file");
        f.write_all(content.as_bytes()).expect("write fixture file");
        path
    }

    const COMPONENT_SRC: &str = "/*---\n\
        componentId: section-title\n\
        displayName: Section Title\n\
        description: A divider page.\n\
        inputs:\n\
        \x20 - name: title\n\
        \x20   type: string\n\
        \x20   label: Title\n\
        \x20 - name: logo\n\
        \x20   type: image\n\
        \x20   required: false\n\
        ---*/\n\
        #let section-title(title: \"\") = { }\n";

    const TEMPLATE_SRC: &str = "/*---\n\
        templateId: fee-proposal\n\
        displayName: Fee Proposal\n\
        allowedComponents:\n\
        \x20 - cover-page\n\
        \x20 - section-title\n\
        documentInputs:\n\
        \x20 - name: title\n\
        \x20   type: string\n\
        \x20 - name: date\n\
        \x20   type: string\n\
        \x20   required: false\n\
        ---*/\n";

    #[test]
    fn parse_component_reads_id_and_inputs() {
        let schema = parse_component_str(COMPONENT_SRC).expect("parse component");
        assert_eq!(schema.name, "section-title");
        assert_eq!(schema.inputs.len(), 2);
        assert_eq!(schema.inputs[0].name, "title");
        assert_eq!(schema.inputs[0].kind, InputKind::String);
        assert!(schema.inputs[0].required, "required defaults to true");
        assert_eq!(schema.inputs[1].kind, InputKind::Image);
        assert!(
            !schema.inputs[1].required,
            "explicit required: false honoured"
        );
    }

    #[test]
    fn parse_template_reads_inputs_and_allowed_components() {
        let schema = parse_template_str(TEMPLATE_SRC).expect("parse template");
        assert_eq!(schema.name, "fee-proposal");
        assert_eq!(
            schema.allowed_components,
            vec!["cover-page".to_string(), "section-title".to_string()]
        );
        assert_eq!(schema.document_inputs.len(), 2);
        assert!(schema.document_inputs[0].required);
        assert!(!schema.document_inputs[1].required);
    }

    #[test]
    fn missing_frontmatter_is_typed_error() {
        let err = parse_component_str("#let x = 1\n").expect_err("must reject without frontmatter");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn malformed_yaml_is_typed_error_not_panic() {
        let bad = "/*---\ncomponentId: x\ninputs: [oops\n---*/\n";
        let err = parse_component_str(bad).expect_err("malformed YAML must error");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn unknown_input_kind_is_typed_error() {
        let bad = "/*---\ncomponentId: x\ninputs:\n  - name: a\n    type: widget\n---*/\n";
        let err = parse_component_str(bad).expect_err("unknown kind must error");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn parse_component_file_names_path_on_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_file(dir.path(), "broken.ncmp.typ", "no frontmatter");
        let err = parse_component_file(&path).expect_err("must fail to parse");
        let Error::Schema(msg) = err else {
            panic!("expected schema error");
        };
        assert!(
            msg.contains("broken.ncmp.typ"),
            "error names the path: {msg}"
        );
    }

    #[test]
    fn load_components_from_dir_is_stable_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        write_file(
            &root,
            "zeta.ncmp.typ",
            &COMPONENT_SRC.replace("section-title", "zeta"),
        );
        write_file(
            &root,
            "alpha.ncmp.typ",
            &COMPONENT_SRC.replace("section-title", "alpha"),
        );
        write_file(&root, "ignored.txt", "not a component");

        let schemas = load_components_from_dir(&root).expect("load components");
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["alpha", "zeta"],
            "sorted by path, non-matches skipped"
        );
    }

    #[test]
    fn load_templates_from_dir_is_stable_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        write_file(
            &root,
            "b.ndoct.typ",
            &TEMPLATE_SRC.replace("fee-proposal", "b-tmpl"),
        );
        write_file(
            &root,
            "a.ndoct.typ",
            &TEMPLATE_SRC.replace("fee-proposal", "a-tmpl"),
        );

        let schemas = load_templates_from_dir(&root).expect("load templates");
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["a-tmpl", "b-tmpl"]);
    }

    #[test]
    fn load_components_missing_dir_is_typed_error() {
        let err = load_components_from_dir(Path::new("/no/such/dir/at/all"))
            .expect_err("missing dir must error");
        assert!(matches!(err, Error::Schema(_)));
    }

    #[test]
    fn empty_dir_yields_zero_components() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schemas = load_components_from_dir(dir.path()).expect("read empty dir");
        assert!(schemas.is_empty());
    }
}
