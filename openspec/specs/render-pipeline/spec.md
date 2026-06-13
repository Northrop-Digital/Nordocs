## ADDED Requirements

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
The system SHALL compile Typst source to PDF using the embedded `typst` crate directly, without invoking any external `typst` binary.

#### Scenario: PDF produced without external process
- **WHEN** a render or build command is executed
- **THEN** a PDF is produced by the in-process embedded compiler with no child process spawned for Typst

#### Scenario: Embedded default fonts
- **WHEN** no system fonts are available at runtime
- **THEN** the compiler uses fonts embedded via `typst-assets` and `typst-kit`, producing valid PDF output

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
