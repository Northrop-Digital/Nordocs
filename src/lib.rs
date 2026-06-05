//! northdoc — a Rust-native re-implementation of the C# Typst document toolset.
//!
//! This crate embeds the native `typst` compiler (no external process) and
//! exposes a CLI-first surface for the Markdown/data -> Typst -> PDF pipeline,
//! a component/template library, and document validation/preview.
//!
//! ## Module map
//!
//! - [`cli`] — clap command definitions and dispatch (binary `ndoc`).
//! - [`typst_world`] — our [`typst::World`] implementation over an in-memory
//!   virtual filesystem plus embedded fonts/packages.
//! - [`compiler`] — high-level compile/export wrapper (`.typ` -> PDF bytes).
//! - [`markdown`] — Markdown -> Typst conversion (comrak AST walk).
//! - [`fatfile`] — the self-contained `.ndoc.typ` "fat file": compose, extract,
//!   hash (STATE / TEMPLATE / DOCUMENT / IMAGES).
//! - [`authoring`] — transactional read-validate-write over fat files.
//! - [`schema`] — component/template input schema parsing and catalogue.
//! - [`validation`] — schema-based validation for `.ndoc.typ` and `.md` documents.
//! - [`model`] — shared domain types (nodes, inputs, IDs, manifests).
//! - [`error`] — typed library errors.

pub mod authoring;
pub mod cli;
pub mod compiler;
pub mod error;
pub mod fatfile;
pub mod markdown;
pub mod model;
pub mod schema;
pub mod typst_world;
pub mod validation;

pub use error::{Error, Result};
