//! High-level compile/export wrapper.
//!
//! Compilation yields a retained [`CompiledDoc`] that owns both the
//! [`NordocsWorld`] and the laid-out [`typst::layout::PagedDocument`]. PDF, SVG,
//! and PNG are exporters over that single value, so adding formats never
//! recompiles. Diagnostics are flattened into [`Error::Compile`].
//!
//! The free functions ([`compile_to_pdf`], [`compile_to_pdf_with_options`],
//! [`compile_to_pdf_with_images`]) are thin wrappers that compile a
//! [`CompiledDoc`] and immediately export PDF, preserving their historical
//! signatures and byte-for-byte PDF output.

use serde::Serialize;
use typst::layout::{Abs, Page, PagedDocument};
use typst::syntax::{FileId, VirtualPath};
use typst::World;

use crate::error::{Error, Result};
use crate::typst_world::NordocsWorld;

/// Resolved target of a click on a rendered page (backward source map).
///
/// A serialisable re-expression of `typst_ide::Jump` so the CLI and FFI can
/// marshal it without re-deriving anything or handling raw `FileId`s. Produced
/// by [`CompiledDoc::jump_from_click`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Jump {
    /// Jump to a byte `offset` (and resolved 1-based `line`/`column`) within the
    /// source file at `path`.
    File {
        /// Virtual path of the source file the glyph originated from.
        path: String,
        /// Byte offset of the target within the source text.
        offset: usize,
        /// 1-based line number of `offset` within the source.
        line: usize,
        /// 1-based column number (in characters) of `offset` within the line.
        column: usize,
    },
    /// Jump to an external hyperlink.
    Url {
        /// The destination URL.
        url: String,
    },
    /// Jump to an on-page location (e.g. an internal reference/outline target).
    Position(Position),
}

/// An on-page location in the rendered document (forward source map).
///
/// Coordinates are page-local, in typographic points (`pt`). `page` is 1-based
/// to match Typst's own page numbering. Produced by
/// [`CompiledDoc::jump_from_cursor`] and embedded in [`Jump::Position`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Position {
    /// 1-based page number the location falls on.
    pub page: usize,
    /// Horizontal offset from the page origin, in points.
    pub x_pt: f64,
    /// Vertical offset from the page origin, in points.
    pub y_pt: f64,
}

/// A compiled document retaining both the world and the laid-out pages.
///
/// Construct via [`compile`] (or the PDF convenience wrappers). Exporters are
/// methods so the same compilation can be emitted as PDF, SVG, or PNG without
/// recompiling. The world is retained alongside the document so future
/// click-to-source jump support (`source-map-session`) and the FFI session
/// handle (`dotnet-ffi`) can reuse this exact value.
pub struct CompiledDoc {
    world: NordocsWorld,
    document: PagedDocument,
}

impl CompiledDoc {
    /// Number of laid-out pages in the document.
    pub fn page_count(&self) -> usize {
        self.document.pages.len()
    }

    /// Size of the page at `index` as `(width_pt, height_pt)` in points, or
    /// `None` if the index is out of range.
    ///
    /// Pair this with [`CompiledDoc::jump_from_click`] to map a UI click to a
    /// page-local [`typst::layout::Point`]. A renderer showing page *i* scaled
    /// by `s` rendered pixels per point converts a pixel click `(px, py)` to
    /// `point_pt = (px / s, py / s)`; `page_size` lets the caller bound or
    /// clamp that point to the page.
    pub fn page_size(&self, index: usize) -> Option<(f64, f64)> {
        let page = self.document.pages.get(index)?;
        let size = page.frame.size();
        Some((size.x.to_pt(), size.y.to_pt()))
    }

