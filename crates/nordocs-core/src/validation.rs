//! Schema-based validation for `.ndoc.typ` and `.md` documents.
//!
//! The two public entry points validate against the built-in catalogue and
//! return a [`ValidationResult`] listing every violation found. Neither
//! function fails fast on the first error — all violations are collected
//! before returning.

use std::path::Path;

use serde::Serialize;

use crate::error::Result;
use crate::fatfile::ndoc::NdocDocument;
use crate::schema::{Catalogue, ConstraintKind};

/// The severity of a [`Violation`]. Only `Error`-severity entries make a
/// document invalid; `Warning`s are reported but do not fail validation.
/// Mirrors the C# `ValidationIssue.Severity` (`"error"` / `"warning"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// A single issue found during validation.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// Whether this issue invalidates the document or is advisory.
    pub severity: Severity,
    /// Stable machine-readable code, e.g. `unknown-input`, `input-type-mismatch`.
    pub code: String,
    /// Identifies the location of the problem (file path, node id, input name).
    pub location: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl Violation {
    /// Construct an `Error`-severity violation.
    pub fn error(
        code: impl Into<String>,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            location: location.into(),
            message: message.into(),
        }
    }

    /// Construct a `Warning`-severity violation.
    pub fn warning(
        code: impl Into<String>,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            location: location.into(),
            message: message.into(),
        }
    }
}

/// A summary of the document validated, mirroring the C# `ValidationSummary`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationSummary {
    pub template_id: String,
    pub template_version: String,
    pub theme_id: String,
    pub node_count: usize,
    pub global_input_count: usize,
}

/// The output of a validation run.
///
/// `summary` is populated for composed documents (which carry template/theme
/// identity); it is `None` for entry-format and markdown inputs.
#[derive(Debug, Default, Clone)]
pub struct ValidationResult {
    pub violations: Vec<Violation>,
    pub summary: Option<ValidationSummary>,
}

impl ValidationResult {
    /// `true` when no `Error`-severity violations were found. `Warning`s do not
    /// invalidate a document.
    pub fn is_valid(&self) -> bool {
        !self
            .violations
            .iter()
            .any(|v| v.severity == Severity::Error)
    }
}

/// Validate a `.ndoc.typ` fat-file document against the built-in catalogue.
///
/// Reads and parses the document at `path`. If parsing fails (e.g. a missing
/// or malformed document header) the parse error is recorded as a
/// [`Violation`] rather than propagated as an `Err`. Structural constraints
/// that survive parsing — currently entry-name uniqueness — are checked
/// without fail-fast behaviour.
///
/// Returns `Err` only for I/O failures.
pub fn validate_ndoc_file(path: &Path) -> Result<ValidationResult> {
    let src = std::fs::read_to_string(path)?;

    // Canonical composed documents (the `/*===STATE-START===` reference format)
    // are validated for structural integrity rather than entry-name rules,
    // which apply only to the entry-format archive.
    if crate::fatfile::composed::is_composed(&src) {
        return Ok(validate_composed(&src, path));
    }

    let mut violations = Vec::new();

    let doc = match NdocDocument::parse(&src) {
        Ok(d) => d,
        Err(e) => {
            violations.push(Violation::error(
                "invalid-document",
                path.display().to_string(),
                e.to_string(),
            ));
            return Ok(ValidationResult {
                violations,
                summary: None,
            });
        }
    };

    let catalogue = Catalogue::from_builtins();

    for rule in &catalogue.document_constraints.rules {
        match rule.kind {
            ConstraintKind::EntryNameUniqueness => {
                let mut seen = std::collections::HashSet::new();
                for entry in &doc.entries {
                    if !seen.insert(entry.name.as_str()) {
                        violations.push(Violation::error(
                            "duplicate-entry-name",
                            format!("{}:{}", path.display(), entry.name),
                            format!(
                                "duplicate entry name '{}': {}",
                                entry.name, rule.description
                            ),
                        ));
                    }
                }
            }
            // HeaderPresence and ValidEntryKind are enforced by NdocDocument::parse;
            // reaching this point means those constraints are already satisfied.
            // MarkdownFrontmatterValid applies only to .md files.
            ConstraintKind::HeaderPresence
            | ConstraintKind::ValidEntryKind
            | ConstraintKind::MarkdownFrontmatterValid => {}
        }
    }

    Ok(ValidationResult {
        violations,
        summary: None,
    })
}

