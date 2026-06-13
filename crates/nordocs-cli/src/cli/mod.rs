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

use nordocs_core::fatfile::ndoc::EntryKind;

/// nordocs — embed Typst, render Markdown/data to PDF, single binary.
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
    /// Diagnostic: map a page click back to its source location.
    ///
    /// Hidden from `ndoc --help` — this exercises the `source-mapping`
    /// primitive end-to-end for the test suite and standalone coordinate
    /// debugging. The intended consumer of click-to-source is the in-process
    /// FFI, not this command. `ndoc jump --help` still works.
    #[command(hide = true)]
    Jump(JumpArgs),
}

/// Arguments for the hidden `ndoc jump` diagnostic.
#[derive(Debug, clap::Args)]
pub struct JumpArgs {
    /// Input `.typ` / `.ndoc.typ` / `.md` file to compile.
    pub input: std::path::PathBuf,
    /// 1-based page number the click falls on (Typst page numbering).
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    /// Page-local click coordinate in points, as `x,y` (e.g. `12,14`). The
    /// origin is the page's top-left; a renderer converts a pixel click at
    /// scale `s` (pixels per point) with `point_pt = (px / s, py / s)`.
    #[arg(long, value_name = "X,Y")]
    pub at: String,
}

/// Arguments for `ndoc build`.
#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Input Markdown or `.ndoc.typ` file to compile.
    pub input: std::path::PathBuf,
    /// Output format. Defaults to `pdf`. SVG/PNG of a multi-page document write
    /// one file per page (`<base>-1.<ext>` …) unless `--merged` is given.
    #[arg(long, value_enum)]
    pub format: Option<CliFormat>,
    /// Raster resolution for `--format png`, in dots per inch.
    #[arg(long, default_value_t = 144)]
    pub dpi: u32,
    /// For SVG/PNG, emit a single merged file instead of one file per page.
    #[arg(long)]
    pub merged: bool,
}

/// Arguments for `ndoc render`.
#[derive(Debug, clap::Args)]
pub struct RenderArgs {
    /// Input `.typ` or `.ndoc.typ` file.
    pub input: std::path::PathBuf,
    /// Output path. Its `.pdf`/`.svg`/`.png` extension selects the format and
    /// takes precedence over `--format` (a mismatch is an error). Defaults to
    /// the input path with the format's extension.
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
    /// Output format when `-o` is absent or has no format extension. Defaults to
    /// `pdf`. SVG/PNG of a multi-page document write one file per page
    /// (`<base>-1.<ext>` …) unless `--merged` is given.
    #[arg(long, value_enum)]
    pub format: Option<CliFormat>,
    /// Raster resolution for PNG output, in dots per inch.
    #[arg(long, default_value_t = 144)]
    pub dpi: u32,
    /// For SVG/PNG, emit a single merged file instead of one file per page.
    #[arg(long)]
    pub merged: bool,
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
    pub kind: CliEntryKind,
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

/// CLI mirror of the engine's [`EntryKind`].
///
/// `clap::ValueEnum` cannot be implemented directly on `nordocs_core`'s
/// `EntryKind` (orphan rule — both the trait and the type are foreign to this
/// crate), so the CLI owns this thin enum and converts to the engine type. The
/// accepted value strings (`component`, `template`) are unchanged.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CliEntryKind {
    Component,
    Template,
}

impl From<CliEntryKind> for EntryKind {
    fn from(kind: CliEntryKind) -> Self {
        match kind {
            CliEntryKind::Component => EntryKind::Component,
            CliEntryKind::Template => EntryKind::Template,
        }
    }
}

/// Output formats `render`/`build` can emit.
///
/// The engine has no clap dependency, so the CLI owns this enum and maps it to
/// the relevant [`CompiledDoc`](nordocs_core::compiler::CompiledDoc) exporter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CliFormat {
    Pdf,
    Svg,
    Png,
}

impl CliFormat {
    /// The file extension (without a leading dot) for this format.
    fn ext(self) -> &'static str {
        match self {
            CliFormat::Pdf => "pdf",
            CliFormat::Svg => "svg",
            CliFormat::Png => "png",
        }
    }

    /// Map a file extension (without a leading dot) to a format, if recognised.
    fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "pdf" => Some(CliFormat::Pdf),
            "svg" => Some(CliFormat::Svg),
            "png" => Some(CliFormat::Png),
            _ => None,
        }
    }
}