    /// Map a click on a rendered page back to its source location.
    ///
    /// `page_index` is 0-based; `point` is **page-local**, in typographic
    /// points (`pt`), measured from the page's top-left origin. The screen →
    /// document transform a renderer must apply is `point_pt = (px / s, py / s)`
    /// where `s` is the render scale in rendered pixels per point
    /// (`pixels_per_pt = dpi / 72.0`); the caller picks the page explicitly and
    /// subtracts any inter-page gutter before calling. The library never guesses
    /// which page a global coordinate falls in, keeping the contract unambiguous
    /// across merged and per-page layouts.
    ///
    /// Returns `None` when `page_index` is out of range or the click lands on
    /// empty space / synthetic content carrying no source span. Delegates to
    /// [`typst_ide::jump_from_click`] against the page's laid-out frame.
    pub fn jump_from_click(&self, page_index: usize, point: typst::layout::Point) -> Option<Jump> {
        let frame = &self.document.pages.get(page_index)?.frame;
        let jump = typst_ide::jump_from_click(&self.world, &self.document, frame, point)?;
        self.convert_jump(jump)
    }

    /// Map a source cursor to the on-page positions it produced (forward map).
    ///
    /// `file_path` is resolved against the world's virtual filesystem (the
    /// composed source is `main.typ`); `offset` is a byte offset into that
    /// source's text. The returned [`Position`]s are page-local points in `pt`
    /// with 1-based page numbers, suitable for highlighting the cursor's output
    /// in a preview. Returns an empty vector when the file is unknown or the
    /// cursor sits on content with no on-page glyphs. Delegates to
    /// [`typst_ide::jump_from_cursor`].
    pub fn jump_from_cursor(&self, file_path: &str, offset: usize) -> Vec<Position> {
        let id = FileId::new(None, VirtualPath::new(file_path));
        let Ok(source) = self.world.source(id) else {
            return Vec::new();
        };
        typst_ide::jump_from_cursor(&self.document, &source, offset)
            .into_iter()
            .map(convert_position)
            .collect()
    }

    /// Convert a `typst_ide::Jump` into the serialisable [`Jump`], resolving the
    /// `FileId` to a virtual path string and the byte offset to 1-based
    /// line/column via the source text.
    fn convert_jump(&self, jump: typst_ide::Jump) -> Option<Jump> {
        match jump {
            typst_ide::Jump::File(id, offset) => {
                let source = self.world.source(id).ok()?;
                let path = id.vpath().as_rootless_path().to_string_lossy().into_owned();
                let (line, column) = source
                    .lines()
                    .byte_to_line_column(offset)
                    .map(|(l, c)| (l + 1, c + 1))
                    .unwrap_or((0, 0));
                Some(Jump::File {
                    path,
                    offset,
                    line,
                    column,
                })
            }
            typst_ide::Jump::Url(url) => Some(Jump::Url {
                url: url.into_inner().to_string(),
            }),
            typst_ide::Jump::Position(pos) => Some(Jump::Position(convert_position(pos))),
        }
    }

    /// Borrow the page at `index`, returning a validation error if out of range.
    fn page(&self, index: usize) -> Result<&Page> {
        self.document.pages.get(index).ok_or_else(|| {
            Error::Validation(format!(
                "page index {index} out of range (document has {} page(s))",
                self.document.pages.len()
            ))
        })
    }

    /// Export the document to PDF bytes using the default options.
    pub fn to_pdf(&self) -> Result<Vec<u8>> {
        self.to_pdf_with_options(&typst_pdf::PdfOptions::default())
    }

    /// Export the document to PDF bytes with explicit [`typst_pdf::PdfOptions`].
    ///
    /// This is the exact `typst_pdf::pdf()` call/options used historically, so
    /// the produced PDF bytes are unchanged.
    pub fn to_pdf_with_options(&self, pdf_options: &typst_pdf::PdfOptions) -> Result<Vec<u8>> {
        typst_pdf::pdf(&self.document, pdf_options).map_err(diags_to_compile_error)
    }

    /// Export a single page (0-based) to an SVG string.
    pub fn to_svg(&self, page: usize) -> Result<String> {
        Ok(typst_svg::svg(self.page(page)?))
    }

    /// Export all pages to a single merged SVG canvas.
    pub fn to_svg_merged(&self) -> Result<String> {
        Ok(typst_svg::svg_merged(&self.document, Abs::zero()))
    }

    /// Render a single page (0-based) to PNG bytes at the given DPI.
    ///
    /// `typst-render` works in pixels-per-point, so DPI is converted with
    /// `pixels_per_pt = dpi / 72.0`.
    pub fn to_png(&self, page: usize, dpi: f32) -> Result<Vec<u8>> {
        let pixmap = typst_render::render(self.page(page)?, dpi / 72.0);
        pixmap
            .encode_png()
            .map_err(|e| Error::Compile(format!("PNG encode failed: {e}")))
    }