/// Validate a canonical composed fat file: structural integrity (STATE prelude,
/// IMAGES section) plus schema conformance of the node tree and inputs against
/// the built-in [`Catalogue`].
///
/// Problems are recorded as [`Violation`]s (rather than returned as `Err`),
/// mirroring how the entry-format path surfaces parse errors. Schema checks are
/// skipped if the STATE section cannot be parsed.
fn validate_composed(src: &str, path: &Path) -> ValidationResult {
    let mut violations = Vec::new();

    if let Err(e) = crate::fatfile::composed::image_manifest(src) {
        violations.push(Violation::error(
            "invalid-state-prelude",
            path.display().to_string(),
            e.to_string(),
        ));
    }
    if let Err(e) = crate::fatfile::composed::extract_image_blobs(src) {
        violations.push(Violation::error(
            "invalid-images-section",
            format!("{}:images", path.display()),
            e.to_string(),
        ));
    }

    let summary = match crate::fatfile::composed::parse_state(src) {
        Ok(state) => {
            let catalogue = Catalogue::from_builtins();
            violations.extend(validate_state_against_catalogue(&state, &catalogue, path));
            Some(ValidationSummary {
                node_count: count_nodes(&state.nodes),
                global_input_count: state.global_inputs.len(),
                template_id: state.template_id,
                template_version: state.template_version,
                theme_id: state.theme_id,
            })
        }
        Err(e) => {
            violations.push(Violation::error(
                "invalid-state-section",
                path.display().to_string(),
                e.to_string(),
            ));
            None
        }
    };

    ValidationResult {
        violations,
        summary,
    }
}

/// Count every node in the tree, including nested children.
fn count_nodes(nodes: &[crate::fatfile::composed::DocNode]) -> usize {
    nodes.iter().map(|n| 1 + count_nodes(&n.children)).sum()
}

/// Validate a parsed [`DocState`] against the catalogue's template and component
/// schemas. Ports the C# `DocumentAuthoringService.ValidateDocument` rules:
/// global-input and per-node unknown-input / type-mismatch (errors), the
/// node-tree structural checks (component-not-allowed, leaf-has-children,
/// child-not-allowed; errors), and missing-required-input (warnings).
fn validate_state_against_catalogue(
    state: &crate::fatfile::composed::DocState,
    catalogue: &Catalogue,
    path: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let loc = path.display().to_string();

    // Global inputs against the template's document inputs.
    match catalogue.template(&state.template_id) {
        None => violations.push(Violation::error(
            "template-not-found",
            loc.clone(),
            format!(
                "template '{}' is not in the built-in catalogue",
                state.template_id
            ),
        )),
        Some(template) => {
            for (name, value) in &state.global_inputs {
                match template.document_inputs.iter().find(|d| &d.name == name) {
                    None => violations.push(Violation::error(
                        "unknown-input",
                        format!("{loc}:{name}"),
                        format!(
                            "global input '{name}' is not defined on template '{}'",
                            state.template_id
                        ),
                    )),
                    Some(def) if !input_matches_kind(value, def.kind) => {
                        violations.push(Violation::error(
                            "input-type-mismatch",
                            format!("{loc}:{name}"),
                            format!(
                                "global input '{name}' expects {:?} but received {}",
                                def.kind,
                                describe_input(value)
                            ),
                        ));
                    }
                    Some(_) => {}
                }
            }

            let allowed = &template.allowed_components;
            for node in &state.nodes {
                if !allowed.is_empty() && !allowed.contains(&node.component_type) {
                    violations.push(Violation::error(
                        "component-not-allowed",
                        node.id.clone(),
                        format!(
                            "component '{}' is not in the template's allowed components ({})",
                            node.component_type,
                            allowed.join(", ")
                        ),
                    ));
                }
            }
        }
    }

    for node in &state.nodes {
        validate_node(node, catalogue, &mut violations);
    }

    violations
}

