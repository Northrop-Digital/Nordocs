## MODIFIED Requirements

### Requirement: Embedded Typst compilation
The system SHALL compile Typst source using the embedded `typst` crate directly, without invoking any external `typst` binary, retaining the compiled `PagedDocument` so it can be exported to PDF, SVG, or PNG.

#### Scenario: Document produced without external process
- **WHEN** a render or build command is executed
- **THEN** the document is produced by the in-process embedded compiler with no child process spawned for Typst

#### Scenario: Embedded default fonts
- **WHEN** no system fonts are available at runtime
- **THEN** the compiler uses fonts embedded via `typst-assets` and `typst-kit`, producing valid output

#### Scenario: PDF output is unchanged
- **WHEN** a document is exported to PDF after the multi-format refactor
- **THEN** the PDF bytes are identical to those produced before the refactor for the same input

## ADDED Requirements

### Requirement: SVG export
The system SHALL export a compiled document to SVG, producing one SVG per page by default or a single merged SVG canvas on request.

#### Scenario: Per-page SVG produced
- **WHEN** a caller exports an N-page document to SVG without merging
- **THEN** N SVG documents are produced, one per page, in page order

#### Scenario: Merged SVG produced
- **WHEN** a caller exports a document to SVG with merging requested
- **THEN** a single SVG canvas containing all pages is produced

### Requirement: PNG export
The system SHALL export a compiled document to PNG raster images at a caller-specified resolution, producing one PNG per page by default or a single merged image on request.

#### Scenario: PNG produced at requested resolution
- **WHEN** a caller exports a page to PNG at a given DPI
- **THEN** a non-empty PNG is produced whose pixel dimensions correspond to the page size scaled by `dpi / 72`

#### Scenario: Default resolution applied
- **WHEN** a caller exports to PNG without specifying a resolution
- **THEN** a default of 144 DPI is used

### Requirement: System input variable injection
The system SHALL allow callers to inject `sys.inputs` key/value variables that are available to the document before compilation.

#### Scenario: Injected variable is readable in the document
- **WHEN** a document reads `sys.inputs.<key>` and the caller injected a value for `<key>`
- **THEN** the compiled output reflects the injected value

#### Scenario: No variables injected
- **WHEN** a caller compiles without injecting any variables
- **THEN** compilation succeeds and `sys.inputs` is empty
