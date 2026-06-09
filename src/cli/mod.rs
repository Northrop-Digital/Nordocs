//! CLI definition and dispatch for the `ndoc` binary.
//!
//! Uses clap's derive API. The command surface is the refined redesign from the
//! charter (dropping legacy cruft): a top-level set of operations plus the
//! `doc` authoring subgroup. Each command dispatches into a `cmd_*` function
//! that performs the work and emits the shared JSON envelope under `--json`.

pub mod output;

use std::io::Read as _;

use anyhow::Context as _;
use clap::{Parser, Subcommand};

use crate::fatfile::ndoc::EntryKind;

/// northdoc — embed Typst, render Markdown/data to PDF, single binary.
#[derive(Debug, Parser)]
#[command(name = "ndoc", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Emit a JSON envelope to stdout instead of human-readable output.
    #[arg(long, global = true)]
    pub json: bool,
}

/// Top-level command groups for the refined v1 surface.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compile a `.typ` / fat file to PDF.
    Render(RenderArgs),
    /// Compile a Markdown or `.ndoc.typ` document to PDF.
    Build(BuildArgs),
    /// Create a new empty `.ndoc.typ` document.
    New(NewArgs),
    /// Add a named entry to an existing `.ndoc.typ` document.
    Add(AddArgs),
    /// Edit a named entry in an existing `.ndoc.typ` document.
    Edit(EditArgs),
    /// Validate document structure and inputs.
    Validate(ValidateArgs),
    /// Render a quick preview before final build.
    Preview(PreviewArgs),
    /// Document authoring operations (new, outline, add, set, patch, ...).
    Doc(DocArgs),
    /// Introspect and enumerate `.ncmp.typ` component files.
    Component(ComponentArgs),
    /// Load and validate reusable item collections.
    Item(ItemArgs),
    /// Inspect `.ndoct.typ` document templates.
    Template(TemplateArgs),
    /// Embed images into a `.ndoc.typ` document's image manifest.
    Image(ImageArgs),
}

/// Arguments for `ndoc build`.
#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Input Markdown or `.ndoc.typ` file to compile to PDF.
    pub input: std::path::PathBuf,
}

/// Arguments for `ndoc render`.
#[derive(Debug, clap::Args)]
pub struct RenderArgs {
    /// Input `.typ` or `.ndoc.typ` file.
    pub input: std::path::PathBuf,
    /// Output PDF path (defaults to input with a `.pdf` extension).
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
}

/// Arguments for `ndoc new`.
#[derive(Debug, clap::Args)]
pub struct NewArgs {
    /// Path to the new `.ndoc.typ` document to create.
    pub path: std::path::PathBuf,
}

/// Arguments for `ndoc add`.
#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// Path to the `.ndoc.typ` document.
    pub document: std::path::PathBuf,
    /// Unique name for the new entry.
    pub name: String,
    /// Entry kind for the new entry.
    #[arg(long, default_value = "component")]
    pub kind: EntryKind,
    /// Read entry content from this file instead of stdin.
    #[arg(long)]
    pub content_file: Option<std::path::PathBuf>,
}

/// Arguments for `ndoc edit`.
#[derive(Debug, clap::Args)]
pub struct EditArgs {
    /// Path to the `.ndoc.typ` document.
    pub document: std::path::PathBuf,
    /// Name of the entry to update.
    pub name: String,
    /// Read replacement content from this file instead of stdin.
    #[arg(long)]
    pub content_file: Option<std::path::PathBuf>,
}

/// Arguments for `ndoc validate`.
#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Input `.ndoc.typ` or `.md` file to validate.
    pub input: std::path::PathBuf,
}

/// Arguments for `ndoc preview`.
#[derive(Debug, clap::Args)]
pub struct PreviewArgs {
    /// Input `.ndoc.typ` or `.md` file to preview.
    pub input: std::path::PathBuf,
}

/// Arguments for the `ndoc doc` authoring subgroup.
#[derive(Debug, clap::Args)]
pub struct DocArgs {
    #[command(subcommand)]
    pub command: DocCommand,
}

/// Authoring subcommands under `ndoc doc`.
#[derive(Debug, Subcommand)]
pub enum DocCommand {
    /// Create a new document bound to a template.
    New(DocNewArgs),
    /// Print the document outline (node tree).
    Outline(DocOutlineArgs),
    /// Add a node.
    Add(DocAddArgs),
    /// Remove a node.
    Remove(DocRemoveArgs),
    /// Set a node input (or a document-level input).
    Set(DocSetArgs),
    /// Show the input schema for a component/template.
    Schema(DocSchemaArgs),
}

/// Arguments for `ndoc doc new`.
#[derive(Debug, clap::Args)]
pub struct DocNewArgs {
    /// Template id (resolved to `{id}.ndoct.typ`) or a path to a `.ndoct.typ`
    /// template file. The created document is bound to this template.
    pub template: String,
    /// Output path for the new `.ndoc.typ` document. Defaults to
    /// `{template-id}.ndoc.typ` in the current directory when omitted.
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
}

/// Arguments for `ndoc doc outline`.
#[derive(Debug, clap::Args)]
pub struct DocOutlineArgs {
    /// Path to the four-section `.ndoc.typ` document to outline.
    pub document: std::path::PathBuf,
}

/// Arguments for `ndoc doc add`.
///
/// `--parent`, `--before`, and `--after` are mutually exclusive placements; when
/// none is given the node is appended at the document root.
#[derive(Debug, clap::Args)]
pub struct DocAddArgs {
    /// Path to the four-section `.ndoc.typ` document to mutate.
    pub document: std::path::PathBuf,
    /// Component type the new node invokes (e.g. `heading`, `paragraph`).
    #[arg(long = "type", value_name = "COMPONENT")]
    pub node_type: String,
    /// Insert the new node as the last child of this existing node id.
    #[arg(long, group = "placement")]
    pub parent: Option<String>,
    /// Insert the new node as the sibling immediately before this node id.
    #[arg(long, group = "placement")]
    pub before: Option<String>,
    /// Insert the new node as the sibling immediately after this node id.
    #[arg(long, group = "placement")]
    pub after: Option<String>,
    /// Seed an input as `key=value` (repeatable). Values are stored as strings;
    /// schema-aware typing is applied later by `doc set`.
    #[arg(long = "inputs", value_name = "KEY=VALUE")]
    pub inputs: Vec<String>,
}