/// Resolve the output format from the `-o` extension and the `--format` flag.
///
/// Precedence: a recognised `-o` extension wins; otherwise `--format`; otherwise
/// `pdf`. A recognised `-o` extension that disagrees with an explicit `--format`
/// is a hard error rather than a silent precedence surprise.
fn resolve_format(
    output: Option<&std::path::Path>,
    format: Option<CliFormat>,
) -> anyhow::Result<CliFormat> {
    let from_output = output
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .and_then(CliFormat::from_ext);

    match (from_output, format) {
        (Some(o), Some(f)) if o != f => anyhow::bail!(
            "format conflict: output extension '.{}' disagrees with --format {}",
            o.ext(),
            f.ext()
        ),
        (Some(o), _) => Ok(o),
        (None, Some(f)) => Ok(f),
        (None, None) => Ok(CliFormat::Pdf),
    }
}

/// Build the per-page output path `<base>-<page>.<ext>` from a full output path.
///
/// The trailing extension of `output_path` is dropped and replaced by the
/// `-<page>.<ext>` suffix, so `out.svg` + page 2 becomes `out-2.svg`.
fn page_output_path(output_path: &std::path::Path, page: usize, ext: &str) -> std::path::PathBuf {
    let stem = output_path.with_extension("");
    std::path::PathBuf::from(format!("{}-{}.{ext}", stem.display(), page))
}

/// Export `doc` to `output_path` in `format`, returning every written path.
///
/// PDF is always a single file. SVG/PNG honour `merged` (one merged file) and
/// otherwise split a multi-page document into `<base>-1.<ext> … <base>-N.<ext>`;
/// a single-page document writes the bare `<base>.<ext>`. The chosen naming
/// convention is printed (unless `--json`) so truncation/expansion is never
/// silent. `dpi` is the PNG resolution.
fn write_compiled(
    doc: &nordocs_core::compiler::CompiledDoc,
    format: CliFormat,
    output_path: &std::path::Path,
    dpi: u32,
    merged: bool,
    json: bool,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    use anyhow::Context as _;

    let ext = format.ext();
    let dpi = dpi as f32;
    let mut written = Vec::new();

    let write_bytes = |path: &std::path::Path, bytes: &[u8]| -> anyhow::Result<()> {
        std::fs::write(path, bytes).with_context(|| format!("failed to write '{}'", path.display()))
    };

    match format {
        CliFormat::Pdf => {
            let bytes = doc.to_pdf()?;
            write_bytes(output_path, &bytes)?;
            written.push(output_path.to_path_buf());
        }
        CliFormat::Svg if merged => {
            let svg = doc.to_svg_merged()?;
            write_bytes(output_path, svg.as_bytes())?;
            written.push(output_path.to_path_buf());
            if !json {
                println!("wrote merged SVG to '{}'", output_path.display());
            }
        }
        CliFormat::Png if merged => {
            let png = doc.to_png_merged(dpi)?;
            write_bytes(output_path, &png)?;
            written.push(output_path.to_path_buf());
            if !json {
                println!("wrote merged PNG to '{}'", output_path.display());
            }
        }
        CliFormat::Svg | CliFormat::Png => {
            let pages = doc.page_count();
            if pages <= 1 {
                let bytes = render_page(doc, format, 0, dpi)?;
                write_bytes(output_path, &bytes)?;
                written.push(output_path.to_path_buf());
                if !json {
                    println!(
                        "wrote single-page {} to '{}'",
                        ext.to_uppercase(),
                        output_path.display()
                    );
                }
            } else {
                for page in 0..pages {
                    let path = page_output_path(output_path, page + 1, ext);
                    let bytes = render_page(doc, format, page, dpi)?;
                    write_bytes(&path, &bytes)?;
                    written.push(path);
                }
                if !json {
                    let first = written.first().map(|p| p.display().to_string());
                    let last = written.last().map(|p| p.display().to_string());
                    if let (Some(first), Some(last)) = (first, last) {
                        println!(
                            "wrote {pages} pages of {} as '{first}' … '{last}'",
                            ext.to_uppercase()
                        );
                    }
                }
            }
        }
    }

    Ok(written)
}