/// Recursively validate one node and its children against component schemas.
fn validate_node(
    node: &crate::fatfile::composed::DocNode,
    catalogue: &Catalogue,
    violations: &mut Vec<Violation>,
) {
    let Some(schema) = catalogue.component(&node.component_type) else {
        violations.push(Violation::error(
            "component-not-allowed",
            node.id.clone(),
            format!(
                "no component schema found for '{}' in the built-in catalogue",
                node.component_type
            ),
        ));
        return;
    };

    // Leaf components may not carry children.
    if !schema.has_body && !node.children.is_empty() {
        violations.push(Violation::error(
            "leaf-has-children",
            node.id.clone(),
            format!(
                "component '{}' is declared hasBody: false and cannot carry child nodes",
                node.component_type
            ),
        ));
    }

    // Per-input unknown-key and type-mismatch checks.
    for (name, value) in &node.inputs {
        match schema.inputs.iter().find(|i| &i.name == name) {
            None => violations.push(Violation::error(
                "unknown-input",
                format!("{}:{name}", node.id),
                format!(
                    "input '{name}' is not declared on component '{}'",
                    node.component_type
                ),
            )),
            Some(def) if !input_matches_kind(value, def.kind) => {
                violations.push(Violation::error(
                    "input-type-mismatch",
                    format!("{}:{name}", node.id),
                    format!(
                        "input '{name}' on component '{}' expects {:?} but received {}",
                        node.component_type,
                        def.kind,
                        describe_input(value)
                    ),
                ));
            }
            Some(_) => {}
        }
    }

    // Missing required inputs are warnings, not errors.
    for def in schema.inputs.iter().filter(|i| i.required) {
        if !node.inputs.contains_key(&def.name) {
            violations.push(Violation::warning(
                "required-input-missing",
                format!("{}:{}", node.id, def.name),
                format!(
                    "required input '{}' is missing on component '{}'",
                    def.name, node.component_type
                ),
            ));
        }
    }

    // Children must be in the parent's allowedChildren (when constrained).
    for child in &node.children {
        if !schema.allowed_children.is_empty()
            && !schema.allowed_children.contains(&child.component_type)
        {
            violations.push(Violation::error(
                "child-not-allowed",
                child.id.clone(),
                format!(
                    "component '{}' is not an allowed child of '{}' (allowed: {})",
                    child.component_type,
                    node.component_type,
                    schema.allowed_children.join(", ")
                ),
            ));
        }
        validate_node(child, catalogue, violations);
    }
}

/// Whether a parsed input value is assignable to a declared [`InputKind`].
///
/// Ports the C# `IsValueAssignable` / `TryCoerce`: strings satisfy the
/// string-like kinds; numbers and booleans accept either a native scalar or a
/// parseable string.
fn input_matches_kind(
    value: &crate::fatfile::composed::ParsedInput,
    kind: crate::model::InputKind,
) -> bool {
    use crate::fatfile::composed::ParsedInput;
    use crate::model::InputKind;

    let as_str = |v: &serde_yaml_ng::Value| v.as_str().map(str::to_string);
    match value {
        ParsedInput::Content(_) => matches!(
            kind,
            InputKind::String | InputKind::Content | InputKind::Color | InputKind::Image
        ),
        ParsedInput::Scalar(v) => match kind {
            InputKind::String | InputKind::Content | InputKind::Color | InputKind::Image => {
                v.is_string()
            }
            InputKind::Number => {
                v.is_i64()
                    || v.is_u64()
                    || v.is_f64()
                    || as_str(v).is_some_and(|s| s.trim().parse::<f64>().is_ok())
            }
            InputKind::Boolean => {
                v.is_bool()
                    || as_str(v).is_some_and(|s| {
                        let t = s.trim();
                        t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false")
                    })
            }
        },
    }
}

/// Render a parsed input value for an error message.
fn describe_input(value: &crate::fatfile::composed::ParsedInput) -> String {
    use crate::fatfile::composed::ParsedInput;
    match value {
        ParsedInput::Content(_) => "markdown content".to_string(),
        ParsedInput::Scalar(v) => format!(
            "'{}'",
            serde_yaml_ng::to_string(v).unwrap_or_default().trim()
        ),
    }
}

/// Validate a `.md` Markdown file.
///
/// Checks that the optional YAML frontmatter block (if present) parses
/// without errors. Returns an unsupported-file-type [`Violation`] for any
/// path whose extension is not `md`.
///
/// Returns `Err` only for I/O failures.
pub fn validate_markdown_file(path: &Path) -> Result<ValidationResult> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension != "md" {
        return Ok(ValidationResult {
            violations: vec![Violation::error(
                "unsupported-file-type",
                path.display().to_string(),
                format!(
                    "unsupported file type '{}': expected a '.ndoc.typ' or '.md' file",
                    path.display()
                ),
            )],
            summary: None,
        });
    }

    let src = std::fs::read_to_string(path)?;
    let mut violations = Vec::new();

    let catalogue = Catalogue::from_builtins();

    if catalogue
        .document_constraints
        .rules
        .iter()
        .any(|r| r.kind == ConstraintKind::MarkdownFrontmatterValid)
    {
        if let Some(fm_str) = extract_frontmatter(&src) {
            if let Err(e) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(fm_str) {
                violations.push(Violation::error(
                    "invalid-frontmatter",
                    format!("{}:frontmatter", path.display()),
                    format!("invalid YAML frontmatter: {e}"),
                ));
            }
        }
    }

    Ok(ValidationResult {
        violations,
        summary: None,
    })
}