/// Arguments for `ndoc doc remove`.
#[derive(Debug, clap::Args)]
pub struct DocRemoveArgs {
    /// Path to the four-section `.ndoc.typ` document to mutate.
    pub document: std::path::PathBuf,
    /// Stable id of the node to remove.
    pub node_id: String,
    /// Remove the node's descendants too. Without this flag the node's children
    /// are preserved, promoted into the removed node's position.
    #[arg(long)]
    pub with_children: bool,
}

/// Arguments for `ndoc doc set`.
///
/// Targets either a node input (`<node_id> --key --value`) or a document-level
/// input (`--document --key --value`). Exactly one target must be chosen: a node
/// id positional is mutually exclusive with `--document`.
#[derive(Debug, clap::Args)]
pub struct DocSetArgs {
    /// Path to the four-section `.ndoc.typ` document to mutate.
    pub document: std::path::PathBuf,
    /// Stable id of the node whose input to set. Omit when targeting a
    /// document-level input with `--document`.
    pub node_id: Option<String>,
    /// Target a document-level input instead of a node input.
    #[arg(long = "document")]
    pub document_level: bool,
    /// Name of the input to set.
    #[arg(long)]
    pub key: String,
    /// New value, validated and coerced against the input's declared kind.
    #[arg(long)]
    pub value: String,
}

/// Arguments for `ndoc doc schema`.
#[derive(Debug, clap::Args)]
pub struct DocSchemaArgs {
    /// Path to a `.ncmp.typ` component or `.ndoct.typ` template file whose
    /// declared inputs to report.
    pub target: std::path::PathBuf,
}

/// Arguments for the `ndoc component` introspection subgroup.
#[derive(Debug, clap::Args)]
pub struct ComponentArgs {
    #[command(subcommand)]
    pub command: ComponentCommand,
}

/// Introspection subcommands under `ndoc component`.
#[derive(Debug, Subcommand)]
pub enum ComponentCommand {
    /// Show a single component file's declared input schema.
    Schema {
        /// Path to a `.ncmp.typ` component file.
        file: std::path::PathBuf,
    },
    /// List every component (`*.ncmp.typ`) found in a directory.
    List {
        /// Directory to enumerate component files in.
        dir: std::path::PathBuf,
    },
}

/// Arguments for the `ndoc item` collection subgroup.
#[derive(Debug, clap::Args)]
pub struct ItemArgs {
    #[command(subcommand)]
    pub command: ItemCommand,
}

/// Subcommands under `ndoc item`.
#[derive(Debug, Subcommand)]
pub enum ItemCommand {
    /// Discover item collections in a directory and summarise them.
    Load {
        /// Directory to discover `*.item.md` files in.
        dir: std::path::PathBuf,
    },
    /// Validate items in a directory against their sibling component schemas.
    Validate {
        /// Directory containing items and their `*.ncmp.typ` schemas.
        dir: std::path::PathBuf,
    },
}

/// Arguments for the `ndoc template` introspection subgroup.
#[derive(Debug, clap::Args)]
pub struct TemplateArgs {
    #[command(subcommand)]
    pub command: TemplateCommand,
}

/// Introspection subcommands under `ndoc template`.
#[derive(Debug, Subcommand)]
pub enum TemplateCommand {
    /// Show a template's document inputs and permitted components.
    Show {
        /// Template id (resolved to `{id}.ndoct.typ`) or a path to a
        /// `.ndoct.typ` file.
        target: String,
    },
}

/// Arguments for the `ndoc image` embedding subgroup.
#[derive(Debug, clap::Args)]
pub struct ImageArgs {
    #[command(subcommand)]
    pub command: ImageCommand,
}

/// Subcommands under `ndoc image`.
#[derive(Debug, Subcommand)]
pub enum ImageCommand {
    /// Embed an image into a `.ndoc.typ` document.
    Add {
        /// Target four-section `.ndoc.typ` document to embed into.
        file: std::path::PathBuf,
        /// Path to the image file to embed.
        image: std::path::PathBuf,
    },
}

/// Allow clap to parse [`EntryKind`] as a CLI value.
impl clap::ValueEnum for EntryKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Component, Self::Template]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Component => clap::builder::PossibleValue::new("component"),
            Self::Template => clap::builder::PossibleValue::new("template"),
        })
    }
}

impl Cli {
    /// Dispatch the parsed command. Returns `anyhow::Result` so the binary can
    /// print a rich error chain.
    pub fn run(self) -> anyhow::Result<()> {
        let json = self.json;
        match self.command {
            Command::Render(args) => cmd_render(args, json),
            Command::Build(args) => cmd_build(args, json),
            Command::New(args) => cmd_new(args, json),
            Command::Add(args) => cmd_add(args, json),
            Command::Edit(args) => cmd_edit(args, json),
            Command::Validate(args) => cmd_validate(args, json),
            Command::Preview(args) => cmd_preview(args, json),
            Command::Doc(args) => cmd_doc(args, json),
            Command::Component(args) => cmd_component(args, json),
            Command::Item(args) => cmd_item(args, json),
            Command::Template(args) => cmd_template(args, json),
            Command::Image(args) => cmd_image(args, json),
        }
    }
}

