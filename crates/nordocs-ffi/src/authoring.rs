//! Preview, authoring, validation, and catalogue exports.
//!
//! Thin wrappers over the `nordocs_core` façade for the operations the C# app
//! drives beyond raw compilation:
//!
//! - **Preview** — [`ndoc_render_component_preview`] / [`ndoc_render_document_preview`]
//!   (the `IPreviewRenderer` surface), returning PDF bytes.
//! - **Authoring** — entry-format create/add/edit, the structured `Document`
//!   read/write round-trip the `doc` node-tree commands build on, and image embed.
//! - **Validation & catalogues** — validate a document, and introspect
//!   component/template schemas, the built-in catalogue, and item collections.
//!
//! Structured results cross the boundary as JSON [`FfiString`]s (the same shape
//! the CLI `--json` envelope emits); binary previews cross as [`ByteBuffer`]s.
//! Every export runs under the panic guard and reports failure via `out_err`.

use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

use interoptopus::ffi_function;
use interoptopus::patterns::slice::FFISlice;
use interoptopus::patterns::string::AsciiPointer;

use nordocs_core::model::Document;
use nordocs_core::schema::{Catalogue, ComponentSchema};

use crate::convert::{arg_str, arg_str_opt, settle, settle_unit};
use crate::error::FfiError;
use crate::guard::run_guarded;
use crate::marshal::{ByteBuffer, FfiString};

/// Parse a JSON object of `name -> value` into ordered input-value pairs.
fn parse_input_values(json: &str) -> nordocs_core::Result<Vec<(String, serde_json::Value)>> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(trimmed)?;
    Ok(map.into_iter().collect())
}

/// Render a single component to PDF for live preview (the
/// `IPreviewRenderer.RenderComponentPreview` surface).
///
/// `schema` is the component's [`ComponentSchema`] as JSON; `input_values` is a
/// JSON object of author-supplied overrides; `theme_code` is an optional theme
/// (null uses the engine's minimal theme). On failure returns the empty buffer
/// and sets `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_render_component_preview(
    component_source: AsciiPointer,
    schema: AsciiPointer,
    input_values: AsciiPointer,
    theme_code: AsciiPointer,
    out_err: &mut FfiError,
) -> ByteBuffer {
    let result = run_guarded(move || {
        let component_source = arg_str(component_source)?;
        let schema: ComponentSchema = serde_json::from_str(&arg_str(schema)?)?;
        let input_values = parse_input_values(&arg_str_opt(input_values).unwrap_or_default())?;
        let theme = arg_str_opt(theme_code);
        nordocs_core::service::render_component_preview(
            &component_source,
            &schema,
            &input_values,
            theme.as_deref(),
        )
    });
    settle(
        result.map(ByteBuffer::from_vec),
        out_err,
        ByteBuffer::empty(),
    )
}

/// Render a complete document to PDF for live preview (the
/// `IPreviewRenderer.RenderDocumentPreview` surface).
///
/// `state` is the [`Document`] as JSON; `component_sources` is a JSON object of
/// `name -> Typst source`; `component_schemas` is a JSON object of
/// `name -> ComponentSchema`. On failure returns the empty buffer and sets
/// `out_err`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_render_document_preview(
    state: AsciiPointer,
    theme_code: AsciiPointer,
    component_sources: AsciiPointer,
    component_schemas: AsciiPointer,
    out_err: &mut FfiError,
) -> ByteBuffer {
    let result = run_guarded(move || {
        let state: Document = serde_json::from_str(&arg_str(state)?)?;
        let theme = arg_str(theme_code)?;
        let sources: BTreeMap<String, String> = serde_json::from_str(&arg_str(component_sources)?)?;
        let schemas: BTreeMap<String, ComponentSchema> =
            serde_json::from_str(&arg_str(component_schemas)?)?;
        nordocs_core::service::render_document_preview(&state, &theme, &sources, &schemas)
    });
    settle(
        result.map(ByteBuffer::from_vec),
        out_err,
        ByteBuffer::empty(),
    )
}

