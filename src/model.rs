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

impl NodeId {
    /// Mint a `{component}-{4hex}` id that does not collide with `existing`.
    ///
    /// The 4 hex digits are derived deterministically from the component name
    /// and a salt; on collision the salt is advanced until a free id is found.
    /// Determinism keeps fat-file output stable for a given mint order, which
    /// matters for snapshot tests, while the existence check guarantees the id
    /// is unique within the document.
    pub fn mint(component: &str, existing: &std::collections::HashSet<NodeId>) -> NodeId {
        // 65_536 distinct 4-hex suffixes exist per component; the loop is bounded
        // by that space and terminates well before exhaustion for real documents.
        for salt in 0u32..=u32::from(u16::MAX) {
            let mut hasher = blake3::Hasher::new();
            hasher.update(component.as_bytes());
            hasher.update(&salt.to_le_bytes());
            let digest = hasher.finalize();
            let suffix = u16::from_le_bytes([digest.as_bytes()[0], digest.as_bytes()[1]]);
            let candidate = NodeId(format!("{component}-{suffix:04x}"));
            if !existing.contains(&candidate) {
                return candidate;
            }
        }
        // Suffix space exhausted (>65k nodes of one component): fall back to a
        // wider hex suffix that is still unique by construction.
        NodeId(format!("{component}-{:08x}", existing.len()))
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

/// A reference to an image embedded in the fat file's IMAGES section.
///
/// The manifest entry pairs the original file `name` with the blake3 `hash` of
/// its bytes. The bytes themselves live (base64-encoded, keyed by `hash`) in the
/// `// === IMAGES ===` section, so identical content shared by several names is
/// stored only once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRef {
    /// Original file name of the embedded image (e.g. `logo.png`).
    pub name: String,
    /// 64-character lowercase blake3 hex digest of the image bytes.
    pub hash: String,
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
    /// Embedded image manifest: `{name, hash}` entries whose bytes live in the
    /// IMAGES section. Empty by default and omitted from serialised STATE when
    /// no images are embedded, so documents that predate image support and
    /// documents with no images round-trip unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageRef>,
}

impl Document {
    /// Collect every node id in the tree, in document order, into a set.
    ///
    /// Used by [`NodeId::mint`] callers to guarantee freshly minted ids do not
    /// collide with any existing node, at any depth.
    pub fn node_ids(&self) -> std::collections::HashSet<NodeId> {
        fn walk(node: &Node, acc: &mut std::collections::HashSet<NodeId>) {
            acc.insert(node.id.clone());
            for child in &node.children {
                walk(child, acc);
            }
        }
        let mut ids = std::collections::HashSet::new();
        for node in &self.nodes {
            walk(node, &mut ids);
        }
        ids
    }
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
            images: Vec::new(),
        };
        let json = serde_json::to_string(&doc).expect("serialize Document");
        let back: Document = serde_json::from_str(&json).expect("deserialize Document");
        assert_eq!(doc, back);
    }

    #[test]
    fn mint_produces_type_dash_4hex_format() {
        let id = NodeId::mint("heading", &std::collections::HashSet::new());
        let suffix =
            id.0.strip_prefix("heading-")
                .expect("minted id must carry the component prefix");
        assert_eq!(
            suffix.len(),
            4,
            "minted suffix must be exactly 4 hex digits"
        );
        assert!(
            suffix.chars().all(|c| c.is_ascii_hexdigit()),
            "minted suffix must be hex: {suffix:?}"
        );
    }

    #[test]
    fn mint_avoids_collision_with_existing_ids() {
        let mut existing = std::collections::HashSet::new();
        let mut last = NodeId::mint("para", &existing);
        for _ in 0..50 {
            assert!(
                !existing.contains(&last),
                "mint returned an id already in the set: {last}"
            );
            existing.insert(last.clone());
            last = NodeId::mint("para", &existing);
        }
        assert_eq!(existing.len(), 50, "every minted id must be unique");
    }

    #[test]
    fn node_ids_collects_nested_ids() {
        let doc = Document {
            template: "t".to_string(),
            inputs: std::collections::BTreeMap::new(),
            nodes: vec![Node {
                id: NodeId("section-aabb".to_string()),
                component: "section".to_string(),
                inputs: std::collections::BTreeMap::new(),
                children: vec![Node {
                    id: NodeId("para-0001".to_string()),
                    component: "paragraph".to_string(),
                    inputs: std::collections::BTreeMap::new(),
                    children: Vec::new(),
                }],
            }],
            images: Vec::new(),
        };
        let ids = doc.node_ids();
        assert!(ids.contains(&NodeId("section-aabb".to_string())));
        assert!(ids.contains(&NodeId("para-0001".to_string())));
        assert_eq!(ids.len(), 2);
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
