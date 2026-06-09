//! Component / template input schemas and catalogue.
//!
//! Ports the C# `SchemaParser` / `Catalogue` and the resolver layer. Components
//! (`.ncmp.typ`) and templates (`.ndoct.typ`) declare typed inputs; this module
//! parses those declarations into [`ComponentSchema`] and resolves which
//! components a template permits (`allowedComponents`).
//!
//! The [`Catalogue::from_builtins`] constructor returns the canonical built-in
//! catalogue used by `ndoc validate`. It encodes the minimal structural rules for
//! schema validation and preview: four [`DocumentConstraint`] rules and a handful of built-in
//! component / template schemas.

use serde::{Deserialize, Serialize};

use crate::model::InputKind;

pub mod parse;

/// A single declared input on a component or template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSchema {
    pub name: String,
    pub kind: InputKind,
    pub required: bool,
}

/// The parsed schema for one component (`.ncmp.typ`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSchema {
    pub name: String,
    pub inputs: Vec<InputSchema>,
    /// Whether the component accepts child nodes. A `has_body: false` (leaf)
    /// component may not carry children. Mirrors the C# `ComponentSchema.HasBody`
    /// and defaults to `true`.
    pub has_body: bool,
    /// Component names permitted as direct children. Empty means unconstrained
    /// (any allowed component may nest). Mirrors `ComponentSchema.AllowedChildren`.
    pub allowed_children: Vec<String>,
}

impl ComponentSchema {
    /// Construct an empty schema for a named component (body-bearing, no child
    /// restriction — the permissive defaults).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inputs: Vec::new(),
            has_body: true,
            allowed_children: Vec::new(),
        }
    }
}

/// The schema for one template (`.ndoct.typ`).
///
/// Templates declare document-level inputs (front matter) and constrain which
/// component types may be used in documents bound to this template.
/// An empty `allowed_components` list means all registered components are permitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateSchema {
    pub name: String,
    /// Document-level inputs the template requires or accepts.
    pub document_inputs: Vec<InputSchema>,
    /// Component names this template permits; empty means all are allowed.
    pub allowed_components: Vec<String>,
}

impl TemplateSchema {
    /// Construct an empty schema for a named template.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            document_inputs: Vec::new(),
            allowed_components: Vec::new(),
        }
    }
}

/// The declarative category of a structural constraint on a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintKind {
    /// The document must begin with the `// ndoc document v1` header line.
    HeaderPresence,
    /// All entry names within the document must be unique.
    EntryNameUniqueness,
    /// Every entry `kind=` attribute must be a known value (`component` or `template`).
    ValidEntryKind,
    /// If a Markdown frontmatter block is present it must parse as valid YAML.
    MarkdownFrontmatterValid,
}

/// A single structural rule that must hold for a document to be considered valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentConstraint {
    pub kind: ConstraintKind,
    /// Human-readable description surfaced to the user when the rule is violated.
    pub description: String,
}

/// The set of structural rules that apply to all documents at the schema level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DocumentConstraints {
    pub rules: Vec<DocumentConstraint>,
}

/// A catalogue of components/templates discovered in a library directory.
///
/// `Catalogue::from_builtins()` returns the canonical built-in catalogue used by
/// `ndoc validate`. Direct construction via `Catalogue::new()` yields an empty
/// catalogue suitable for testing or incremental population.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Catalogue {
    pub components: Vec<ComponentSchema>,
    pub templates: Vec<TemplateSchema>,
    pub document_constraints: DocumentConstraints,
}

impl Catalogue {
    /// Create an empty catalogue with no components, templates, or constraints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the canonical built-in catalogue used by `ndoc validate`.
    ///
    /// Registers the minimal set of component/template schemas and the four
    /// structural document constraints required by `ndoc validate`:
    ///
    /// - [`ConstraintKind::HeaderPresence`] — document must begin with `// ndoc document v1`
    /// - [`ConstraintKind::EntryNameUniqueness`] — no two entries may share a name
    /// - [`ConstraintKind::ValidEntryKind`] — entry `kind=` must be `component` or `template`
    /// - [`ConstraintKind::MarkdownFrontmatterValid`] — YAML frontmatter must parse cleanly
    pub fn from_builtins() -> Self {
        let components = vec![
            ComponentSchema {
                name: "heading".to_string(),
                has_body: true,
                allowed_children: Vec::new(),
                inputs: vec![
                    InputSchema {
                        name: "level".to_string(),
                        kind: InputKind::Number,
                        required: true,
                    },
                    InputSchema {
                        name: "text".to_string(),
                        kind: InputKind::Content,
                        required: true,
                    },
                ],
            },
            ComponentSchema {
                name: "paragraph".to_string(),
                has_body: true,
                allowed_children: Vec::new(),
                inputs: vec![InputSchema {
                    name: "text".to_string(),
                    kind: InputKind::Content,
                    required: true,
                }],
            },
        ];

        let templates = vec![TemplateSchema {
            name: "default".to_string(),
            document_inputs: vec![InputSchema {
                name: "title".to_string(),
                kind: InputKind::String,
                required: false,
            }],
            // Empty: all registered components are permitted.
            allowed_components: Vec::new(),
        }];

        let document_constraints = DocumentConstraints {
            rules: vec![
                DocumentConstraint {
                    kind: ConstraintKind::HeaderPresence,
                    description: "document must begin with '// ndoc document v1'".to_string(),
                },
                DocumentConstraint {
                    kind: ConstraintKind::EntryNameUniqueness,
                    description: "all entry names within the document must be unique".to_string(),
                },
                DocumentConstraint {
                    kind: ConstraintKind::ValidEntryKind,
                    description: "entry kind must be 'component' or 'template'".to_string(),
                },
                DocumentConstraint {
                    kind: ConstraintKind::MarkdownFrontmatterValid,
                    description: "YAML frontmatter block must parse without errors".to_string(),
                },
            ],
        };

        Self {
            components,
            templates,
            document_constraints,
        }
    }