fn cmd_build(args: BuildArgs, json: bool) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    let output_path = if name.ends_with(".ndoc.typ") {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        let doc = crate::fatfile::ndoc::NdocDocument::parse(&src)
            .with_context(|| format!("failed to parse document '{}'", path.display()))?;
        let typst_source = doc
            .entries
            .iter()
            .map(|e| e.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let pdf_bytes = crate::compiler::compile_to_pdf(&typst_source)
            .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?;
        let base = name.strip_suffix(".ndoc.typ").unwrap_or(name.as_ref());
        let out = std::path::PathBuf::from(format!("{base}.pdf"));
        std::fs::write(&out, &pdf_bytes)
            .with_context(|| format!("failed to write PDF to '{}'", out.display()))?;
        out
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        let markdown = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        let typst_source = crate::markdown::markdown_to_typst(&markdown)
            .with_context(|| format!("failed to convert '{}' to Typst", path.display()))?;
        let pdf_bytes = crate::compiler::compile_to_pdf(&typst_source)
            .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?;
        let out = path.with_extension("pdf");
        std::fs::write(&out, &pdf_bytes)
            .with_context(|| format!("failed to write PDF to '{}'", out.display()))?;
        out
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    };

    if json {
        output::emit_json_result(Some(
            serde_json::json!({"output": output_path.display().to_string()}),
        ));
    }

    Ok(())
}

fn cmd_new(args: NewArgs, json: bool) -> anyhow::Result<()> {
    crate::authoring::ndoc::create_document(&args.path)
        .with_context(|| format!("failed to create document at '{}'", args.path.display()))?;
    if json {
        output::emit_json_result(Some(
            serde_json::json!({"path": args.path.display().to_string()}),
        ));
    }
    Ok(())
}

fn cmd_add(args: AddArgs, json: bool) -> anyhow::Result<()> {
    let content = read_content(args.content_file.as_deref())?;
    let kind = args.kind;
    crate::authoring::ndoc::add_entry(&args.document, &args.name, kind, &content).with_context(
        || {
            format!(
                "failed to add entry '{}' to '{}'",
                args.name,
                args.document.display()
            )
        },
    )?;
    if json {
        output::emit_json_result(None);
    }
    Ok(())
}

fn cmd_edit(args: EditArgs, json: bool) -> anyhow::Result<()> {
    let content = read_content(args.content_file.as_deref())?;
    crate::authoring::ndoc::edit_entry(&args.document, &args.name, &content).with_context(
        || {
            format!(
                "failed to edit entry '{}' in '{}'",
                args.name,
                args.document.display()
            )
        },
    )?;
    if json {
        output::emit_json_result(None);
    }
    Ok(())
}

/// Validate a `.ndoc.typ` or `.md` file against the built-in schema catalogue.
///
/// Dispatches to the appropriate validator by file extension, prints every
/// violation to stdout as `{location}: {message}`, and exits with code 1 when
/// any violation is found. Unsupported extensions bail with an actionable
/// message before validation is attempted.
fn cmd_validate(args: ValidateArgs, json: bool) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    let result = if name.ends_with(".ndoc.typ") {
        crate::validation::validate_ndoc_file(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        crate::validation::validate_markdown_file(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    };

    if json {
        let violations: Vec<serde_json::Value> = result
            .violations
            .iter()
            .map(|v| serde_json::json!({"location": v.location, "message": v.message}))
            .collect();
        output::emit_json_result(Some(serde_json::json!({"violations": violations})));
        if !result.is_valid() {
            std::process::exit(1);
        }
    } else {
        for v in &result.violations {
            println!("{}: {}", v.location, v.message);
        }
        if !result.is_valid() {
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Input suffixes `render` accepts, longest-first so detection is unambiguous.
///
/// `render` deliberately rejects a bare `.typ`: it only compiles the redesign's
/// authored file shapes. A `.ndoc.typ` four-section authoring file is already a
/// composed fat file (its `// === STATE ===` block is inert Typst), so all three
/// suffixes compile their raw contents through the embedded compiler.
const RENDER_SUFFIXES: [&str; 3] = [".ndoct.typ", ".ncmp.typ", ".ndoc.typ"];

/// Compile a single component/template/authoring file to PDF.
///
/// Accepts only `.ncmp.typ`, `.ndoct.typ`, and `.ndoc.typ` (never a bare
/// `.typ`). The default output is the input path with the recognised suffix
/// replaced by `.pdf`; `-o` overrides it. This stays distinct from `build`,
/// which owns `.md` and entry-format `.ndoc.typ` files.
fn cmd_render(args: RenderArgs, json: bool) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    let base = RENDER_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "unsupported input '{}': render accepts only .ncmp.typ, .ndoct.typ, or .ndoc.typ files",
                path.display()
            )
        })?;

    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    let pdf_bytes = crate::compiler::compile_to_pdf(&source)
        .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?;

    let output_path = args
        .output
        .unwrap_or_else(|| std::path::PathBuf::from(format!("{base}.pdf")));
    std::fs::write(&output_path, &pdf_bytes)
        .with_context(|| format!("failed to write PDF to '{}'", output_path.display()))?;

    if json {
        output::emit_json_result(Some(
            serde_json::json!({"output": output_path.display().to_string()}),
        ));
    }

    Ok(())
}

fn cmd_preview(args: PreviewArgs, json: bool) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    let typst_source = if name.ends_with(".ndoc.typ") {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        let doc = crate::fatfile::ndoc::NdocDocument::parse(&src)
            .with_context(|| format!("failed to parse document '{}'", path.display()))?;
        doc.entries
            .iter()
            .map(|e| e.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        let markdown = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        crate::markdown::markdown_to_typst(&markdown)
            .with_context(|| format!("failed to convert '{}' to Typst", path.display()))?
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    };

    let pdf_bytes = crate::compiler::compile_to_pdf(&typst_source)
        .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?;

    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let temp_pdf = std::env::temp_dir().join(format!("{stem}.pdf"));
    std::fs::write(&temp_pdf, &pdf_bytes)
        .with_context(|| format!("failed to write preview PDF to '{}'", temp_pdf.display()))?;

    if json {
        anyhow::ensure!(
            !pdf_bytes.is_empty(),
            "compiled PDF is unexpectedly empty for '{}'",
            path.display()
        );
        output::emit_json_result(Some(
            serde_json::json!({"preview_path": temp_pdf.display().to_string()}),
        ));
        return Ok(());
    }

    if std::env::var("NDOC_NO_OPEN").as_deref() == Ok("1") {
        anyhow::ensure!(
            !pdf_bytes.is_empty(),
            "compiled PDF is unexpectedly empty for '{}'",
            path.display()
        );
        return Ok(());
    }

    open_with_default_viewer(&temp_pdf)
}

