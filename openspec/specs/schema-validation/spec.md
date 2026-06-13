## ADDED Requirements

### Requirement: Document structure validation command
The system SHALL provide a `validate` command that checks a document's structure against its schema and reports all violations.

#### Scenario: Valid document passes validation
- **WHEN** the user runs `ndoc validate <document>` on a document that conforms to its schema
- **THEN** the command exits with code 0 and reports no errors

#### Scenario: Invalid document fails validation with errors
- **WHEN** the user runs `ndoc validate <document>` on a document with schema violations
- **THEN** the command exits with a non-zero code and reports each violation with its location and description

#### Scenario: Missing required field reported
- **WHEN** a document is missing a field marked as required in its schema
- **THEN** the validate command reports the missing field by name

### Requirement: JSON output envelope
The system SHALL support a `--json` flag on all commands that emits machine-readable output in a standardised JSON envelope.

#### Scenario: JSON envelope wraps success output
- **WHEN** any command is invoked with `--json` and succeeds
- **THEN** stdout is a JSON object with a `success: true` field and a `data` field containing the command result

#### Scenario: JSON envelope wraps error output
- **WHEN** any command is invoked with `--json` and fails
- **THEN** stdout is a JSON object with a `success: false` field and an `error` field containing the error message; no non-JSON output is written to stdout

#### Scenario: JSON flag does not affect exit codes
- **WHEN** a command is invoked with `--json`
- **THEN** exit codes follow the same convention as without `--json` (0 for success, non-zero for failure)

### Requirement: Schema loading and error identification
The system SHALL load schemas from the catalogue and identify which schema a document is validated against, producing errors that reference the schema by name.

#### Scenario: Schema identified in error output
- **WHEN** validation fails against a named schema
- **THEN** the error output includes the schema name so the user knows which schema to inspect

#### Scenario: Unknown schema reference fails validation
- **WHEN** a document references a schema that does not exist in the catalogue
- **THEN** the validate command reports a schema-not-found error before attempting field-level validation

### Requirement: Pre-render validation
The system SHALL validate document structure before invoking the render pipeline, preventing compilation of structurally invalid documents.

#### Scenario: Invalid document is rejected before compilation
- **WHEN** the user runs `ndoc render` on a document with schema violations
- **THEN** the render command fails with validation errors and does not invoke the Typst compiler

#### Scenario: Valid document proceeds to compilation
- **WHEN** the user runs `ndoc render` on a document that passes validation
- **THEN** the render command proceeds to the Typst compilation step
