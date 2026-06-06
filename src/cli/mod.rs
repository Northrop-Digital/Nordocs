//! CLI definition and dispatch for the `ndoc` binary.
//!
//! Uses clap's derive API. The command surface is the refined redesign from the
//! charter (dropping legacy cruft): a top-level set of operations plus the
//! `doc` authoring subgroup. Each command currently dispatches into a stub that
//! returns success; logic is filled in as the corresponding services are ported.

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
    /// Manage the component library (scaffolded, not yet implemented).
    Component,
    /// Manage reusable items/data collections (scaffolded, not yet implemented).
    Item,
    /// Manage document templates (scaffolded, not yet implemented).
    Template,
    /// Manage embedded images (scaffolded, not yet implemented).
    Image,
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
    /// Create a new document from a template.
    New,
    /// Print the document outline (node tree).
    Outline,
    /// Add a node.
    Add,
    /// Remove a node.
    Remove,
    /// Set a node input.
    Set,
    /// Show the input schema for a component/template.
    Schema,
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
            Command::Component => stub("component", json),
            Command::Item => stub("item", json),
            Command::Template => stub("template", json),
            Command::Image => stub("image", json),
        }
    }
}

/// Placeholder for a not-yet-ported command.
fn stub(name: &str, json: bool) -> anyhow::Result<()> {
    if json {
        output::emit_json_result(None);
    } else {
        println!("ndoc: '{name}' is scaffolded but not yet implemented");
    }
    Ok(())
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

fn cmd_render(args: RenderArgs, json: bool) -> anyhow::Result<()> {
    let _ = args;
    stub("render", json)
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
    let name = match args.command {
        DocCommand::New => "doc new",
        DocCommand::Outline => "doc outline",
        DocCommand::Add => "doc add",
        DocCommand::Remove => "doc remove",
        DocCommand::Set => "doc set",
        DocCommand::Schema => "doc schema",
    };
    if json {
        output::emit_json_result(None);
        Ok(())
    } else {
        stub(name, false)
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
