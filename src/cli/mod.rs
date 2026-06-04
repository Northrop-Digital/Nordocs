//! CLI definition and dispatch for the `ndoc` binary.
//!
//! Uses clap's derive API. The command surface is the refined redesign from the
//! charter (dropping legacy cruft): a top-level set of operations plus the
//! `doc` authoring subgroup. Each command currently dispatches into a stub that
//! returns success; logic is filled in as the corresponding services are ported.

use clap::{Parser, Subcommand};

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
    /// Compose a fat file from a document + library.
    Build,
    /// Validate document structure and inputs.
    Validate,
    /// Render a quick preview before final build.
    Preview,
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

/// Arguments for `ndoc render`.
#[derive(Debug, clap::Args)]
pub struct RenderArgs {
    /// Input `.typ` or `.ndoc.typ` file.
    pub input: std::path::PathBuf,
    /// Output PDF path (defaults to input with a `.pdf` extension).
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
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

impl Cli {
    /// Dispatch the parsed command. Returns `anyhow::Result` so the binary can
    /// print a rich error chain.
    pub fn run(self) -> anyhow::Result<()> {
        match self.command {
            Command::Render(args) => cmd_render(args),
            Command::Build => stub("build"),
            Command::Validate => stub("validate"),
            Command::Preview => stub("preview"),
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

fn cmd_render(args: RenderArgs) -> anyhow::Result<()> {
    let _ = args;
    // TODO(port): read input, compose if needed, compile_to_pdf, write output.
    stub("render")
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
