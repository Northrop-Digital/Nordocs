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
