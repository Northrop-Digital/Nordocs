## ADDED Requirements

### Requirement: Fat-file format structure
The system SHALL use `.ndoc.typ` as the canonical fat-file format that composes all document sections — metadata, Markdown source, data, and Typst template — into a single file with delimited sections.

#### Scenario: Fat file contains all required sections
- **WHEN** a fat file is composed from a valid document
- **THEN** the resulting `.ndoc.typ` file contains each section (metadata, content, data, template) separated by the section delimiter

#### Scenario: Section order is stable
- **WHEN** the same document is composed multiple times
- **THEN** the section order in the `.ndoc.typ` output is identical across invocations

### Requirement: Hash guard integrity check
The system SHALL embed a hash guard in each fat file that detects external modification of the file contents.

#### Scenario: Unmodified fat file passes integrity check
- **WHEN** a fat file is read back immediately after being composed
- **THEN** the hash guard verification succeeds

#### Scenario: Externally modified fat file fails integrity check
- **WHEN** a fat file's content is modified outside of the ndoc tooling
- **THEN** the hash guard verification fails and an error is reported

### Requirement: Compose operation
The system SHALL provide a compose operation that assembles source materials (Markdown, data, Typst template) into a `.ndoc.typ` fat file.

#### Scenario: Compose produces valid fat file
- **WHEN** compose is invoked with valid source materials
- **THEN** a `.ndoc.typ` file is written at the specified output path

#### Scenario: Compose overwrites existing fat file
- **WHEN** a `.ndoc.typ` file already exists at the output path
- **THEN** compose replaces it with the newly composed content

### Requirement: Extract operation
The system SHALL provide an extract operation that splits a `.ndoc.typ` fat file back into its constituent source materials.

#### Scenario: Extract recovers original sections
- **WHEN** extract is invoked on a fat file that was previously composed
- **THEN** the extracted sections match the original source materials used during composition
