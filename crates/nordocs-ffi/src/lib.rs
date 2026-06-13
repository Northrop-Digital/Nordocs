//! nordocs-ffi — a C-ABI / .NET FFI binding over the `nordocs-core` engine.
//!
//! This crate is a `cdylib` that exposes the binding-agnostic
//! [`nordocs_core::service`] façade to C# (and any C-ABI consumer) through
//! [interoptopus]-annotated exports. It adds no engine logic of its own: each
//! export marshals inputs, calls the façade under the panic guard, and marshals
//! the result back.
//!
//! ## Foundation (this module group)
//!
//! - [`error`] — the structured FFI error (`code + message`) and the mapping
//!   from every [`nordocs_core::Error`] variant.
//! - [`guard`] — the [`std::panic::catch_unwind`] wrapper so neither a `Result`
//!   nor a panic crosses the ABI.
//! - [`marshal`] — flat `#[repr(C)]` byte-buffer and UTF-8 string DTOs, with
//!   explicit length/capacity and matching `free` exports.
//!
//! The compile / markdown / preview / authoring / validation / session exports
//! are layered on top of this foundation by later task groups.
//!
//! [interoptopus]: https://github.com/ralfbiedert/interoptopus

pub mod authoring;
pub mod compile;
pub mod convert;
pub mod error;
pub mod guard;
pub mod marshal;
pub mod session;

// Binding generation + reference-parity checks (task group 5). Test-only: it
// uses the C# backend (a dev-dependency) and never ships in the cdylib.
#[cfg(test)]
mod generate;

pub use error::{ndoc_error_free, FfiError, FfiErrorCode};
pub use marshal::{ndoc_byte_buffer_free, ndoc_string_free, ByteBuffer, FfiString};

pub use compile::{
    ndoc_compile, ndoc_compile_file_to_pdf, ndoc_compile_result_free, ndoc_compile_to_pdf,
    ndoc_markdown_to_typst, CompileResult, FfiFormat,
};

pub use authoring::{
    ndoc_add_entry, ndoc_catalogue, ndoc_component_schema, ndoc_create_document, ndoc_edit_entry,
    ndoc_embed_image, ndoc_item_collections, ndoc_read_document, ndoc_render_component_preview,
    ndoc_render_document_preview, ndoc_template_schema, ndoc_validate_file, ndoc_write_document,
};

pub use session::{
    ndoc_compile_session, ndoc_session_free, ndoc_session_jump_from_click,
    ndoc_session_jump_from_cursor, ndoc_session_page_count, ndoc_session_page_size,
    ndoc_session_png, ndoc_session_svg, CompiledSession, FfiPageSize,
};

/// The interoptopus inventory of FFI symbols.
///
/// This is the single source the C# backend generator reads (task group 5). The
/// foundation registers the marshalling/error free exports and the flat DTOs;
/// later task groups register the compile/markdown/preview/session functions.
#[allow(dead_code)] // The generation entry point (task group 5) calls this.
pub fn ffi_inventory() -> interoptopus::Inventory {
    use interoptopus::{extra_type, function, InventoryBuilder};

    InventoryBuilder::new()
        // Foundation: marshalling/error free exports and flat DTOs.
        .register(function!(ndoc_byte_buffer_free))
        .register(function!(ndoc_string_free))
        .register(function!(ndoc_error_free))
        .register(extra_type!(ByteBuffer))
        .register(extra_type!(FfiString))
        .register(extra_type!(FfiError))
        .register(extra_type!(FfiErrorCode))
        // Group 2: compiler + markdown.
        .register(function!(ndoc_compile_to_pdf))
        .register(function!(ndoc_compile_file_to_pdf))
        .register(function!(ndoc_compile))
        .register(function!(ndoc_compile_result_free))
        .register(function!(ndoc_markdown_to_typst))
        .register(extra_type!(FfiFormat))
        .register(extra_type!(CompileResult))
        // Group 3: preview + authoring + validation + catalogues.
        .register(function!(ndoc_render_component_preview))
        .register(function!(ndoc_render_document_preview))
        .register(function!(ndoc_create_document))
        .register(function!(ndoc_add_entry))
        .register(function!(ndoc_edit_entry))
        .register(function!(ndoc_read_document))
        .register(function!(ndoc_write_document))
        .register(function!(ndoc_embed_image))
        .register(function!(ndoc_validate_file))
        .register(function!(ndoc_component_schema))
        .register(function!(ndoc_template_schema))
        .register(function!(ndoc_catalogue))
        .register(function!(ndoc_item_collections))
        // Group 4: source-map session handle.
        .register(function!(ndoc_compile_session))
        .register(function!(ndoc_session_free))
        .register(function!(ndoc_session_page_count))
        .register(function!(ndoc_session_page_size))
        .register(function!(ndoc_session_svg))
        .register(function!(ndoc_session_png))
        .register(function!(ndoc_session_jump_from_click))
        .register(function!(ndoc_session_jump_from_cursor))
        .register(extra_type!(FfiPageSize))
        .register(extra_type!(CompiledSession))
        .inventory()
}

#[cfg(test)]
mod tests {
    use super::*;
    use interoptopus::Inventory;

    #[test]
    fn inventory_registers_the_free_exports() {
        let inventory: Inventory = ffi_inventory();
        let names: Vec<&str> = inventory.functions().iter().map(|f| f.name()).collect();
        assert!(names.contains(&"ndoc_byte_buffer_free"));
        assert!(names.contains(&"ndoc_string_free"));
        assert!(names.contains(&"ndoc_error_free"));
    }
}
