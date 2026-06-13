# render-pipeline Specification

## Purpose
The embedded Typst render pipeline: Markdown/data converted to Typst and compiled in-process to PDF, SVG, and PNG over a retained `CompiledDoc`, including `sys.inputs` injection. No external `typst` binary is spawned.
## Requirements
### Requirement: Markdown to Typst conversion
The system SHALL convert Markdown source (CommonMark + GitHub Flavored Markdown extensions) into valid Typst markup as an intermediate representation before compilation.

#### Scenario: GFM table renders to Typst
- **WHEN** a Markdown document contains a GFM table
- **THEN** the output Typst file contains a correctly structured Typst table expression

#### Scenario: Task list renders to Typst
- **WHEN** a Markdown document contains a GFM task list (`- [ ]` / `- [x]`)
- **THEN** the output Typst file represents each list item with its checked/unchecked state

#### Scenario: Strikethrough renders to Typst
- **WHEN** a Markdown document contains `~~strikethrough~~` text
- **THEN** the output Typst file applies the appropriate strikethrough decoration

#### Scenario: Footnotes render to Typst
- **WHEN** a Markdown document contains GFM footnotes
- **THEN** the output Typst file contains the footnote definitions in Typst syntax

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

### Requirement: Single-pass render command
The system SHALL provide a command that accepts a document source and produces a PDF output in a single invocation.

#### Scenario: Render succeeds with valid input
- **WHEN** the user runs `ndoc render <source>` with a valid document
- **THEN** a PDF is written to the default output path and the process exits with code 0

#### Scenario: Render fails with informative error
- **WHEN** the user runs `ndoc render <source>` and the source is invalid
- **THEN** the process exits with a non-zero code and prints a human-readable error message describing the failure

### Requirement: Build command for project-level output
The system SHALL provide a `build` command that processes all documents in a project and writes PDF artefacts to the configured output directory.

#### Scenario: Build succeeds for multi-document project
- **WHEN** the user runs `ndoc build` in a project directory containing multiple documents
- **THEN** a PDF is produced for each document and all exit with code 0

#### Scenario: Build fails fast on first compile error
- **WHEN** one document in the project fails to compile
- **THEN** the build reports the error for that document and exits with a non-zero code

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

