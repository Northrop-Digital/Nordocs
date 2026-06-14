# cli-surface Specification

## Purpose
The `ndoc` command-line surface: the top-level subcommands, their flags and default output paths, consistent exit codes, and the global `--json` envelope exposed by the `nordocs-cli` binary.
## Requirements
### Requirement: Core document commands
The system SHALL expose `render`, `build`, `validate`, and `preview` as top-level `ndoc` commands.

#### Scenario: render command accepted
- **WHEN** the user runs `ndoc render --help`
- **THEN** the command exits with code 0 and prints usage for the render subcommand

#### Scenario: build command accepted
- **WHEN** the user runs `ndoc build --help`
- **THEN** the command exits with code 0 and prints usage for the build subcommand

#### Scenario: validate command accepted
- **WHEN** the user runs `ndoc validate --help`
- **THEN** the command exits with code 0 and prints usage for the validate subcommand

#### Scenario: preview command accepted
- **WHEN** the user runs `ndoc preview --help`
- **THEN** the command exits with code 0 and prints usage for the preview subcommand

### Requirement: Authoring subcommands
The system SHALL expose `new`, `add`, and `edit` as subcommands under the `doc` authoring group.

#### Scenario: ndoc doc new creates a document
- **WHEN** the user runs `ndoc doc new <name>`
- **THEN** a new document scaffold is created in the current project

#### Scenario: ndoc doc add adds content
- **WHEN** the user runs `ndoc doc add <type> <name>`
- **THEN** the specified content type is added to the current document

#### Scenario: ndoc doc edit opens a document
- **WHEN** the user runs `ndoc doc edit <name>`
- **THEN** the document is opened for editing

### Requirement: Catalogue management subcommands
The system SHALL expose `component`, `item`, `template`, and `image` as top-level catalogue management command groups.

#### Scenario: component subgroup accepted
- **WHEN** the user runs `ndoc component --help`
- **THEN** the command exits with code 0 and lists available component subcommands

#### Scenario: template subgroup accepted
- **WHEN** the user runs `ndoc template --help`
- **THEN** the command exits with code 0 and lists available template subcommands

#### Scenario: item subgroup accepted
- **WHEN** the user runs `ndoc item --help`
- **THEN** the command exits with code 0 and lists available item subcommands

#### Scenario: image subgroup accepted
- **WHEN** the user runs `ndoc image --help`
- **THEN** the command exits with code 0 and lists available image subcommands

### Requirement: Default output paths
The system SHALL write rendered output to a deterministic default path when no output path is explicitly specified, with the file extension matching the selected output format.

#### Scenario: Default PDF path used when not specified
- **WHEN** the user runs `ndoc render <source>` without an `--output` flag and without a format override
- **THEN** the PDF is written to the default output path derived from the source file name with a `.pdf` extension

#### Scenario: Default path matches selected format
- **WHEN** the user runs `ndoc render <source> --format svg` without an `--output` flag
- **THEN** the output is written to the default path derived from the source file name with a `.svg` extension

#### Scenario: Multi-page output is suffixed per page
- **WHEN** a render to SVG or PNG produces multiple pages without merging
- **THEN** files are written as `<base>-1.<ext>`, `<base>-2.<ext>`, … and a single-page result is written as `<base>.<ext>` with no numeric suffix

### Requirement: Consistent exit codes
The system SHALL exit with code 0 on success and a non-zero code on any error, across all commands.

#### Scenario: Successful command exits 0
- **WHEN** any ndoc command completes without error
- **THEN** the process exit code is 0

#### Scenario: Failed command exits non-zero
- **WHEN** any ndoc command encounters an error
- **THEN** the process exit code is non-zero (≥ 1)

### Requirement: JSON output flag available on all commands
The system SHALL accept a `--json` flag on every command that produces output, emitting the JSON envelope defined in the schema-validation spec.

#### Scenario: --json flag accepted on render
- **WHEN** the user runs `ndoc render --json <source>`
- **THEN** all stdout output is valid JSON matching the envelope schema

#### Scenario: --json flag accepted on validate
- **WHEN** the user runs `ndoc validate --json <document>`
- **THEN** all stdout output is valid JSON matching the envelope schema

### Requirement: Output format selection
The `render` and `build` commands SHALL allow the output format (PDF, SVG, or PNG) to be selected by the `--output` file extension or by an explicit `--format` flag, with `--dpi` controlling PNG resolution and `--merged` selecting single-file multi-page output.

#### Scenario: Format inferred from output extension
- **WHEN** the user runs `ndoc render <source> -o out.svg`
- **THEN** SVG output is produced regardless of any default

#### Scenario: Format chosen by flag
- **WHEN** the user runs `ndoc render <source> --format png`
- **THEN** PNG output is produced

#### Scenario: Conflicting format selectors rejected
- **WHEN** the user runs `ndoc render <source> -o out.svg --format png`
- **THEN** the command exits with a non-zero code and reports the conflict

#### Scenario: PNG resolution controlled by dpi
- **WHEN** the user runs `ndoc render <source> --format png --dpi 300`
- **THEN** the PNG is rasterised at 300 DPI

#### Scenario: preview remains PDF-only
- **WHEN** the user runs `ndoc preview <source>`
- **THEN** a PDF preview is produced and no `--format` option is offered on `preview`

### Requirement: Jump diagnostic subcommand
The system SHALL expose a hidden `ndoc jump` subcommand that compiles a document and reports the source location for a click at a given page and point, as a diagnostic for the click-to-source capability. The command SHALL be invocable but SHALL NOT appear in the top-level `ndoc --help` listing.

#### Scenario: jump command is invocable
- **WHEN** the user runs `ndoc jump --help`
- **THEN** the command exits with code 0 and prints usage for the jump subcommand

#### Scenario: jump command is hidden from top-level help
- **WHEN** the user runs `ndoc --help`
- **THEN** the `jump` subcommand is not listed among the available commands

#### Scenario: jump reports a source location as JSON
- **WHEN** the user runs `ndoc jump <source> --page 1 --at <x>,<y> --json` for a point over rendered content
- **THEN** the JSON envelope contains the resolved source file, offset, line, and column

#### Scenario: jump over empty space reports no target
- **WHEN** the user runs `ndoc jump <source> --page 1 --at <x>,<y>` for a point with no content
- **THEN** the command reports that there is no jump target and exits with code 0

