## MODIFIED Requirements

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

## ADDED Requirements

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