/// Render one 0-based page of `doc` to bytes in `format` (SVG or PNG only).
fn render_page(
    doc: &nordocs_core::compiler::CompiledDoc,
    format: CliFormat,
    page: usize,
    dpi: f32,
) -> anyhow::Result<Vec<u8>> {
    match format {
        CliFormat::Svg => Ok(doc.to_svg(page)?.into_bytes()),
        CliFormat::Png => Ok(doc.to_png(page, dpi)?),
        CliFormat::Pdf => unreachable!("render_page is never called for PDF"),
    }
}

/// Emit the `--json` success envelope for a multi-format render/build.
///
/// Reports the full list of written paths under `outputs` and, for backward
/// compatibility, the primary (first) path under `output`.
fn emit_output_json(written: &[std::path::PathBuf]) {
    let outputs: Vec<String> = written.iter().map(|p| p.display().to_string()).collect();
    let primary = outputs.first().cloned();
    output::emit_json_result(Some(serde_json::json!({
        "output": primary,
        "outputs": outputs,
    })));
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
            Command::Jump(args) => cmd_jump(args, json),
        }
    }
}

/// Parse an `--at x,y` argument into page-local point coordinates (in points).
fn parse_at(at: &str) -> anyhow::Result<(f64, f64)> {
    let (x, y) = at
        .split_once(',')
        .ok_or_else(|| anyhow::anyhow!("--at must be in the form 'x,y' in points, got '{at}'"))?;
    let x = x
        .trim()
        .parse::<f64>()
        .with_context(|| format!("invalid x coordinate in --at '{at}'"))?;
    let y = y
        .trim()
        .parse::<f64>()
        .with_context(|| format!("invalid y coordinate in --at '{at}'"))?;
    Ok((x, y))
}

/// Compile `path` into a retained session, routing by file shape exactly like
/// `build`/`render`: `.ndoc.typ` archives compose, `.md` converts first, any
/// other Typst file compiles verbatim.
fn compile_input_to_session(
    path: &std::path::Path,
) -> anyhow::Result<nordocs_core::compiler::CompiledDoc> {
    let name = path.to_string_lossy();
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    if name.ends_with(".ndoc.typ") {
        nordocs_core::service::document_archive_to_session(&source)
            .with_context(|| format!("failed to compile '{}'", path.display()))
    } else if name.ends_with(".md") {
        let typst_source = nordocs_core::markdown::markdown_to_typst(&source)
            .with_context(|| format!("failed to convert '{}' to Typst", path.display()))?;
        nordocs_core::compiler::compile(&typst_source, &[])
            .with_context(|| format!("failed to compile '{}'", path.display()))
    } else {
        nordocs_core::compiler::compile(&source, &[])
            .with_context(|| format!("failed to compile '{}'", path.display()))
    }
}

/// Hidden diagnostic: compile `input`, build a page-local Typst point from
/// `--at`/`--page`, and emit the resolved [`nordocs_core::Jump`].
fn cmd_jump(args: JumpArgs, json: bool) -> anyhow::Result<()> {
    use nordocs_core::Jump;

    anyhow::ensure!(args.page >= 1, "--page is 1-based; page 0 is invalid");
    let page_index = args.page - 1;
    let (x_pt, y_pt) = parse_at(&args.at)?;

    let doc = compile_input_to_session(&args.input)?;
    let point =
        typst::layout::Point::new(typst::layout::Abs::pt(x_pt), typst::layout::Abs::pt(y_pt));
    let jump = doc.jump_from_click(page_index, point);

    if json {
        let data = match &jump {
            Some(jump) => serde_json::to_value(jump)
                .with_context(|| "failed to serialise jump result".to_string())?,
            None => serde_json::Value::Null,
        };
        output::emit_json_result(Some(data));
        return Ok(());
    }

    match jump {
        Some(Jump::File {
            path,
            offset,
            line,
            column,
        }) => println!("file {path}:{line}:{column} (byte offset {offset})"),
        Some(Jump::Url { url }) => println!("url {url}"),
        Some(Jump::Position(pos)) => println!(
            "position page {} at {:.2},{:.2}pt",
            pos.page, pos.x_pt, pos.y_pt
        ),
        None => println!(
            "no jump target at page {} ({:.2},{:.2})pt",
            args.page, x_pt, y_pt
        ),
    }
    Ok(())
}

