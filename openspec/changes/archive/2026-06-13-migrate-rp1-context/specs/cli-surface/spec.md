## ADDED Requirements

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
The system SHALL write rendered output to a deterministic default path when no output path is explicitly specified.

#### Scenario: Default PDF path used when not specified
- **WHEN** the user runs `ndoc render <source>` without an `--output` flag
- **THEN** the PDF is written to the default output path derived from the source file name

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