/// Open `path` in the OS default viewer and return immediately.
///
/// Spawns the platform viewer without blocking: `open` on macOS,
/// `xdg-open` on Linux, `cmd /C start` on Windows. The viewer manages its
/// own lifecycle; callers must not delete `path` before the viewer opens it.
#[cfg(target_os = "macos")]
fn open_with_default_viewer(path: &std::path::Path) -> anyhow::Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .with_context(|| format!("failed to open '{}' in system viewer", path.display()))
}

#[cfg(target_os = "linux")]
fn open_with_default_viewer(path: &std::path::Path) -> anyhow::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .with_context(|| format!("failed to open '{}' in system viewer", path.display()))
}

#[cfg(target_os = "windows")]
fn open_with_default_viewer(path: &std::path::Path) -> anyhow::Result<()> {
    let path_str = path.to_string_lossy().into_owned();
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &path_str])
        .spawn()
        .map(|_| ())
        .with_context(|| format!("failed to open '{}' in system viewer", path.display()))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn open_with_default_viewer(path: &std::path::Path) -> anyhow::Result<()> {
    anyhow::bail!(
        "system viewer not supported on this platform; preview PDF is at '{}'",
        path.display()
    )
}

fn cmd_doc(args: DocArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        DocCommand::New(new_args) => cmd_doc_new(new_args, json),
        DocCommand::Outline(outline_args) => cmd_doc_outline(outline_args, json),
        DocCommand::Add(add_args) => cmd_doc_add(add_args, json),
        DocCommand::Remove(remove_args) => cmd_doc_remove(remove_args, json),
        DocCommand::Set(set_args) => cmd_doc_set(set_args, json),
        DocCommand::Schema(schema_args) => cmd_doc_schema(schema_args, json),
    }
}

/// Create a new `.ndoc.typ` document bound to a template.
///
/// `new <template> [-o]` resolves the template argument — a path used as-is, or
/// a bare id resolved to `{id}.ndoct.typ` — parses it to confirm it exists and
/// is well-formed, then writes an empty document bound to the template's id. The
/// output path defaults to `{template-id}.ndoc.typ` when `-o` is omitted, and
/// the command refuses to overwrite an existing path (exits non-zero), matching
/// the top-level `new` behaviour. `--json` reports the created path.
fn cmd_doc_new(args: DocNewArgs, json: bool) -> anyhow::Result<()> {
    let template_path = resolve_template_path(&args.template);
    let schema = crate::schema::parse::parse_template_file(&template_path)
        .with_context(|| format!("failed to read template '{}'", template_path.display()))?;

    let output = args
        .output
        .unwrap_or_else(|| std::path::PathBuf::from(format!("{}{DOCUMENT_SUFFIX}", schema.name)));
    anyhow::ensure!(
        !output.exists(),
        "refusing to overwrite existing document '{}'",
        output.display()
    );

    let doc = crate::model::Document {
        template: schema.name,
        inputs: std::collections::BTreeMap::new(),
        nodes: Vec::new(),
        images: Vec::new(),
    };
    crate::authoring::doc_state::write_document(&output, &doc)
        .with_context(|| format!("failed to create document at '{}'", output.display()))?;

    if json {
        output::emit_json_result(Some(
            serde_json::json!({"path": output.display().to_string()}),
        ));
    } else {
        println!(
            "created '{}' (template: {})",
            output.display(),
            doc.template
        );
    }
    Ok(())
}

/// Print the node tree of a `.ndoc.typ` document.
///
/// Reads the document's persisted node tree and prints each node's stable id and
/// component type in document order, indenting two spaces per nesting level so
/// the tree shape is visible. `--json` emits a structured tree of
/// `{id, component, children}` under `data.nodes` (plus the bound `template`).
/// A missing or unparseable document bails with the offending path named.
fn cmd_doc_outline(args: DocOutlineArgs, json: bool) -> anyhow::Result<()> {
    let doc = crate::authoring::doc_state::read_document(&args.document)
        .with_context(|| format!("failed to read document '{}'", args.document.display()))?;

    if json {
        let nodes: Vec<serde_json::Value> = doc.nodes.iter().map(outline_node_json).collect();
        output::emit_json_result(Some(
            serde_json::json!({"template": doc.template, "nodes": nodes}),
        ));
    } else {
        println!("template: {}", doc.template);
        for node in &doc.nodes {
            print_outline_node(node, 0);
        }
    }
    Ok(())
}

/// Print one node and its descendants, indenting two spaces per `depth` level.
fn print_outline_node(node: &crate::model::Node, depth: usize) {
    println!("{}{} ({})", "  ".repeat(depth), node.id, node.component);
    for child in &node.children {
        print_outline_node(child, depth + 1);
    }
}

/// Build the structured outline JSON for one node: `{id, component, children}`.
///
/// Deliberately omits node inputs — outline answers "what nodes exist and how
/// are they nested" so callers can address them; input values are surfaced by
/// `doc set`/`doc schema` instead.
fn outline_node_json(node: &crate::model::Node) -> serde_json::Value {
    let children: Vec<serde_json::Value> = node.children.iter().map(outline_node_json).collect();
    serde_json::json!({
        "id": node.id.to_string(),
        "component": node.component,
        "children": children,
    })
}

