//! nordocs-core — the binding-agnostic engine of the nordocs Typst toolset.
//!
//! This crate embeds the native `typst` compiler (no external process) and
//! exposes the Markdown/data -> Typst -> PDF pipeline, a component/template
//! library, and document validation/preview as a structured-result service
//! façade. It contains no `clap`, no terminal output, and no process or viewer
//! side effects, so it can be linked by any front end — the `ndoc` CLI
//! (`nordocs-cli`), the `.NET` FFI (`nordocs-ffi`), or a future WASM shell.
//!
//! ## Module map
//!
//! - [`typst_world`] — our [`typst::World`] implementation over an in-memory
//!   virtual filesystem plus embedded fonts/packages.
//! - [`compiler`] — high-level compile/export wrapper (`.typ` -> PDF bytes).
//! - [`markdown`] — Markdown -> Typst conversion (comrak AST walk).
//! - [`fatfile`] — the self-contained `.ndoc.typ` "fat file": compose, extract,
//!   hash (STATE / TEMPLATE / DOCUMENT / IMAGES).
//! - [`authoring`] — transactional read-validate-write over fat files.
//! - [`item`] — reusable item collections and schema-driven validation.
//! - [`schema`] — component/template input schema parsing and catalogue.
//! - [`validation`] — schema-based validation for `.ndoc.typ` and `.md` documents.
//! - [`model`] — shared domain types (nodes, inputs, IDs, manifests).
//! - [`preview`] — component/document preview composition: compose the live
//!   TEMPLATE/DOCUMENT Typst and compile it to PDF (the `IPreviewRenderer` shape).
//! - [`service`] — the binding-agnostic façade: structured-result compile /
//!   render / convert operations with no I/O or process side effects, called by
//!   the CLI and the FFI alike (the `core-api` capability).
//! - [`error`] — typed library errors.

pub mod authoring;
pub mod compiler;
pub mod error;
pub mod fatfile;
pub mod item;
pub mod markdown;
pub mod model;
pub mod preview;
pub mod schema;
pub mod service;
pub mod typst_world;
pub mod validation;

pub use compiler::{CompiledDoc, Jump, Position};
pub use error::{Error, Result};