fn cmd_build(args: BuildArgs, json: bool) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    // `build` has no `-o`; the format is chosen by `--format` (default pdf).
    let format = resolve_format(None, args.format)?;
    let ext = format.ext();

    let (doc, output_path) = if name.ends_with(".ndoc.typ") {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        // `.ndoc.typ` covers two shapes: a canonical composed document (the
        // `/*===STATE-START===` reference format, self-contained with embedded
        // images) and the entry-format archive used by the authoring commands.
        // The façade routes by the file's own markers so both compile.
        let doc = nordocs_core::service::document_archive_to_session(&src)
            .with_context(|| format!("failed to compile '{}'", path.display()))?;
        let base = name.strip_suffix(".ndoc.typ").unwrap_or(name.as_ref());
        (doc, std::path::PathBuf::from(format!("{base}.{ext}")))
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        let markdown = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        let typst_source = nordocs_core::markdown::markdown_to_typst(&markdown)
            .with_context(|| format!("failed to convert '{}' to Typst", path.display()))?;
        let doc = nordocs_core::compiler::compile(&typst_source, &[])
            .with_context(|| format!("failed to compile '{}'", path.display()))?;
        (doc, path.with_extension(ext))
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    };

    let written = write_compiled(&doc, format, &output_path, args.dpi, args.merged, json)
        .with_context(|| format!("failed to export '{}'", path.display()))?;

    if json {
        emit_output_json(&written);
    }

    Ok(())
}

fn cmd_new(args: NewArgs, json: bool) -> anyhow::Result<()> {
    nordocs_core::authoring::ndoc::create_document(&args.path)
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
    let kind: EntryKind = args.kind.into();
    nordocs_core::authoring::ndoc::add_entry(&args.document, &args.name, kind, &content)
        .with_context(|| {
            format!(
                "failed to add entry '{}' to '{}'",
                args.name,
                args.document.display()
            )
        })?;
    if json {
        output::emit_json_result(None);
    }
    Ok(())
}