/// Add a node to a `.ndoc.typ` document's node tree.
///
/// Validates the requested `--type` against the built-in component catalogue
/// (the same known set `validate` uses), mints a fresh stable id, seeds any
/// `--inputs`, and places the node relative to `--parent`/`--before`/`--after`
/// (root when none is given). The whole operation is read-mutate-write: the
/// single atomic [`write_document`](crate::authoring::doc_state::write_document)
/// happens only after validation and placement succeed, so an unknown type or an
/// unknown placement target leaves the document unchanged and exits non-zero.
/// `--json` reports the minted node id.
fn cmd_doc_add(args: DocAddArgs, json: bool) -> anyhow::Result<()> {
    let catalogue = crate::schema::Catalogue::from_builtins();
    anyhow::ensure!(
        catalogue.component(&args.node_type).is_some(),
        "unknown component type '{}'",
        args.node_type
    );

    let inputs = parse_seed_inputs(&args.inputs)?;

    let mut doc = crate::authoring::doc_state::read_document(&args.document)
        .with_context(|| format!("failed to read document '{}'", args.document.display()))?;

    let id = crate::model::NodeId::mint(&args.node_type, &doc.node_ids());
    let node = crate::model::Node {
        id: id.clone(),
        component: args.node_type.clone(),
        inputs,
        children: Vec::new(),
    };

    place_node(
        &mut doc.nodes,
        node,
        args.parent.as_deref(),
        args.before.as_deref(),
        args.after.as_deref(),
    )?;

    crate::authoring::doc_state::write_document(&args.document, &doc)
        .with_context(|| format!("failed to write document '{}'", args.document.display()))?;

    if json {
        output::emit_json_result(Some(serde_json::json!({"node_id": id.to_string()})));
    } else {
        println!("added {id} ({})", args.node_type);
    }
    Ok(())
}

/// Remove a node from a `.ndoc.typ` document's node tree.
///
/// Locates the node by id anywhere in the tree and removes it. Without
/// `--with-children` the node's children are preserved — promoted into the
/// removed node's position; with the flag the whole subtree is dropped. An
/// unknown id leaves the document unchanged and exits non-zero. The single
/// atomic write happens only after a successful removal.
fn cmd_doc_remove(args: DocRemoveArgs, json: bool) -> anyhow::Result<()> {
    let mut doc = crate::authoring::doc_state::read_document(&args.document)
        .with_context(|| format!("failed to read document '{}'", args.document.display()))?;

    let removed = remove_node(&mut doc.nodes, &args.node_id, args.with_children);
    anyhow::ensure!(removed, "unknown node id '{}'", args.node_id);

    crate::authoring::doc_state::write_document(&args.document, &doc)
        .with_context(|| format!("failed to write document '{}'", args.document.display()))?;

    if json {
        output::emit_json_result(Some(serde_json::json!({"removed": args.node_id})));
    } else {
        println!("removed {}", args.node_id);
    }
    Ok(())
}

/// Parse `key=value` seed inputs into string-kinded [`InputValue`]s.
///
/// Values are stored verbatim as `InputKind::String`; typed coercion against the
/// declared schema is `doc set`'s responsibility. A pair missing `=` is rejected.
fn parse_seed_inputs(
    pairs: &[String],
) -> anyhow::Result<std::collections::BTreeMap<String, crate::model::InputValue>> {
    let mut map = std::collections::BTreeMap::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .with_context(|| format!("invalid --inputs entry '{pair}': expected key=value"))?;
        anyhow::ensure!(
            !key.is_empty(),
            "invalid --inputs entry '{pair}': empty key"
        );
        map.insert(
            key.to_string(),
            crate::model::InputValue {
                kind: crate::model::InputKind::String,
                value: serde_json::Value::String(value.to_string()),
            },
        );
    }
    Ok(map)
}

/// Place `node` in `nodes` according to the requested placement.
///
/// Clap guarantees at most one of `parent`/`before`/`after` is set. `parent`
/// appends as the target's last child; `before`/`after` insert as a sibling
/// adjacent to the target; none appends at the root. An unknown placement target
/// is an error and leaves `nodes` unchanged (the recursive helpers report
/// whether they found the target before any caller mutates on its behalf).
fn place_node(
    nodes: &mut Vec<crate::model::Node>,
    node: crate::model::Node,
    parent: Option<&str>,
    before: Option<&str>,
    after: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(parent_id) = parent {
        anyhow::ensure!(
            insert_as_child(nodes, parent_id, node),
            "unknown parent node id '{parent_id}'"
        );
    } else if let Some(target_id) = before {
        anyhow::ensure!(
            insert_sibling(nodes, target_id, node, false),
            "unknown node id '{target_id}'"
        );
    } else if let Some(target_id) = after {
        anyhow::ensure!(
            insert_sibling(nodes, target_id, node, true),
            "unknown node id '{target_id}'"
        );
    } else {
        nodes.push(node);
    }
    Ok(())
}

/// Append `node` as the last child of the node with `parent_id`, searching the
/// whole tree. Returns whether the parent was found (and the insert performed).
fn insert_as_child(
    nodes: &mut [crate::model::Node],
    parent_id: &str,
    node: crate::model::Node,
) -> bool {
    for candidate in nodes.iter_mut() {
        if candidate.id.0 == parent_id {
            candidate.children.push(node);
            return true;
        }
    }
    // The target was not at this level; descend. A separate pass keeps the
    // borrow on each child's children scoped to one node at a time.
    for candidate in nodes.iter_mut() {
        if insert_as_child(&mut candidate.children, parent_id, node.clone()) {
            return true;
        }
    }
    false
}

/// Insert `node` as a sibling adjacent to `target_id` (after it when `after` is
/// true, otherwise before it), searching the whole tree. Returns whether the
/// target was found.
fn insert_sibling(
    nodes: &mut Vec<crate::model::Node>,
    target_id: &str,
    node: crate::model::Node,
    after: bool,
) -> bool {
    if let Some(idx) = nodes.iter().position(|n| n.id.0 == target_id) {
        nodes.insert(if after { idx + 1 } else { idx }, node);
        return true;
    }
    for candidate in nodes.iter_mut() {
        if insert_sibling(&mut candidate.children, target_id, node.clone(), after) {
            return true;
        }
    }
    false
}