    /// Render all pages stacked vertically into a single merged PNG.
    pub fn to_png_merged(&self, dpi: f32) -> Result<Vec<u8>> {
        let pixmap = typst_render::render_merged(
            &self.document,
            dpi / 72.0,
            Abs::zero(),
            Some(typst::visualize::Color::WHITE),
        );
        pixmap
            .encode_png()
            .map_err(|e| Error::Compile(format!("PNG encode failed: {e}")))
    }
}

/// Compile a composed `.typ` source string into a retained [`CompiledDoc`].
///
/// Builds a fresh [`NordocsWorld`] with the ordered `sys_inputs` injected as
/// `sys.inputs`, runs the Typst compiler, and retains both the world and the
/// laid-out document. Compilation warnings are currently discarded; surface
/// them once the CLI grows a diagnostics channel.
pub fn compile(source: &str, sys_inputs: &[(String, String)]) -> Result<CompiledDoc> {
    let world = NordocsWorld::with_inputs(source.to_owned(), sys_inputs);
    compile_world(world)
}

/// Compile a composed `.typ` source string into PDF bytes.
pub fn compile_to_pdf(main_source: &str) -> Result<Vec<u8>> {
    compile_to_pdf_with_options(main_source, &typst_pdf::PdfOptions::default())
}

/// Compile a composed source string into a retained [`CompiledDoc`] with
/// embedded images.
///
/// Each `(name, bytes)` pair is registered in the compiler's virtual filesystem
/// at `images/{name}` so the document's `image("images/{name}")` calls resolve.
/// The returned document can be exported to PDF, SVG, or PNG without
/// recompiling.
pub fn compile_with_images(main_source: &str, images: &[(String, Vec<u8>)]) -> Result<CompiledDoc> {
    let mut world = NordocsWorld::new(main_source.to_owned());
    for (name, bytes) in images {
        world.insert_file(
            &format!("images/{name}"),
            typst::foundations::Bytes::new(bytes.clone()),
        );
    }
    compile_world(world)
}

/// Compile a composed source string into PDF bytes with embedded images.
///
/// Each `(name, bytes)` pair is registered in the compiler's virtual filesystem
/// at `images/{name}` so the document's `image("images/{name}")` calls resolve.
pub fn compile_to_pdf_with_images(
    main_source: &str,
    images: &[(String, Vec<u8>)],
) -> Result<Vec<u8>> {
    compile_with_images(main_source, images)?.to_pdf()
}

fn compile_to_pdf_with_options(
    main_source: &str,
    pdf_options: &typst_pdf::PdfOptions,
) -> Result<Vec<u8>> {
    compile(main_source, &[])?.to_pdf_with_options(pdf_options)
}

/// Compile a fully-prepared [`NordocsWorld`] into a retained [`CompiledDoc`].
///
/// Shared by every entry point so image-overlay and bare-source compiles run
/// the identical compile path. Keeps the incremental `comemo` cache bounded
/// after each compile. Compilation warnings are currently discarded; surface
/// them once the CLI grows a diagnostics channel.
fn compile_world(world: NordocsWorld) -> Result<CompiledDoc> {
    let compiled = typst::compile::<PagedDocument>(&world);
    let document = compiled.output.map_err(diags_to_compile_error)?;

    // Keep the incremental cache bounded between invocations.
    NordocsWorld::evict_cache(5);

    Ok(CompiledDoc { world, document })
}

/// Convert a `typst::layout::Position` (1-based page, page-local point) into the
/// serialisable [`Position`] with coordinates in points.
fn convert_position(pos: typst::layout::Position) -> Position {
    Position {
        page: pos.page.get(),
        x_pt: pos.point.x.to_pt(),
        y_pt: pos.point.y.to_pt(),
    }
}

/// Flatten Typst diagnostics into an [`Error::Compile`] message.
fn diags_to_compile_error<I>(diags: I) -> Error
where
    I: IntoIterator,
    I::Item: AsCompileDiagnostic,
{
    let msg = diags
        .into_iter()
        .map(|d| d.message_string())
        .collect::<Vec<_>>()
        .join("; ");
    Error::Compile(msg)
}