fn cmd_edit(args: EditArgs, json: bool) -> anyhow::Result<()> {
    let content = read_content(args.content_file.as_deref())?;
    nordocs_core::authoring::ndoc::edit_entry(&args.document, &args.name, &content).with_context(
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
/// issue to stdout as `[{severity}] {location}: {message}`, and exits with code
/// 1 when any `error`-severity issue is found (warnings do not fail). Composed
/// documents also report a summary. Unsupported extensions bail with an
/// actionable message before validation is attempted.
fn cmd_validate(args: ValidateArgs, json: bool) -> anyhow::Result<()> {
    use nordocs_core::validation::Severity;

    let path = &args.input;
    let name = path.to_string_lossy();

    let result = if name.ends_with(".ndoc.typ") {
        nordocs_core::validation::validate_ndoc_file(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        nordocs_core::validation::validate_markdown_file(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    };

    let severity_label = |s: Severity| match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };

    if json {
        let violations: Vec<serde_json::Value> = result
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
        let summary = result.summary.as_ref().map(|s| {
            serde_json::json!({
                "templateId": s.template_id,
                "templateVersion": s.template_version,
                "themeId": s.theme_id,
                "nodeCount": s.node_count,
                "globalInputCount": s.global_input_count,
            })
        });
        output::emit_json_result(Some(serde_json::json!({
            "violations": violations,
            "summary": summary,
            "valid": result.is_valid(),
        })));
        if !result.is_valid() {
            std::process::exit(1);
        }
    } else {
        for v in &result.violations {
            println!(
                "[{}] {}: {}",
                severity_label(v.severity),
                v.location,
                v.message
            );
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

    let format = resolve_format(args.output.as_deref(), args.format)?;

    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    let doc = nordocs_core::compiler::compile(&source, &[])
        .with_context(|| format!("failed to compile '{}'", path.display()))?;

    let output_path = args
        .output
        .unwrap_or_else(|| std::path::PathBuf::from(format!("{base}.{}", format.ext())));

    let written = write_compiled(&doc, format, &output_path, args.dpi, args.merged, json)
        .with_context(|| format!("failed to export '{}'", path.display()))?;

    if json {
        emit_output_json(&written);
    }

    Ok(())
}

fn cmd_preview(args: PreviewArgs, json: bool) -> anyhow::Result<()> {
    let path = &args.input;
    let name = path.to_string_lossy();

    let pdf_bytes = if name.ends_with(".ndoc.typ") {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        // Same routing as `build`: a canonical composed document renders
        // directly (resolving its embedded images), while an entry-format
        // archive is parsed and its entries concatenated before compiling.
        nordocs_core::service::document_archive_to_pdf(&src)
            .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?
    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
        let markdown = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        let typst_source = nordocs_core::markdown::markdown_to_typst(&markdown)
            .with_context(|| format!("failed to convert '{}' to Typst", path.display()))?;
        nordocs_core::compiler::compile_to_pdf(&typst_source)
            .with_context(|| format!("failed to compile '{}' to PDF", path.display()))?
    } else {
        anyhow::bail!(
            "unsupported input format '{}': expected a '.md' or '.ndoc.typ' file",
            path.display()
        );
    };

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
    let schema = nordocs_core::schema::parse::parse_template_file(&template_path)
        .with_context(|| format!("failed to read template '{}'", template_path.display()))?;

    let output = args
        .output
        .unwrap_or_else(|| std::path::PathBuf::from(format!("{}{DOCUMENT_SUFFIX}", schema.name)));
    anyhow::ensure!(
        !output.exists(),
        "refusing to overwrite existing document '{}'",
        output.display()
    );

    let doc = nordocs_core::model::Document {
        template: schema.name,
        inputs: std::collections::BTreeMap::new(),
        nodes: Vec::new(),
        images: Vec::new(),
    };
    nordocs_core::authoring::doc_state::write_document(&output, &doc)
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
    let doc = nordocs_core::authoring::doc_state::read_document(&args.document)
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
fn print_outline_node(node: &nordocs_core::model::Node, depth: usize) {
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
fn outline_node_json(node: &nordocs_core::model::Node) -> serde_json::Value {
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
/// single atomic [`write_document`](nordocs_core::authoring::doc_state::write_document)
/// happens only after validation and placement succeed, so an unknown type or an
/// unknown placement target leaves the document unchanged and exits non-zero.
/// `--json` reports the minted node id.
fn cmd_doc_add(args: DocAddArgs, json: bool) -> anyhow::Result<()> {
    let catalogue = nordocs_core::schema::Catalogue::from_builtins();
    anyhow::ensure!(
        catalogue.component(&args.node_type).is_some(),
        "unknown component type '{}'",
        args.node_type
    );

    let inputs = parse_seed_inputs(&args.inputs)?;

    let mut doc = nordocs_core::authoring::doc_state::read_document(&args.document)
        .with_context(|| format!("failed to read document '{}'", args.document.display()))?;

    let id = nordocs_core::model::NodeId::mint(&args.node_type, &doc.node_ids());
    let node = nordocs_core::model::Node {
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

    nordocs_core::authoring::doc_state::write_document(&args.document, &doc)
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
    let mut doc = nordocs_core::authoring::doc_state::read_document(&args.document)
        .with_context(|| format!("failed to read document '{}'", args.document.display()))?;

    let removed = remove_node(&mut doc.nodes, &args.node_id, args.with_children);
    anyhow::ensure!(removed, "unknown node id '{}'", args.node_id);

    nordocs_core::authoring::doc_state::write_document(&args.document, &doc)
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
) -> anyhow::Result<std::collections::BTreeMap<String, nordocs_core::model::InputValue>> {
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
            nordocs_core::model::InputValue {
                kind: nordocs_core::model::InputKind::String,
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
    nodes: &mut Vec<nordocs_core::model::Node>,
    node: nordocs_core::model::Node,
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
    nodes: &mut [nordocs_core::model::Node],
    parent_id: &str,
    node: nordocs_core::model::Node,
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
    nodes: &mut Vec<nordocs_core::model::Node>,
    target_id: &str,
    node: nordocs_core::model::Node,
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
fn remove_node(
    nodes: &mut Vec<nordocs_core::model::Node>,
    target_id: &str,
    with_children: bool,
) -> bool {
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
/// Resolves the target input's declared [`InputKind`](nordocs_core::model::InputKind)
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

    let catalogue = nordocs_core::schema::Catalogue::from_builtins();
    let mut doc = nordocs_core::authoring::doc_state::read_document(&args.document)
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
    let input = nordocs_core::model::InputValue { kind, value };

    if args.document_level {
        doc.inputs.insert(args.key.clone(), input);
    } else {
        let node_id = args.node_id.as_deref().unwrap_or_default();
        let node =
            find_node_mut(&mut doc.nodes, node_id).expect("node existed during kind resolution");
        node.inputs.insert(args.key.clone(), input);
    }

    nordocs_core::authoring::doc_state::write_document(&args.document, &doc)
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

/// Find the declared [`InputKind`](nordocs_core::model::InputKind) for `key` among
/// `inputs`, or `None` when no input declares that name.
fn declared_kind(
    inputs: &[nordocs_core::schema::InputSchema],
    key: &str,
) -> Option<nordocs_core::model::InputKind> {
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
    kind: nordocs_core::model::InputKind,
    value: &str,
    image_names: &std::collections::HashSet<&str>,
) -> anyhow::Result<serde_json::Value> {
    use nordocs_core::model::InputKind;
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
fn find_node<'a>(
    nodes: &'a [nordocs_core::model::Node],
    id: &str,
) -> Option<&'a nordocs_core::model::Node> {
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
    nodes: &'a mut [nordocs_core::model::Node],
    id: &str,
) -> Option<&'a mut nordocs_core::model::Node> {
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
        let schema = nordocs_core::schema::parse::parse_component_file(&args.target)
            .with_context(|| format!("failed to read component '{}'", args.target.display()))?;
        if json {
            output::emit_json_result(Some(serde_json::json!({"component": schema})));
        } else {
            println!("component: {}", schema.name);
            print_inputs(&schema.inputs);
        }
    } else if name.ends_with(TEMPLATE_SUFFIX) {
        let schema = nordocs_core::schema::parse::parse_template_file(&args.target)
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
fn print_inputs(inputs: &[nordocs_core::schema::InputSchema]) {
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
            let schema = nordocs_core::schema::parse::parse_component_file(&file)
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
            let schemas = nordocs_core::schema::parse::load_components_from_dir(&dir)
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
            let items = nordocs_core::item::load_items_from_dir(&dir)
                .with_context(|| format!("failed to load items in '{}'", dir.display()))?;
            let collections = nordocs_core::item::summarise_collections(&items);
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
            let items = nordocs_core::item::load_items_from_dir(&dir)
                .with_context(|| format!("failed to load items in '{}'", dir.display()))?;
            let components = nordocs_core::schema::parse::load_components_from_dir(&dir)
                .with_context(|| format!("failed to load schemas in '{}'", dir.display()))?;
            let issues = nordocs_core::item::validate_items(&items, &components);

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

/// Render an [`InputKind`](nordocs_core::model::InputKind) as its lowercase wire string
/// (the same token used in the file frontmatter `type:` key).
fn input_kind_str(kind: nordocs_core::model::InputKind) -> String {
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
            let schema = nordocs_core::schema::parse::parse_template_file(&path)
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

            let outcome =
                nordocs_core::authoring::doc_state::embed_image(&file, &image_name, &bytes)
                    .with_context(|| {
                        format!(
                            "failed to embed image '{}' into '{}'",
                            image.display(),
                            file.display()
                        )
                    })?;

            use nordocs_core::authoring::doc_state::ImageEmbed;
            let (hash, added) = match outcome {
                ImageEmbed::Added { hash } => (hash, true),
                ImageEmbed::AlreadyPresent { hash } => (hash, false),
            };

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

#[cfg(test)]
mod tests {
    use super::*;
    use nordocs_core::model::InputKind;
    use std::collections::HashSet;

    #[test]
    fn coerce_boolean_value_parses_true_and_false_to_json_bool() {
        let names = HashSet::new();
        assert_eq!(
            coerce_value(InputKind::Boolean, "true", &names).unwrap(),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            coerce_value(InputKind::Boolean, "false", &names).unwrap(),
            serde_json::Value::Bool(false)
        );
        coerce_value(InputKind::Boolean, "yes", &names)
            .expect_err("a non true/false value for a boolean input is rejected");
    }
}