/// Remove the node with `target_id` from the tree, returning whether it existed.
///
/// When `with_children` is false the removed node's children are spliced into
/// its former position (siblings shift right), preserving the subtree; when true
/// the whole subtree is dropped.
fn remove_node(nodes: &mut Vec<crate::model::Node>, target_id: &str, with_children: bool) -> bool {
    if let Some(idx) = nodes.iter().position(|n| n.id.0 == target_id) {
        let removed = nodes.remove(idx);
        if !with_children {
            for (offset, child) in removed.children.into_iter().enumerate() {
                nodes.insert(idx + offset, child);
            }
        }
        return true;
    }
    for candidate in nodes.iter_mut() {
        if remove_node(&mut candidate.children, target_id, with_children) {
            return true;
        }
    }
    false
}

/// Set an input value on a node (or a document-level input).
///
/// Resolves the target input's declared [`InputKind`](crate::model::InputKind)
/// from the built-in catalogue (the same schema source `doc add` validates
/// against): a node target looks up its `component`'s schema, a `--document`
/// target looks up the bound template's `document_inputs`. The supplied `--value`
/// is validated and coerced against that kind before anything is written, so an
/// unknown node id, an input key absent from the schema, or a value that does not
/// match the kind leaves the document unchanged and exits non-zero. The single
/// atomic write happens only after validation succeeds. `--json` reports the
/// node/document target, key, and stored value.
fn cmd_doc_set(args: DocSetArgs, json: bool) -> anyhow::Result<()> {
    anyhow::ensure!(
        args.document_level == args.node_id.is_none(),
        "specify exactly one target: a <node_id> or --document"
    );

    let catalogue = crate::schema::Catalogue::from_builtins();
    let mut doc = crate::authoring::doc_state::read_document(&args.document)
        .with_context(|| format!("failed to read document '{}'", args.document.display()))?;

    // Resolve the declared kind for the target input before mutating anything.
    let kind = if args.document_level {
        let template = catalogue
            .template(&doc.template)
            .with_context(|| format!("no schema for template '{}'", doc.template))?;
        declared_kind(&template.document_inputs, &args.key).with_context(|| {
            format!(
                "input '{}' is not in the schema for template '{}'",
                args.key, doc.template
            )
        })?
    } else {
        let node_id = args.node_id.as_deref().unwrap_or_default();
        let component = find_node(&doc.nodes, node_id)
            .with_context(|| format!("unknown node id '{node_id}'"))?
            .component
            .clone();
        let schema = catalogue
            .component(&component)
            .with_context(|| format!("no schema for component '{component}'"))?;
        declared_kind(&schema.inputs, &args.key).with_context(|| {
            format!(
                "input '{}' is not in the schema for component '{component}'",
                args.key
            )
        })?
    };

    let image_names: std::collections::HashSet<&str> =
        doc.images.iter().map(|i| i.name.as_str()).collect();
    let value = coerce_value(kind, &args.value, &image_names)?;
    let input = crate::model::InputValue { kind, value };

    if args.document_level {
        doc.inputs.insert(args.key.clone(), input);
    } else {
        let node_id = args.node_id.as_deref().unwrap_or_default();
        let node =
            find_node_mut(&mut doc.nodes, node_id).expect("node existed during kind resolution");
        node.inputs.insert(args.key.clone(), input);
    }

    crate::authoring::doc_state::write_document(&args.document, &doc)
        .with_context(|| format!("failed to write document '{}'", args.document.display()))?;

    if json {
        let target = match &args.node_id {
            Some(id) => serde_json::json!({"node": id}),
            None => serde_json::json!({"document": true}),
        };
        output::emit_json_result(Some(serde_json::json!({
            "target": target,
            "key": args.key,
            "value": args.value,
        })));
    } else {
        match &args.node_id {
            Some(id) => println!("set {}.{} = {}", id, args.key, args.value),
            None => println!("set document.{} = {}", args.key, args.value),
        }
    }
    Ok(())
}

/// Find the declared [`InputKind`](crate::model::InputKind) for `key` among
/// `inputs`, or `None` when no input declares that name.
fn declared_kind(
    inputs: &[crate::schema::InputSchema],
    key: &str,
) -> Option<crate::model::InputKind> {
    inputs.iter().find(|i| i.name == key).map(|i| i.kind)
}

/// Validate and coerce a raw string `value` into a JSON value matching `kind`.
///
/// Each kind has a concrete shape so a mismatch is a real, reportable error:
/// numbers must parse as a finite `f64`, booleans as `true`/`false`, colors as a
/// `#`-prefixed 3/6/8-digit hex string, and an image must name an entry already
/// embedded in the document (per the IMAGES manifest). `string` and `content`
/// accept any text verbatim.
fn coerce_value(
    kind: crate::model::InputKind,
    value: &str,
    image_names: &std::collections::HashSet<&str>,
) -> anyhow::Result<serde_json::Value> {
    use crate::model::InputKind;
    match kind {
        InputKind::String | InputKind::Content => Ok(serde_json::Value::String(value.to_string())),
        InputKind::Number => {
            let n: f64 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("value '{value}' is not a valid number"))?;
            let number = serde_json::Number::from_f64(n)
                .ok_or_else(|| anyhow::anyhow!("value '{value}' is not a finite number"))?;
            Ok(serde_json::Value::Number(number))
        }
        InputKind::Boolean => match value {
            "true" => Ok(serde_json::Value::Bool(true)),
            "false" => Ok(serde_json::Value::Bool(false)),
            other => anyhow::bail!("value '{other}' is not a boolean (expected true or false)"),
        },
        InputKind::Color => {
            anyhow::ensure!(
                is_hex_color(value),
                "value '{value}' is not a hex color (expected #rgb, #rrggbb, or #rrggbbaa)"
            );
            Ok(serde_json::Value::String(value.to_string()))
        }
        InputKind::Image => {
            anyhow::ensure!(
                image_names.contains(value),
                "image '{value}' is not embedded in the document (embed it with `ndoc image add` first)"
            );
            Ok(serde_json::Value::String(value.to_string()))
        }
    }
}

