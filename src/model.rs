//! Shared domain types for the document model.
//!
//! Ported (in skeleton form) from the C# core: documents are trees of nodes,
//! each node invokes a component with typed inputs. Node IDs are stable
//! `{type}-{4hex}` strings that survive reorders.

use serde::{Deserialize, Serialize};

/// The kind of a typed component input.
///
/// Mirrors the C# input system: scalar values plus `content` (Markdown that is
/// converted to Typst) and `image` (ingested, embedded, tracked in the fat
/// file's IMAGES manifest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    String,
    Number,
    Boolean,
    Color,
    /// Markdown content, converted to Typst at compose time.
    Content,
    /// Image reference, embedded into the fat file.
    Image,
}

/// A single typed input value attached to a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputValue {
    pub kind: InputKind,
    /// Raw value as authored (JSON-encoded); interpretation depends on `kind`.
    pub value: serde_json::Value,
}

/// A stable node identifier of the form `{type}-{4hex}`, e.g. `heading-1a2b`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A node in the document tree: a component invocation with inputs and children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    /// Component name this node invokes.
    pub component: String,
    /// Named inputs supplied to the component.
    #[serde(default)]
    pub inputs: std::collections::BTreeMap<String, InputValue>,
    /// Child nodes, in document order.
    #[serde(default)]
    pub children: Vec<Node>,
}

/// A whole document: the root tree plus document-level inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Template this document is bound to.
    pub template: String,
    /// Document-level inputs (the `documentInputs` of the template).
    #[serde(default)]
    pub inputs: std::collections::BTreeMap<String, InputValue>,
    /// Top-level nodes, in document order.
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_display_formats_inner_string() {
        let id = NodeId("heading-1a2b".to_string());
        assert_eq!(format!("{id}"), "heading-1a2b");
    }

    #[test]
    fn input_value_serde_round_trip() {
        let iv = InputValue {
            kind: InputKind::String,
            value: serde_json::json!("hello"),
        };
        let json = serde_json::to_string(&iv).expect("serialize InputValue");
        let back: InputValue = serde_json::from_str(&json).expect("deserialize InputValue");
        assert_eq!(iv, back);
    }

    #[test]
    fn node_with_children_serde_round_trip() {
        let child = Node {
            id: NodeId("para-0001".to_string()),
            component: "paragraph".to_string(),
            inputs: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    "text".to_string(),
                    InputValue {
                        kind: InputKind::Content,
                        value: serde_json::json!("body text"),
                    },
                );
                m
            },
            children: Vec::new(),
        };
        let node = Node {
            id: NodeId("section-aabb".to_string()),
            component: "section".to_string(),
            inputs: std::collections::BTreeMap::new(),
            children: vec![child],
        };
        let json = serde_json::to_string(&node).expect("serialize Node");
        let back: Node = serde_json::from_str(&json).expect("deserialize Node");
        assert_eq!(node, back);
    }

    #[test]
    fn document_serde_round_trip() {
        let doc = Document {
            template: "default".to_string(),
            inputs: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    "title".to_string(),
                    InputValue {
                        kind: InputKind::String,
                        value: serde_json::json!("My Doc"),
                    },
                );
                m
            },
            nodes: vec![Node {
                id: NodeId("heading-1234".to_string()),
                component: "heading".to_string(),
                inputs: std::collections::BTreeMap::new(),
                children: Vec::new(),
            }],
        };
        let json = serde_json::to_string(&doc).expect("serialize Document");
        let back: Document = serde_json::from_str(&json).expect("deserialize Document");
        assert_eq!(doc, back);
    }

    #[test]
    fn input_kind_serde_uses_lowercase_names() {
        let cases: &[(InputKind, &str)] = &[
            (InputKind::String, "\"string\""),
            (InputKind::Number, "\"number\""),
            (InputKind::Boolean, "\"boolean\""),
            (InputKind::Color, "\"color\""),
            (InputKind::Content, "\"content\""),
            (InputKind::Image, "\"image\""),
        ];
        for (kind, expected) in cases {
            let json = serde_json::to_string(kind).expect("serialize InputKind");
            assert_eq!(
                json, *expected,
                "InputKind::{kind:?} must serialize to lowercase name"
            );
            let back: InputKind = serde_json::from_str(&json).expect("deserialize InputKind");
            assert_eq!(
                back, *kind,
                "InputKind::{kind:?} must round-trip through JSON"
            );
        }
    }
}
