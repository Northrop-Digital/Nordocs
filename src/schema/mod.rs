//! Component / template input schemas and catalogue.
//!
//! Ports the C# `SchemaParser` / `Catalogue` and the resolver layer. Components
//! (`.ncmp.typ`) and templates (`.ndoct.typ`) declare typed inputs; this module
//! parses those declarations into [`ComponentSchema`] and resolves which
//! components a template permits (`allowedComponents`).

use crate::model::InputKind;

/// A single declared input on a component or template.
#[derive(Debug, Clone, PartialEq)]
pub struct InputSchema {
    pub name: String,
    pub kind: InputKind,
    pub required: bool,
}

/// The parsed schema for one component (`.ncmp.typ`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentSchema {
    pub name: String,
    pub inputs: Vec<InputSchema>,
}

impl ComponentSchema {
    /// Construct an empty schema for a named component.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inputs: Vec::new(),
        }
    }
}

/// A catalogue of components/templates discovered in a library directory.
///
/// Skeleton holds the parsed component schemas; template resolution and the
/// on-disk discovery walk are added as the resolvers are ported.
#[derive(Debug, Default)]
pub struct Catalogue {
    pub components: Vec<ComponentSchema>,
}

impl Catalogue {
    /// Create an empty catalogue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a component schema by name.
    pub fn component(&self, name: &str) -> Option<&ComponentSchema> {
        self.components.iter().find(|c| c.name == name)
    }
}