/// Create a new, empty entry-format `.ndoc.typ` document at `path`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_create_document(path: AsciiPointer, out_err: &mut FfiError) {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        nordocs_core::authoring::ndoc::create_document(Path::new(&path))
    });
    settle_unit(result, out_err);
}

/// Append an entry (`kind` is `"component"` or `"template"`) to an entry-format
/// document.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_add_entry(
    path: AsciiPointer,
    name: AsciiPointer,
    kind: AsciiPointer,
    content: AsciiPointer,
    out_err: &mut FfiError,
) {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        let name = arg_str(name)?;
        let kind = nordocs_core::fatfile::ndoc::EntryKind::from_str(&arg_str(kind)?)?;
        let content = arg_str(content)?;
        nordocs_core::authoring::ndoc::add_entry(Path::new(&path), &name, kind, &content)
    });
    settle_unit(result, out_err);
}

/// Replace the content of the named entry in an entry-format document.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_edit_entry(
    path: AsciiPointer,
    name: AsciiPointer,
    content: AsciiPointer,
    out_err: &mut FfiError,
) {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        let name = arg_str(name)?;
        let content = arg_str(content)?;
        nordocs_core::authoring::ndoc::edit_entry(Path::new(&path), &name, &content)
    });
    settle_unit(result, out_err);
}

/// Read the structured [`Document`] persisted in a four-section authoring file,
/// returned as JSON.
///
/// This is the read half of the `doc` node-tree round-trip: a host reads the
/// document, mutates the JSON tree (the `doc new`/`add`/`set`/`remove`
/// operations), and writes it back with [`ndoc_write_document`].
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_read_document(path: AsciiPointer, out_err: &mut FfiError) -> FfiString {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        let doc = nordocs_core::authoring::doc_state::read_document(Path::new(&path))?;
        Ok(serde_json::to_string(&doc)?)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Persist a structured [`Document`] (`doc_json`) into a four-section authoring
/// file, preserving its other sections.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_write_document(
    path: AsciiPointer,
    doc_json: AsciiPointer,
    out_err: &mut FfiError,
) {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        let doc: Document = serde_json::from_str(&arg_str(doc_json)?)?;
        nordocs_core::authoring::doc_state::write_document(Path::new(&path), &doc)
    });
    settle_unit(result, out_err);
}