/// Whether `s` is a `#`-prefixed hex color of 3, 6, or 8 digits.
fn is_hex_color(s: &str) -> bool {
    let Some(digits) = s.strip_prefix('#') else {
        return false;
    };
    matches!(digits.len(), 3 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit())
}

/// Find the node with `id` anywhere in the tree, in document order.
fn find_node<'a>(nodes: &'a [crate::model::Node], id: &str) -> Option<&'a crate::model::Node> {
    for node in nodes {
        if node.id.0 == id {
            return Some(node);
        }
        if let Some(found) = find_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

/// Find the node with `id` anywhere in the tree, mutably.
fn find_node_mut<'a>(
    nodes: &'a mut [crate::model::Node],
    id: &str,
) -> Option<&'a mut crate::model::Node> {
    for node in nodes.iter_mut() {
        if node.id.0 == id {
            return Some(node);
        }
        if let Some(found) = find_node_mut(&mut node.children, id) {
            return Some(found);
        }
    }
    None
}

/// Show the declared input schema for a component or template file.
///
/// `schema <target>` dispatches by suffix: a `.ncmp.typ` file reports its
/// component inputs, a `.ndoct.typ` file reports its template document inputs;
/// any other suffix is an actionable error. Each input is printed as
/// `name: kind (required|optional)`. `--json` emits the full schema under
/// `data`. An unreadable or unparseable file bails with the offending path named.
fn cmd_doc_schema(args: DocSchemaArgs, json: bool) -> anyhow::Result<()> {
    let name = args.target.to_string_lossy();
    if name.ends_with(COMPONENT_SUFFIX) {
        let schema = crate::schema::parse::parse_component_file(&args.target)
            .with_context(|| format!("failed to read component '{}'", args.target.display()))?;
        if json {
            output::emit_json_result(Some(serde_json::json!({"component": schema})));
        } else {
            println!("component: {}", schema.name);
            print_inputs(&schema.inputs);
        }
    } else if name.ends_with(TEMPLATE_SUFFIX) {
        let schema = crate::schema::parse::parse_template_file(&args.target)
            .with_context(|| format!("failed to read template '{}'", args.target.display()))?;
        if json {
            output::emit_json_result(Some(serde_json::json!({"template": schema})));
        } else {
            println!("template: {}", schema.name);
            print_inputs(&schema.document_inputs);
        }
    } else {
        anyhow::bail!(
            "unsupported target '{}': doc schema accepts a .ncmp.typ or .ndoct.typ file",
            args.target.display()
        );
    }
    Ok(())
}

/// Print declared inputs as `name: kind (required|optional)`, one per line.
fn print_inputs(inputs: &[crate::schema::InputSchema]) {
    for input in inputs {
        println!(
            "  {}: {} ({})",
            input.name,
            input_kind_str(input.kind),
            if input.required {
                "required"
            } else {
                "optional"
            }
        );
    }
}

/// Suffix identifying a component file.
const COMPONENT_SUFFIX: &str = ".ncmp.typ";

/// Introspect (`schema`) or enumerate (`list`) `.ncmp.typ` component files.
///
/// `schema <file>` parses one component and reports each input's name, kind, and
/// required flag. `list <dir>` enumerates a directory's components in stable
/// order. Both honour `--json`: `schema` emits the full [`ComponentSchema`] under
/// `data`, `list` emits an array of `{name, inputs}` entries. A missing or
/// unparseable file, or a missing directory, bails with the offending path named;
/// an empty directory reports zero components and exits 0.
fn cmd_component(args: ComponentArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        ComponentCommand::Schema { file } => {
            let schema = crate::schema::parse::parse_component_file(&file)
                .with_context(|| format!("failed to read component '{}'", file.display()))?;
            if json {
                output::emit_json_result(Some(serde_json::json!({"component": schema})));
            } else {
                println!("component: {}", schema.name);
                for input in &schema.inputs {
                    println!(
                        "  {}: {} ({})",
                        input.name,
                        input_kind_str(input.kind),
                        if input.required {
                            "required"
                        } else {
                            "optional"
                        }
                    );
                }
            }
        }
        ComponentCommand::List { dir } => {
            let schemas = crate::schema::parse::load_components_from_dir(&dir)
                .with_context(|| format!("failed to list components in '{}'", dir.display()))?;
            if json {
                let entries: Vec<serde_json::Value> = schemas
                    .iter()
                    .map(|s| serde_json::json!({"name": s.name, "inputs": s.inputs.len()}))
                    .collect();
                output::emit_json_result(Some(serde_json::json!({"components": entries})));
            } else if schemas.is_empty() {
                println!("(no components found)");
            } else {
                for schema in &schemas {
                    println!("{} ({} inputs)", schema.name, schema.inputs.len());
                }
            }
        }
    }
    Ok(())
}