/// Internal helper so [`diags_to_compile_error`] can accept both
/// `EcoVec<SourceDiagnostic>` and its borrowed iterator forms.
trait AsCompileDiagnostic {
    fn message_string(&self) -> String;
}

impl AsCompileDiagnostic for typst::diag::SourceDiagnostic {
    fn message_string(&self) -> String {
        self.message.to_string()
    }
}

impl AsCompileDiagnostic for &typst::diag::SourceDiagnostic {
    fn message_string(&self) -> String {
        self.message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{compile, compile_to_pdf, compile_to_pdf_with_options};
    use crate::error::Error;

    #[test]
    fn compile_to_pdf_happy_path() {
        let result = compile_to_pdf("Hello, Typst!").expect("valid source should compile to PDF");
        assert!(!result.is_empty(), "PDF bytes should be non-empty");
    }

    #[test]
    fn compile_to_pdf_invalid_source() {
        let result = compile_to_pdf("#panic(\"forced compile error\")");
        match result {
            Err(Error::Compile(msg)) => {
                assert!(!msg.is_empty(), "compile error message should be non-empty")
            }
            other => panic!("expected Error::Compile, got {:?}", other),
        }
    }

    #[test]
    fn compile_to_pdf_empty_source_succeeds() {
        let pdf = compile_to_pdf("").expect("empty source compiles to a blank PDF");
        assert!(!pdf.is_empty(), "blank PDF must produce non-empty bytes");
    }

    #[test]
    fn compile_to_pdf_image_or_placeholder_with_empty_name_renders_placeholder() {
        // An empty name takes the placeholder branch (a dashed rect), so the
        // source must compile to a PDF without touching any image asset.
        let source = "#let image-or-placeholder(\n\
              name,\n\
              width: auto,\n\
              height: auto,\n\
              placeholder-width: auto,\n\
              placeholder-height: auto,\n\
              ..args,\n\
            ) = {\n\
              if name == none or name == \"\" {\n\
                rect(\n\
                  width: placeholder-width,\n\
                  height: placeholder-height,\n\
                  stroke: (dash: \"dashed\", paint: luma(180), thickness: 1pt),\n\
                  radius: 12pt,\n\
                )[\n\
                  #align(center + horizon)[\n\
                    #text(size: 10pt, fill: luma(150))[Photo]\n\
                  ]\n\
                ]\n\
              } else {\n\
                image(\"images/\" + name, width: width, height: height, ..args)\n\
              }\n\
            }\n\n\
            #image-or-placeholder(\"\", placeholder-width: 4cm, placeholder-height: 6cm)\n";
        let pdf = compile_to_pdf(source).expect("placeholder branch compiles to PDF");
        assert!(
            !pdf.is_empty(),
            "placeholder render must produce non-empty PDF"
        );
        assert_eq!(&pdf[..5], b"%PDF-");
    }

    #[test]
    fn compile_to_pdf_export_error_maps_to_compile_error() {
        // PDF/UA-1 requires a document title. A plain document without
        // `#set document(title: ...)` fails at typst_pdf::pdf(), exercising
        // the PDF export error closure.
        let standards = typst_pdf::PdfStandards::new(&[typst_pdf::PdfStandard::Ua_1])
            .expect("PDF/UA-1 standards creation should succeed");
        let options = typst_pdf::PdfOptions {
            standards,
            ..typst_pdf::PdfOptions::default()
        };
        let result = compile_to_pdf_with_options("Hello!", &options);
        match result {
            Err(Error::Compile(msg)) => {
                assert!(
                    !msg.is_empty(),
                    "PDF/UA-1 violation should produce a non-empty error message"
                );
            }
            other => panic!(
                "expected Error::Compile from PDF/UA-1 title violation, got {:?}",
                other
            ),
        }
    }

    // A small, stable fixture used for the SVG snapshot. Fixed page size and
    // text keeps the rendered SVG deterministic across runs.
    const SVG_FIXTURE: &str = "#set page(width: 120pt, height: 80pt, margin: 10pt)\nSVG";

    #[test]
    fn to_svg_single_page_snapshot() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let svg = doc.to_svg(0).expect("first page exports to SVG");
        assert!(
            svg.starts_with("<svg"),
            "SVG must start with <svg: {svg:.40}"
        );
        assert!(
            svg.contains("</svg>"),
            "SVG must be a complete document with a closing tag"
        );
        insta::assert_snapshot!("svg_single_page", svg);
    }