/// Extract the raw YAML content between a leading `---` frontmatter fence.
///
/// Returns `None` if the source does not begin with a `---` delimiter.
fn extract_frontmatter(src: &str) -> Option<&str> {
    let body = src
        .strip_prefix("---\n")
        .or_else(|| src.strip_prefix("---\r\n"))?;

    let end = body
        .find("\n---\n")
        .or_else(|| body.find("\n---\r\n"))
        .or_else(|| body.find("\n---"));

    end.map(|pos| &body[..pos])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fatfile::ndoc::{compute_entry_hash, EntryKind, NdocDocument, NdocEntry};

    fn write_temp(suffix: &str, content: &str) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new()
            .suffix(suffix)
            .tempfile()
            .expect("temp file");
        std::fs::write(file.path(), content).expect("write temp file");
        file
    }

    // --- validate_ndoc_file ---

    #[test]
    fn validate_valid_ndoc_file_no_violations() {
        let doc = NdocDocument::new();
        let raw = doc.compose();
        let file = write_temp(".ndoc.typ", &raw);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            result.is_valid(),
            "empty ndoc document must have no violations"
        );
    }

    #[test]
    fn validate_ndoc_file_invalid_header_returns_violation() {
        let file = write_temp(".ndoc.typ", "not a valid ndoc document\n");
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "malformed header must produce at least one violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("invalid document header"),
            "violation message must describe the bad header"
        );
    }

    #[test]
    fn validate_ndoc_file_duplicate_entry_names_returns_violation() {
        let mut doc = NdocDocument::new();
        doc.entries.push(NdocEntry {
            name: "mycomp".to_string(),
            kind: EntryKind::Component,
            content: "first".to_string(),
            hash: compute_entry_hash("first"),
        });
        doc.entries.push(NdocEntry {
            name: "mycomp".to_string(),
            kind: EntryKind::Component,
            content: "second".to_string(),
            hash: compute_entry_hash("second"),
        });
        let raw = doc.compose();
        let file = write_temp(".ndoc.typ", &raw);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "duplicate entry names must produce at least one violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("duplicate entry name"),
            "violation message must name the constraint"
        );
    }

    // --- validate_ndoc_file: composed format ---

    /// Build a minimal composed document, optionally with a malformed prelude
    /// or a duplicated image hash, for the composed-format validation tests.
    fn composed_doc(prelude: &str, images: &str) -> String {
        format!(
            "/*===STATE-START===\n{prelude}\n===STATE-END===*/\n\
             /*===IMAGES-START===\n{images}===IMAGES-END===*/\n\
             // ===TEMPLATE-START===\n// ===TEMPLATE-END===\n\
             // ===DOCUMENT-START===\n= Doc\n"
        )
    }

    #[test]
    fn validate_composed_ndoc_file_no_violations() {
        // `default` is a built-in template; with no nodes/global inputs the
        // document is structurally and schema-clean.
        let src = composed_doc(
            "{ \"templateId\": \"default\", \"images\": [ { \"name\": \"a.svg\", \"hash\": \"aaaa\" } ] }",
            "---aaaa---\nAA==\n---END---\n",
        );
        let file = write_temp(".ndoc.typ", &src);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            result.is_valid(),
            "well-formed composed document must have no violations, got {:?}",
            result.violations
        );
    }

    #[test]
    fn validate_composed_ndoc_file_bad_prelude_returns_violation() {
        let src = composed_doc("{ not valid json", "");
        let file = write_temp(".ndoc.typ", &src);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "malformed prelude must produce a violation"
        );
        assert!(
            result.violations[0].message.contains("prelude"),
            "violation must describe the prelude problem, got {:?}",
            result.violations[0].message
        );
    }

    #[test]
    fn validate_composed_ndoc_file_duplicate_image_hash_returns_violation() {
        let src = composed_doc(
            "{ \"templateId\": \"t\", \"images\": [] }",
            "---aaaa---\nAA==\n---END---\n---aaaa---\nAQ==\n---END---\n",
        );
        let file = write_temp(".ndoc.typ", &src);
        let result = validate_ndoc_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "duplicate image hash must produce a violation"
        );
        assert!(
            result.violations[0].message.contains("duplicate hash"),
            "violation must name the duplicate-hash problem, got {:?}",
            result.violations[0].message
        );
    }

    // --- composed schema validation (validate_state_against_catalogue) ---

    use crate::fatfile::composed::{DocNode, DocState, ParsedInput};
    use crate::model::InputKind;
    use crate::schema::{ComponentSchema, InputSchema, TemplateSchema};

    fn scalar_str(s: &str) -> ParsedInput {
        ParsedInput::Scalar(serde_yaml_ng::Value::String(s.to_string()))
    }
    fn scalar_num(n: i64) -> ParsedInput {
        ParsedInput::Scalar(serde_yaml_ng::Value::Number(n.into()))
    }
    fn input_def(name: &str, kind: InputKind, required: bool) -> InputSchema {
        InputSchema {
            name: name.to_string(),
            kind,
            required,
        }
    }

    /// A catalogue with a body-bearing `section` (requires `heading`, allows
    /// only `leaf` children), a leaf `leaf`, and an `extra` that the template
    /// does not allow.
    fn cat() -> Catalogue {
        Catalogue {
            components: vec![
                ComponentSchema {
                    name: "section".to_string(),
                    has_body: true,
                    allowed_children: vec!["leaf".to_string()],
                    inputs: vec![input_def("heading", InputKind::String, true)],
                },
                ComponentSchema {
                    name: "leaf".to_string(),
                    has_body: false,
                    allowed_children: Vec::new(),
                    inputs: Vec::new(),
                },
                ComponentSchema {
                    name: "extra".to_string(),
                    has_body: true,
                    allowed_children: Vec::new(),
                    inputs: Vec::new(),
                },
            ],
            templates: vec![TemplateSchema {
                name: "tpl".to_string(),
                document_inputs: vec![input_def("title", InputKind::String, false)],
                allowed_components: vec!["section".to_string(), "leaf".to_string()],
            }],
            document_constraints: Default::default(),
        }
    }

    fn node(
        id: &str,
        ty: &str,
        inputs: Vec<(&str, ParsedInput)>,
        children: Vec<DocNode>,
    ) -> DocNode {
        DocNode {
            id: id.to_string(),
            component_type: ty.to_string(),
            inputs: inputs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            children,
        }
    }

    fn state(global: Vec<(&str, ParsedInput)>, nodes: Vec<DocNode>) -> DocState {
        DocState {
            template_id: "tpl".to_string(),
            template_version: "1".to_string(),
            theme_id: "th".to_string(),
            global_inputs: global
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            nodes,
        }
    }

    fn run(s: &DocState) -> Vec<Violation> {
        validate_state_against_catalogue(s, &cat(), Path::new("doc.ndoc.typ"))
    }

    fn codes(vs: &[Violation]) -> Vec<&str> {
        vs.iter().map(|v| v.code.as_str()).collect()
    }

    #[test]
    fn schema_valid_document_has_no_violations() {
        let s = state(
            vec![("title", scalar_str("Hi"))],
            vec![node(
                "section-1",
                "section",
                vec![("heading", scalar_str("H"))],
                vec![node("leaf-1", "leaf", vec![], vec![])],
            )],
        );
        assert!(run(&s).is_empty(), "valid document must produce no issues");
    }

    #[test]
    fn schema_unknown_global_input_is_error() {
        let s = state(vec![("bogus", scalar_str("x"))], vec![]);
        let vs = run(&s);
        assert_eq!(codes(&vs), vec!["unknown-input"]);
        assert_eq!(vs[0].severity, Severity::Error);
    }

    #[test]
    fn schema_global_input_type_mismatch_is_error() {
        let s = state(vec![("title", scalar_num(7))], vec![]);
        let vs = run(&s);
        assert_eq!(codes(&vs), vec!["input-type-mismatch"]);
    }

    #[test]
    fn schema_component_not_in_template_allowed_is_error() {
        // `extra` has a schema but is not in the template's allowedComponents.
        let s = state(vec![], vec![node("extra-1", "extra", vec![], vec![])]);
        assert!(codes(&run(&s)).contains(&"component-not-allowed"));
    }

    #[test]
    fn schema_unknown_component_is_error() {
        let s = state(vec![], vec![node("ghost-1", "ghost", vec![], vec![])]);
        assert!(
            codes(&run(&s)).contains(&"component-not-allowed"),
            "a node with no schema must be flagged"
        );
    }

    #[test]
    fn schema_leaf_with_children_is_error() {
        let s = state(
            vec![],
            vec![node(
                "leaf-1",
                "leaf",
                vec![],
                vec![node("leaf-2", "leaf", vec![], vec![])],
            )],
        );
        assert!(codes(&run(&s)).contains(&"leaf-has-children"));
    }

    #[test]
    fn schema_child_not_in_allowed_children_is_error() {
        let s = state(
            vec![],
            vec![node(
                "section-1",
                "section",
                vec![("heading", scalar_str("H"))],
                vec![node("extra-1", "extra", vec![], vec![])],
            )],
        );
        assert!(codes(&run(&s)).contains(&"child-not-allowed"));
    }

    #[test]
    fn schema_unknown_node_input_is_error() {
        let s = state(
            vec![],
            vec![node(
                "section-1",
                "section",
                vec![("heading", scalar_str("H")), ("foo", scalar_str("x"))],
                vec![],
            )],
        );
        assert!(codes(&run(&s)).contains(&"unknown-input"));
    }

    #[test]
    fn schema_node_input_type_mismatch_is_error() {
        // `heading` is String; a numeric scalar does not satisfy it.
        let s = state(
            vec![],
            vec![node(
                "section-1",
                "section",
                vec![("heading", scalar_num(3))],
                vec![],
            )],
        );
        assert!(codes(&run(&s)).contains(&"input-type-mismatch"));
    }

    #[test]
    fn schema_missing_required_input_is_warning_not_error() {
        // `section` requires `heading`; omitting it is advisory only.
        let s = state(vec![], vec![node("section-1", "section", vec![], vec![])]);
        let vs = run(&s);
        assert_eq!(codes(&vs), vec!["required-input-missing"]);
        assert_eq!(vs[0].severity, Severity::Warning);
        let result = ValidationResult {
            violations: vs,
            summary: None,
        };
        assert!(result.is_valid(), "a warning-only document is still valid");
    }

    #[test]
    fn schema_content_value_satisfies_string_input() {
        // A content-typed value is assignable to a string-like declared input.
        let s = state(
            vec![],
            vec![node(
                "section-1",
                "section",
                vec![("heading", ParsedInput::Content("body".to_string()))],
                vec![],
            )],
        );
        assert!(run(&s).is_empty(), "content satisfies a String input");
    }

    #[test]
    fn schema_numeric_string_satisfies_number_input() {
        use crate::fatfile::composed::ParsedInput;
        assert!(input_matches_kind(&scalar_str("42"), InputKind::Number));
        assert!(input_matches_kind(
            &ParsedInput::Scalar(serde_yaml_ng::Value::Bool(true)),
            InputKind::Boolean
        ));
        assert!(!input_matches_kind(&scalar_str("nope"), InputKind::Number));
    }

    // --- validate_markdown_file ---

    #[test]
    fn validate_valid_markdown_file_no_violations() {
        let content = "---\ntitle: Test Doc\n---\n\n# Heading\n\nParagraph.\n";
        let file = write_temp(".md", content);
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(result.is_valid(), "valid markdown must have no violations");
    }

    #[test]
    fn validate_markdown_file_no_frontmatter_no_violations() {
        let content = "# Simple Heading\n\nJust a paragraph.\n";
        let file = write_temp(".md", content);
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(
            result.is_valid(),
            "markdown without frontmatter must have no violations"
        );
    }

    #[test]
    fn validate_markdown_file_invalid_frontmatter_returns_violation() {
        // Deliberately unclosed flow sequence to force a YAML parse error.
        let content = "---\ntitle: [unclosed bracket\n---\n\n# Heading\n";
        let file = write_temp(".md", content);
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "invalid YAML frontmatter must produce at least one violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("invalid YAML frontmatter"),
            "violation message must describe the frontmatter problem"
        );
    }

    #[test]
    fn validate_unsupported_extension_returns_violation() {
        let file = write_temp(".txt", "some content\n");
        let result = validate_markdown_file(file.path()).expect("I/O succeeds");
        assert!(
            !result.is_valid(),
            "unsupported extension must produce a violation"
        );
        assert!(
            result.violations[0]
                .message
                .contains("unsupported file type"),
            "violation message must flag the unsupported type"
        );
    }
}