    /// Look up a component schema by name.
    pub fn component(&self, name: &str) -> Option<&ComponentSchema> {
        self.components.iter().find(|c| c.name == name)
    }

    /// Look up a template schema by name.
    pub fn template(&self, name: &str) -> Option<&TemplateSchema> {
        self.templates.iter().find(|t| t.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_new_is_empty() {
        let cat = Catalogue::new();
        assert!(cat.components.is_empty());
        assert!(cat.templates.is_empty());
        assert!(cat.document_constraints.rules.is_empty());
    }

    #[test]
    fn constraint_kind_serde_uses_snake_case() {
        let cases: &[(ConstraintKind, &str)] = &[
            (ConstraintKind::HeaderPresence, "\"header_presence\""),
            (
                ConstraintKind::EntryNameUniqueness,
                "\"entry_name_uniqueness\"",
            ),
            (ConstraintKind::ValidEntryKind, "\"valid_entry_kind\""),
            (
                ConstraintKind::MarkdownFrontmatterValid,
                "\"markdown_frontmatter_valid\"",
            ),
        ];
        for (kind, expected) in cases {
            let json = serde_json::to_string(kind).expect("serialize ConstraintKind");
            assert_eq!(json, *expected);
            let back: ConstraintKind =
                serde_json::from_str(&json).expect("deserialize ConstraintKind");
            assert_eq!(back, *kind);
        }
    }

    #[test]
    fn input_schema_serde_round_trip() {
        let schema = InputSchema {
            name: "level".to_string(),
            kind: InputKind::Number,
            required: true,
        };
        let json = serde_json::to_string(&schema).expect("serialize InputSchema");
        let back: InputSchema = serde_json::from_str(&json).expect("deserialize InputSchema");
        assert_eq!(schema, back);
    }

    #[test]
    fn component_schema_new_fields() {
        let schema = ComponentSchema::new("x");
        assert_eq!(schema.name, "x");
        assert!(schema.inputs.is_empty());
    }

    #[test]
    fn template_schema_new_fields() {
        let schema = TemplateSchema::new("x");
        assert_eq!(schema.name, "x");
        assert!(schema.document_inputs.is_empty());
        assert!(schema.allowed_components.is_empty());
    }

    #[test]
    fn schema_round_trip_component() {
        let schema = ComponentSchema {
            name: "heading".to_string(),
            has_body: true,
            allowed_children: Vec::new(),
            inputs: vec![
                InputSchema {
                    name: "level".to_string(),
                    kind: InputKind::Number,
                    required: true,
                },
                InputSchema {
                    name: "text".to_string(),
                    kind: InputKind::Content,
                    required: false,
                },
            ],
        };
        let json = serde_json::to_string(&schema).expect("serialize ComponentSchema");
        let back: ComponentSchema =
            serde_json::from_str(&json).expect("deserialize ComponentSchema");
        assert_eq!(schema, back);
    }

    #[test]
    fn schema_round_trip_template() {
        let schema = TemplateSchema {
            name: "article".to_string(),
            document_inputs: vec![InputSchema {
                name: "title".to_string(),
                kind: InputKind::String,
                required: true,
            }],
            allowed_components: vec!["heading".to_string(), "paragraph".to_string()],
        };
        let json = serde_json::to_string(&schema).expect("serialize TemplateSchema");
        let back: TemplateSchema = serde_json::from_str(&json).expect("deserialize TemplateSchema");
        assert_eq!(schema, back);
    }

    #[test]
    fn catalogue_lookup_component() {
        let catalogue = Catalogue::from_builtins();
        let schema = catalogue
            .component("heading")
            .expect("'heading' is a built-in component");
        assert_eq!(schema.name, "heading");
        assert!(schema.inputs.iter().any(|i| i.name == "level"));
        assert!(schema.inputs.iter().any(|i| i.name == "text"));
    }

    #[test]
    fn catalogue_lookup_template() {
        let catalogue = Catalogue::from_builtins();
        let schema = catalogue
            .template("default")
            .expect("'default' is a built-in template");
        assert_eq!(schema.name, "default");
    }

    #[test]
    fn catalogue_lookup_unknown_returns_none() {
        let catalogue = Catalogue::from_builtins();
        assert!(catalogue.component("nonexistent").is_none());
        assert!(catalogue.template("nonexistent").is_none());
    }

    #[test]
    fn catalogue_document_constraints_include_all_required_kinds() {
        let catalogue = Catalogue::from_builtins();
        let kinds: Vec<ConstraintKind> = catalogue
            .document_constraints
            .rules
            .iter()
            .map(|r| r.kind)
            .collect();
        assert!(kinds.contains(&ConstraintKind::HeaderPresence));
        assert!(kinds.contains(&ConstraintKind::EntryNameUniqueness));
        assert!(kinds.contains(&ConstraintKind::ValidEntryKind));
        assert!(kinds.contains(&ConstraintKind::MarkdownFrontmatterValid));
    }

    #[test]
    fn catalogue_round_trip() {
        let original = Catalogue::from_builtins();
        let json = serde_json::to_string(&original).expect("serialize Catalogue");
        let restored: Catalogue = serde_json::from_str(&json).expect("deserialize Catalogue");
        assert_eq!(original, restored);
    }
}