    #[test]
    fn to_svg_merged_combines_pages() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let svg = doc.to_svg_merged().expect("merged SVG exports");
        assert!(svg.starts_with("<svg"), "merged SVG must start with <svg");
        assert!(svg.contains("</svg>"), "merged SVG must be complete");
    }

    #[test]
    fn to_png_single_page_is_valid_png() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let png = doc.to_png(0, 144.0).expect("first page renders to PNG");
        assert!(!png.is_empty(), "PNG bytes must be non-empty");
        // PNG magic number.
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "must begin with PNG magic");
        // IHDR width/height are big-endian u32 at byte offsets 16 and 20.
        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        assert!(
            width > 0 && height > 0,
            "decoded dimensions must be positive"
        );
    }

    #[test]
    fn to_png_merged_is_valid_png() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let png = doc.to_png_merged(144.0).expect("merged PNG renders");
        assert!(!png.is_empty(), "merged PNG bytes must be non-empty");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "must begin with PNG magic");
        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        assert!(
            width > 0 && height > 0,
            "decoded dimensions must be positive"
        );
    }

    #[test]
    fn page_count_reflects_document() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        assert_eq!(doc.page_count(), 1, "single-page fixture has one page");
    }

    #[test]
    fn to_svg_out_of_range_page_errors() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        match doc.to_svg(99) {
            Err(Error::Validation(msg)) => assert!(msg.contains("out of range")),
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    #[test]
    fn to_png_out_of_range_page_errors() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        match doc.to_png(99, 144.0) {
            Err(Error::Validation(msg)) => assert!(msg.contains("out of range")),
            other => panic!("expected Validation error, got {other:?}"),
        }
    }

    use super::{Jump, Position};
    use typst::layout::{Abs, Point};

    /// Scan the page on a 1pt grid for the first click that resolves to a
    /// `Jump::File`, returning the click point and the resolved jump.
    fn first_file_jump(doc: &super::CompiledDoc) -> Option<(Point, Jump)> {
        let (w, h) = doc.page_size(0).expect("page 0 has a size");
        let mut y = 0.0;
        while y < h {
            let mut x = 0.0;
            while x < w {
                let point = Point::new(Abs::pt(x), Abs::pt(y));
                if let Some(jump @ Jump::File { .. }) = doc.jump_from_click(0, point) {
                    return Some((point, jump));
                }
                x += 1.0;
            }
            y += 1.0;
        }
        None
    }

    #[test]
    fn page_size_reports_dimensions_in_points() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let (w, h) = doc.page_size(0).expect("page 0 has a size");
        assert!((w - 120.0).abs() < 0.01, "width should be 120pt, got {w}");
        assert!((h - 80.0).abs() < 0.01, "height should be 80pt, got {h}");
        assert!(doc.page_size(99).is_none(), "out-of-range page has no size");
    }

    #[test]
    fn jump_from_click_on_glyph_resolves_source_location() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let (_, jump) = first_file_jump(&doc).expect("a click must land on the SVG glyph");
        let svg_start = SVG_FIXTURE.find("SVG").expect("fixture contains SVG text");
        match jump {
            Jump::File {
                path,
                offset,
                line,
                column,
            } => {
                assert!(
                    path.ends_with("main.typ"),
                    "path should be main.typ: {path}"
                );
                assert_eq!(line, 2, "the SVG text is on the second source line");
                assert!(
                    (1..=4).contains(&column),
                    "column should fall within the 3-glyph word: {column}"
                );
                assert!(
                    (svg_start..=svg_start + 3).contains(&offset),
                    "offset {offset} should land within SVG at {svg_start}"
                );
            }
            other => panic!("expected Jump::File, got {other:?}"),
        }
    }

    #[test]
    fn jump_from_click_on_empty_space_returns_none() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        // The lower-right corner of the page is well clear of the top-left text.
        let point = Point::new(Abs::pt(115.0), Abs::pt(75.0));
        assert!(
            doc.jump_from_click(0, point).is_none(),
            "a click on empty space resolves to no jump target"
        );
    }

    #[test]
    fn jump_from_click_out_of_range_page_returns_none() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        assert!(
            doc.jump_from_click(99, Point::new(Abs::pt(12.0), Abs::pt(12.0)))
                .is_none(),
            "an out-of-range page index resolves to no jump target"
        );
    }

    #[test]
    fn jump_from_cursor_maps_to_on_page_positions() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        let svg_start = SVG_FIXTURE.find("SVG").expect("fixture contains SVG text");
        // A cursor inside the "SVG" text node maps to its rendered glyph.
        let positions: Vec<Position> = doc.jump_from_cursor("main.typ", svg_start + 1);
        assert!(
            !positions.is_empty(),
            "cursor inside rendered text yields at least one on-page position"
        );
        let p = &positions[0];
        assert_eq!(p.page, 1, "single-page fixture maps to 1-based page 1");
        assert!(
            p.x_pt >= 0.0 && p.y_pt >= 0.0,
            "coordinates are non-negative"
        );
    }

    #[test]
    fn jump_from_cursor_unknown_file_is_empty() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        assert!(
            doc.jump_from_cursor("does-not-exist.typ", 0).is_empty(),
            "an unknown source file yields no positions"
        );
    }

    #[test]
    fn jump_from_cursor_out_of_range_offset_is_graceful() {
        let doc = compile(SVG_FIXTURE, &[]).expect("fixture compiles");
        // An offset far past the end of the source must not panic; it simply
        // maps to no on-page positions.
        let positions = doc.jump_from_cursor("main.typ", 100_000);
        assert!(
            positions.is_empty(),
            "an out-of-bounds cursor offset yields no positions"
        );
    }

    #[test]
    fn compile_with_sys_inputs_resolves_value() {
        // Reading an injected sys.inputs value must compile cleanly.
        let doc = compile(
            "#sys.inputs.title",
            &[("title".to_string(), "Injected".to_string())],
        )
        .expect("document reading sys.inputs.title compiles when injected");
        assert_eq!(doc.page_count(), 1);
    }

    #[test]
    fn sys_inputs_value_flows_into_rendered_output() {
        // SVG renders text as glyph paths, so the injected value can't be
        // matched as literal text. Instead, prove the value reaches layout by
        // showing two different inputs produce two different renderings, while
        // a third compile with the first value reproduces the first rendering.
        let render = |value: &str| {
            compile(
                "#sys.inputs.title",
                &[("title".to_string(), value.to_string())],
            )
            .expect("document reading sys.inputs.title compiles")
            .to_svg(0)
            .expect("first page exports to SVG")
        };
        let alpha = render("Alpha");
        let beta = render("Beta");
        let alpha_again = render("Alpha");
        assert_ne!(
            alpha, beta,
            "distinct sys.inputs values must yield distinct rendered output"
        );
        assert_eq!(
            alpha, alpha_again,
            "the same sys.inputs value must render deterministically"
        );
    }

    #[test]
    fn sys_inputs_last_value_wins_for_duplicate_keys() {
        // The ordered (key, value) list is folded into the inputs dict, so a
        // later pair with the same key overrides an earlier one. The render
        // must therefore match a single injection of the later value.
        let duplicated = compile(
            "#sys.inputs.title",
            &[
                ("title".to_string(), "First".to_string()),
                ("title".to_string(), "Second".to_string()),
            ],
        )
        .expect("duplicate keys compile")
        .to_svg(0)
        .expect("exports to SVG");
        let single = compile(
            "#sys.inputs.title",
            &[("title".to_string(), "Second".to_string())],
        )
        .expect("single key compiles")
        .to_svg(0)
        .expect("exports to SVG");
        assert_eq!(
            duplicated, single,
            "the last value for a duplicated key must win"
        );
    }

    // A deterministic two-page fixture: an explicit pagebreak forces a second
    // page with distinct text so per-page exports differ.
    const MULTI_PAGE_FIXTURE: &str =
        "#set page(width: 120pt, height: 80pt, margin: 10pt)\nPage one\n#pagebreak()\nPage two";

    /// Read the big-endian IHDR height (bytes 20..24) from PNG bytes.
    fn png_height(png: &[u8]) -> u32 {
        u32::from_be_bytes([png[20], png[21], png[22], png[23]])
    }

    #[test]
    fn multi_page_document_reports_each_page() {
        let doc = compile(MULTI_PAGE_FIXTURE, &[]).expect("multi-page fixture compiles");
        assert_eq!(doc.page_count(), 2, "pagebreak yields two pages");
        // Both pages are sized; the third index is out of range.
        assert!(doc.page_size(0).is_some(), "page 0 has a size");
        assert!(doc.page_size(1).is_some(), "page 1 has a size");
        assert!(doc.page_size(2).is_none(), "page 2 is out of range");
    }

    #[test]
    fn to_svg_renders_distinct_pages() {
        let doc = compile(MULTI_PAGE_FIXTURE, &[]).expect("multi-page fixture compiles");
        let first = doc.to_svg(0).expect("page 0 exports to SVG");
        let second = doc.to_svg(1).expect("page 1 exports to SVG");
        assert!(first.starts_with("<svg") && second.starts_with("<svg"));
        assert_ne!(
            first, second,
            "the two pages carry different text and must render differently"
        );
    }

    #[test]
    fn to_svg_merged_includes_every_page_background() {
        let doc = compile(MULTI_PAGE_FIXTURE, &[]).expect("multi-page fixture compiles");
        let merged = doc.to_svg_merged().expect("merged SVG exports");
        assert!(merged.starts_with("<svg") && merged.contains("</svg>"));
        // Each page contributes its own white background rectangle, so a merged
        // two-page canvas carries at least two of them.
        let backgrounds = merged.matches("fill=\"#ffffff\"").count();
        assert!(
            backgrounds >= 2,
            "merged SVG should contain both page backgrounds, found {backgrounds}"
        );
    }

    #[test]
    fn to_png_merged_stacks_pages_taller_than_one() {
        let doc = compile(MULTI_PAGE_FIXTURE, &[]).expect("multi-page fixture compiles");
        let single = doc.to_png(0, 144.0).expect("page 0 renders to PNG");
        let merged = doc.to_png_merged(144.0).expect("merged PNG renders");
        assert_eq!(
            &merged[..8],
            b"\x89PNG\r\n\x1a\n",
            "must begin with PNG magic"
        );
        assert!(
            png_height(&merged) > png_height(&single),
            "vertically stacking two pages must exceed a single page's height"
        );
    }

    #[test]
    fn to_png_renders_second_page() {
        let doc = compile(MULTI_PAGE_FIXTURE, &[]).expect("multi-page fixture compiles");
        let png = doc.to_png(1, 144.0).expect("page 1 renders to PNG");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "must begin with PNG magic");
        assert!(png_height(&png) > 0, "decoded height must be positive");
    }

    #[test]
    fn jump_from_click_targets_the_clicked_page() {
        let doc = compile(MULTI_PAGE_FIXTURE, &[]).expect("multi-page fixture compiles");
        // Find a glyph hit on the second page and confirm it resolves to the
        // "Page two" line, not the first page's text.
        let (w, h) = doc.page_size(1).expect("page 1 has a size");
        let mut hit = None;
        let mut y = 0.0;
        while y < h && hit.is_none() {
            let mut x = 0.0;
            while x < w {
                if let Some(j @ Jump::File { .. }) =
                    doc.jump_from_click(1, Point::new(Abs::pt(x), Abs::pt(y)))
                {
                    hit = Some(j);
                    break;
                }
                x += 1.0;
            }
            y += 1.0;
        }
        match hit.expect("a click must land on page two's text") {
            Jump::File { line, path, .. } => {
                assert!(
                    path.ends_with("main.typ"),
                    "path should be main.typ: {path}"
                );
                assert_eq!(line, 4, "\"Page two\" is on the fourth source line");
            }
            other => panic!("expected Jump::File, got {other:?}"),
        }
    }
}
