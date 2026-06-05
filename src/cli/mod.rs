//! CLI definition and dispatch for the `ndoc` binary.
//!
//! Uses clap's derive API. The command surface is the refined redesign from the
//! charter (dropping legacy cruft): a top-level set of operations plus the
//! `doc` authoring subgroup. Each command currently dispatches into a stub that
//! returns success; logic is filled in as the corresponding services are ported.

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
    /// Manage the component library.
    Component,
    /// Manage reusable items/data collections.
    Item,
    /// Manage document templates.
    Template,
    /// Manage embedded images.
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
    /// Entry kind: `component` (default) or `template`.
    #[arg(long)]
    pub kind: Option<EntryKind>,
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
    /// Emit a `{ok,data?,error?}` JSON envelope instead of human output.
    #[arg(long, global = true)]
    pub json: bool,
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
        match self.command {
            Command::Render(args) => cmd_render(args),
            Command::Build(args) => cmd_build(args),
            Command::New(args) => cmd_new(args),
            Command::Add(args) => cmd_add(args),
            Command::Edit(args) => cmd_edit(args),
            Command::Validate(args) => cmd_validate(args),
            Command::Preview(args) => cmd_preview(args),
            Command::Doc(args) => cmd_doc(args),
            Command::Component => stub("component"),
            Command::Item => stub("item"),
            Command::Template => stub("template"),
            Command::Image => stub("image"),
        }
    }
}

/// Placeholder for a not-yet-ported command.
fn stub(name: &str) -> anyhow::Result<()> {
    println!("ndoc: '{name}' is scaffolded but not yet implemented");
    Ok(())
}

fn cmd_build(args: BuildArgs) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    if name.ends_with(".ndoc.typ") {
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
        let output = std::path::PathBuf::from(format!("{base}.pdf"));
        std::fs::write(&output, &pdf_bytes)
            .with_context(|| format!("failed to write PDF to '{}'", output.display()))?;
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        // Existing Markdown pipeline — unchanged.
        let markdown = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        let typst_source = crate::markdown::markdown_to_typst(&markdown)
            .with_context(|| format!("failed to convert '{}' to Typst", path.display()))?;
        let pdf_bytes = crate::compiler::compile_to_pdf(&typst_source)
            .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?;
        let output = path.with_extension("pdf");
        std::fs::write(&output, &pdf_bytes)
            .with_context(|| format!("failed to write PDF to '{}'", output.display()))?;
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    }

    Ok(())
}

fn cmd_new(args: NewArgs) -> anyhow::Result<()> {
    crate::authoring::ndoc::create_document(&args.path)
        .with_context(|| format!("failed to create document at '{}'", args.path.display()))?;
    Ok(())
}

fn cmd_add(args: AddArgs) -> anyhow::Result<()> {
    let content = read_content(args.content_file.as_deref())?;
    let kind = args.kind.unwrap_or(EntryKind::Component);
    crate::authoring::ndoc::add_entry(&args.document, &args.name, kind, &content).with_context(
        || {
            format!(
                "failed to add entry '{}' to '{}'",
                args.name,
                args.document.display()
            )
        },
    )?;
    Ok(())
}

fn cmd_edit(args: EditArgs) -> anyhow::Result<()> {
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
    Ok(())
}

/// Validate a `.ndoc.typ` or `.md` file against the built-in schema catalogue.
///
/// Dispatches to the appropriate validator by file extension, prints every
/// violation to stdout as `{location}: {message}`, and exits with code 1 when
/// any violation is found. Unsupported extensions bail with an actionable
/// message before validation is attempted.
fn cmd_validate(args: ValidateArgs) -> anyhow::Result<()> {
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

    for v in &result.violations {
        println!("{}: {}", v.location, v.message);
    }

    if !result.is_valid() {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_render(args: RenderArgs) -> anyhow::Result<()> {
    let _ = args;
    // TODO(port): read input, compose if needed, compile_to_pdf, write output.
    stub("render")
}

fn cmd_preview(args: PreviewArgs) -> anyhow::Result<()> {
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

    // Write to OS temp dir — ndoc does not delete before the viewer opens.
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let temp_pdf = std::env::temp_dir().join(format!("{stem}.pdf"));
    std::fs::write(&temp_pdf, &pdf_bytes)
        .with_context(|| format!("failed to write preview PDF to '{}'", temp_pdf.display()))?;

    // When NDOC_NO_OPEN=1 (e.g. headless CI), skip the viewer but verify a
    // non-empty PDF was produced so the render path is still exercised.
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

fn cmd_doc(args: DocArgs) -> anyhow::Result<()> {
    let name = match args.command {
        DocCommand::New => "doc new",
        DocCommand::Outline => "doc outline",
        DocCommand::Add => "doc add",
        DocCommand::Remove => "doc remove",
        DocCommand::Set => "doc set",
        DocCommand::Schema => "doc schema",
    };
    if args.json {
        println!("{{\"ok\":true,\"data\":null}}");
        Ok(())
    } else {
        stub(name)
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