/// Embed `image_bytes` (named `image_name`) into a four-section document.
///
/// Returns a JSON object `{ "status": "added" | "already_present", "hash": ... }`
/// describing the (idempotent) outcome.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_embed_image(
    path: AsciiPointer,
    image_name: AsciiPointer,
    image_bytes: FFISlice<u8>,
    out_err: &mut FfiError,
) -> FfiString {
    let result = run_guarded(move || {
        use nordocs_core::authoring::doc_state::ImageEmbed;
        let path = arg_str(path)?;
        let image_name = arg_str(image_name)?;
        let outcome = nordocs_core::authoring::doc_state::embed_image(
            Path::new(&path),
            &image_name,
            image_bytes.as_slice(),
        )?;
        let (status, hash) = match outcome {
            ImageEmbed::Added { hash } => ("added", hash),
            ImageEmbed::AlreadyPresent { hash } => ("already_present", hash),
        };
        Ok(serde_json::json!({ "status": status, "hash": hash }).to_string())
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Validate a `.ndoc.typ` or `.md` document, returning the result as JSON.
///
/// The JSON shape matches the CLI `--json` envelope's payload:
/// `{ "violations": [...], "summary": ... | null, "valid": bool }`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_validate_file(path: AsciiPointer, out_err: &mut FfiError) -> FfiString {
    let result = run_guarded(move || {
        use nordocs_core::validation::Severity;
        let path = arg_str(path)?;
        let path = Path::new(&path);
        let name = path.to_string_lossy();
        let validation = if name.ends_with(".ndoc.typ") {
            nordocs_core::validation::validate_ndoc_file(path)?
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            nordocs_core::validation::validate_markdown_file(path)?
        } else {
            return Err(nordocs_core::Error::Validation(format!(
                "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
                path.display()
            )));
        };
        let severity_label = |s: Severity| match s {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let violations: Vec<serde_json::Value> = validation
            .violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "severity": severity_label(v.severity),
                    "code": v.code,
                    "location": v.location,
                    "message": v.message,
                })
            })
            .collect();
        let summary = validation.summary.as_ref().map(|s| {
            serde_json::json!({
                "templateId": s.template_id,
                "templateVersion": s.template_version,
                "themeId": s.theme_id,
                "nodeCount": s.node_count,
                "globalInputCount": s.global_input_count,
            })
        });
        Ok(serde_json::json!({
            "violations": violations,
            "summary": summary,
            "valid": validation.is_valid(),
        })
        .to_string())
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Parse a `.ncmp.typ` component file, returning its [`ComponentSchema`] as JSON.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_component_schema(path: AsciiPointer, out_err: &mut FfiError) -> FfiString {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        let schema = nordocs_core::schema::parse::parse_component_file(Path::new(&path))?;
        Ok(serde_json::to_string(&schema)?)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Parse a `.ndoct.typ` template file, returning its `TemplateSchema` as JSON.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_template_schema(path: AsciiPointer, out_err: &mut FfiError) -> FfiString {
    let result = run_guarded(move || {
        let path = arg_str(path)?;
        let schema = nordocs_core::schema::parse::parse_template_file(Path::new(&path))?;
        Ok(serde_json::to_string(&schema)?)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Return the built-in component/template [`Catalogue`] as JSON.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_catalogue(out_err: &mut FfiError) -> FfiString {
    let result = run_guarded(move || {
        let catalogue = Catalogue::from_builtins();
        Ok(serde_json::to_string(&catalogue)?)
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

/// Load item collections from `dir`, returning a JSON summary.
///
/// The JSON shape matches the CLI `item load --json` payload:
/// `{ "collections": [ { "collection": name, "items": count }, ... ] }`.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_item_collections(dir: AsciiPointer, out_err: &mut FfiError) -> FfiString {
    let result = run_guarded(move || {
        let dir = arg_str(dir)?;
        let items = nordocs_core::item::load_items_from_dir(Path::new(&dir))?;
        let collections = nordocs_core::item::summarise_collections(&items);
        let entries: Vec<serde_json::Value> = collections
            .iter()
            .map(|(name, count)| serde_json::json!({ "collection": name, "items": count }))
            .collect();
        Ok(serde_json::json!({ "collections": entries }).to_string())
    });
    settle(
        result.map(FfiString::from_string),
        out_err,
        FfiString::empty(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ndoc_error_free, FfiErrorCode};
    use crate::marshal::{ndoc_byte_buffer_free, ndoc_string_free};
    use std::ffi::CString;

    fn ptr(s: &CString) -> AsciiPointer<'_> {
        AsciiPointer::from_slice_with_nul(s.as_bytes_with_nul()).expect("ascii")
    }

    fn read_string(s: &FfiString) -> String {
        // SAFETY: from_string stores valid UTF-8 of length `len`.
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(s.data, s.len as usize))
                .to_string()
        }
    }

    #[test]
    fn authoring_round_trip_writes_and_reads_a_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        let path_c = CString::new(path.to_string_lossy().as_bytes()).expect("path");

        // A minimal valid Document JSON.
        let doc_json = CString::new(
            serde_json::json!({
                "template": "article",
                "inputs": {},
                "nodes": [],
                "images": [],
            })
            .to_string(),
        )
        .expect("json");

        let mut err = FfiError::ok();
        ndoc_write_document(ptr(&path_c), ptr(&doc_json), &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok, "write must succeed");

        let mut err = FfiError::ok();
        let read = ndoc_read_document(ptr(&path_c), &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok, "read must succeed");
        let json = read_string(&read);
        assert!(
            json.contains("\"template\":\"article\""),
            "round-trip: {json}"
        );
        ndoc_string_free(read);
    }

    #[test]
    fn read_document_missing_file_reports_structured_error() {
        let path = CString::new("/nonexistent/path/doc.ndoc.typ").expect("path");
        let mut err = FfiError::ok();
        let result = ndoc_read_document(ptr(&path), &mut err);
        assert_eq!(err.code, FfiErrorCode::Io, "missing file is an IO error");
        assert!(result.data.is_null(), "failure yields the empty string");
        ndoc_string_free(result);
        ndoc_error_free(err);
    }

    #[test]
    fn validate_markdown_file_returns_structured_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "# Heading\n\nBody.").expect("write md");
        let path_c = CString::new(path.to_string_lossy().as_bytes()).expect("path");

        let mut err = FfiError::ok();
        let result = ndoc_validate_file(ptr(&path_c), &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok, "validation runs cleanly");
        let json = read_string(&result);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert!(
            value.get("valid").is_some(),
            "payload has a valid flag: {json}"
        );
        assert!(
            value.get("violations").is_some(),
            "payload lists violations"
        );
        ndoc_string_free(result);
    }

    #[test]
    fn validate_unsupported_format_reports_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.txt");
        std::fs::write(&path, "nope").expect("write");
        let path_c = CString::new(path.to_string_lossy().as_bytes()).expect("path");

        let mut err = FfiError::ok();
        let result = ndoc_validate_file(ptr(&path_c), &mut err);
        assert_eq!(err.code, FfiErrorCode::Validation);
        ndoc_string_free(result);
        ndoc_error_free(err);
    }

    #[test]
    fn catalogue_returns_builtin_schemas_as_json() {
        let mut err = FfiError::ok();
        let result = ndoc_catalogue(&mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);
        let json = read_string(&result);
        assert!(!json.is_empty(), "catalogue JSON must be non-empty");
        let _: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        ndoc_string_free(result);
    }

    #[test]
    fn embed_image_into_authoring_document_reports_added() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.ndoc.typ");
        let path_c = CString::new(path.to_string_lossy().as_bytes()).expect("path");
        let doc_json = CString::new(
            serde_json::json!({"template":"article","inputs":{},"nodes":[],"images":[]})
                .to_string(),
        )
        .expect("json");
        let mut err = FfiError::ok();
        ndoc_write_document(ptr(&path_c), ptr(&doc_json), &mut err);
        assert_eq!(err.code, FfiErrorCode::Ok);

        let name = CString::new("logo.png").expect("name");
        let bytes = b"PNGDATA";
        let mut err = FfiError::ok();
        let result = ndoc_embed_image(
            ptr(&path_c),
            ptr(&name),
            FFISlice::from_slice(bytes),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok, "embed must succeed");
        let json = read_string(&result);
        assert!(
            json.contains("\"status\":\"added\""),
            "embed outcome: {json}"
        );
        ndoc_string_free(result);
    }

    #[test]
    fn render_document_preview_compiles_to_pdf() {
        let state = CString::new(
            serde_json::json!({"template":"preview","inputs":{},"nodes":[],"images":[]})
                .to_string(),
        )
        .expect("state");
        let theme = CString::new("#set page(width: 90pt, height: 60pt)").expect("theme");
        let sources = CString::new("{}").expect("sources");
        let schemas = CString::new("{}").expect("schemas");
        let mut err = FfiError::ok();
        let buffer = ndoc_render_document_preview(
            ptr(&state),
            ptr(&theme),
            ptr(&sources),
            ptr(&schemas),
            &mut err,
        );
        assert_eq!(err.code, FfiErrorCode::Ok, "preview must compile");
        assert!(buffer.len >= 5);
        // SAFETY: reading the bytes we own.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) };
        assert_eq!(&bytes[..5], b"%PDF-");
        ndoc_byte_buffer_free(buffer);
    }
}
