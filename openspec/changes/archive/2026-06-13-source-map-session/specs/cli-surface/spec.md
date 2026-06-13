## ADDED Requirements

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