/// Load (`load`) or validate (`validate`) reusable item collections.
///
/// `load <dir>` discovers `*.item.md` files, summarises the collections found
/// (`{collection}: {count}`), and exits 0. `validate <dir>` additionally checks
/// each item against the sibling `*.ncmp.typ` component schemas in the same
/// directory, printing every issue as `{source}: [{code}] {message}` and exiting
/// non-zero when any issue is found. Both honour `--json`: `load` emits a
/// `collections` array, `validate` emits a `valid` flag plus an `issues` array.
/// A missing or unreadable directory bails with the offending path named.
fn cmd_item(args: ItemArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        ItemCommand::Load { dir } => {
            let items = crate::item::load_items_from_dir(&dir)
                .with_context(|| format!("failed to load items in '{}'", dir.display()))?;
            let collections = crate::item::summarise_collections(&items);
            if json {
                let entries: Vec<serde_json::Value> = collections
                    .iter()
                    .map(|(name, count)| serde_json::json!({"collection": name, "items": count}))
                    .collect();
                output::emit_json_result(Some(serde_json::json!({"collections": entries})));
            } else if collections.is_empty() {
                println!("(no item collections found)");
            } else {
                for (name, count) in &collections {
                    println!("{name}: {count} items");
                }
            }
        }
        ItemCommand::Validate { dir } => {
            let items = crate::item::load_items_from_dir(&dir)
                .with_context(|| format!("failed to load items in '{}'", dir.display()))?;
            let components = crate::schema::parse::load_components_from_dir(&dir)
                .with_context(|| format!("failed to load schemas in '{}'", dir.display()))?;
            let issues = crate::item::validate_items(&items, &components);

            if json {
                let entries: Vec<serde_json::Value> = issues
                    .iter()
                    .map(|i| {
                        serde_json::json!({
                            "source": i.source_path.display().to_string(),
                            "code": i.code,
                            "message": i.message,
                        })
                    })
                    .collect();
                output::emit_json_result(Some(
                    serde_json::json!({"valid": issues.is_empty(), "issues": entries}),
                ));
            } else {
                for issue in &issues {
                    println!(
                        "{}: [{}] {}",
                        issue.source_path.display(),
                        issue.code,
                        issue.message
                    );
                }
            }
            if !issues.is_empty() {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

/// Render an [`InputKind`](crate::model::InputKind) as its lowercase wire string
/// (the same token used in the file frontmatter `type:` key).
fn input_kind_str(kind: crate::model::InputKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{kind:?}"))
}

/// Suffix identifying a template file.
const TEMPLATE_SUFFIX: &str = ".ndoct.typ";

/// Inspect (`show`) a `.ndoct.typ` document template.
///
/// `show <id|path>` resolves its argument to a template file — a path used
/// as-is, or a bare id resolved to `{id}.ndoct.typ` — then reports the
/// template's document inputs (name, kind, required) and the components it
/// permits. `--json` emits the full [`TemplateSchema`] under `data.template`.
/// An unknown id or unreadable/unparseable file bails with the offending path
/// named (the typed `Error::Schema` from the parser carries the path).
fn cmd_template(args: TemplateArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        TemplateCommand::Show { target } => {
            let path = resolve_template_path(&target);
            let schema = crate::schema::parse::parse_template_file(&path)
                .with_context(|| format!("failed to read template '{}'", path.display()))?;
            if json {
                output::emit_json_result(Some(serde_json::json!({"template": schema})));
            } else {
                println!("template: {}", schema.name);
                println!("document inputs:");
                if schema.document_inputs.is_empty() {
                    println!("  (none)");
                } else {
                    for input in &schema.document_inputs {
                        println!(
                            "  {}: {} ({})",
                            input.name,
                            input_kind_str(input.kind),
                            if input.required {
                                "required"
                            } else {
                                "optional"
                            }
                        );
                    }
                }
                println!("permitted components:");
                if schema.allowed_components.is_empty() {
                    println!("  (all)");
                } else {
                    for name in &schema.allowed_components {
                        println!("  {name}");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Suffix identifying the four-section authoring document `image add` targets.
const DOCUMENT_SUFFIX: &str = ".ndoc.typ";

/// Embed (`add`) an image into a `.ndoc.typ` document's image manifest.
///
/// `add <file> <image>` reads the image bytes, records a `{name, hash}` entry in
/// the document's STATE-section manifest, and embeds the base64 bytes in the
/// IMAGES section (deduped by blake3 hash). Embedding identical content under
/// the same name again is an idempotent no-op. `--json` reports the image name,
/// hash, and whether it was newly added. A non-`.ndoc.typ` target, a missing
/// target/image, or a name already bound to different content exits non-zero
/// with the offending detail named.
fn cmd_image(args: ImageArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        ImageCommand::Add { file, image } => {
            let name = file.to_string_lossy();
            anyhow::ensure!(
                name.ends_with(DOCUMENT_SUFFIX),
                "unsupported target '{}': image add accepts only .ndoc.typ documents",
                file.display()
            );

            let bytes = std::fs::read(&image)
                .with_context(|| format!("failed to read image '{}'", image.display()))?;
            let image_name = image
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| {
                    anyhow::anyhow!("image path '{}' has no file name", image.display())
                })?;

            let outcome = crate::authoring::doc_state::embed_image(&file, &image_name, &bytes)
                .with_context(|| {
                    format!(
                        "failed to embed image '{}' into '{}'",
                        image.display(),
                        file.display()
                    )
                })?;

            let hash = blake3::hash(&bytes).to_hex().to_string();
            let added = matches!(outcome, crate::authoring::doc_state::ImageEmbed::Added);

            if json {
                output::emit_json_result(Some(serde_json::json!({
                    "name": image_name,
                    "hash": hash,
                    "added": added,
                })));
            } else if added {
                println!("embedded image '{image_name}' ({})", &hash[..12]);
            } else {
                println!("image '{image_name}' already embedded ({})", &hash[..12]);
            }
        }
    }
    Ok(())
}

/// Resolve a `show` argument to a template file path.
///
/// A `target` that already ends with `.ndoct.typ` is used verbatim; a bare id
/// (e.g. `fee-proposal`) resolves to `{id}.ndoct.typ` relative to the current
/// directory. Either way the path is handed to the parser, which names it in
/// any failure.
fn resolve_template_path(target: &str) -> std::path::PathBuf {
    if target.ends_with(TEMPLATE_SUFFIX) {
        std::path::PathBuf::from(target)
    } else {
        std::path::PathBuf::from(format!("{target}{TEMPLATE_SUFFIX}"))
    }
}

/// Read content from `content_file` if supplied, otherwise read from stdin.
fn read_content(content_file: Option<&std::path::Path>) -> anyhow::Result<String> {
    match content_file {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read content from '{}'", path.display())),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("failed to read content from stdin")?;
            Ok(buf)
        }
    }
}
