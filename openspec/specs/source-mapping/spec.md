# source-mapping Specification

## Purpose
Bidirectional mapping between a rendered Typst document and its source over a retained compiled session: click-to-source (`jump_from_click`) and cursor-to-preview (`jump_from_cursor`), plus page geometry, exposed as serialisable results for downstream renderers.
## Requirements
### Requirement: Click-to-source mapping
The system SHALL map a click at a page-local point on a compiled document back to the source location that produced the clicked content, returning the source file, byte offset, and 1-based line and column.

#### Scenario: Click resolves to source location
- **WHEN** a caller invokes the click-to-source map with a page index and a point over rendered content
- **THEN** the result identifies the source file and the byte offset, line, and column of the originating Typst code

#### Scenario: Click over a hyperlink resolves to the URL
- **WHEN** a caller clicks content that is a hyperlink
- **THEN** the result is the target URL rather than a source location

#### Scenario: Click over empty space yields no target
- **WHEN** a caller clicks a point with no associated content
- **THEN** the result indicates that there is no jump target

### Requirement: Cursor-to-preview mapping
The system SHALL map a source position (file and byte offset) to the set of on-page positions in the compiled document where that source produces output, for preview highlighting.

#### Scenario: Source cursor resolves to page positions
- **WHEN** a caller invokes the cursor-to-preview map for a source offset that produces visible output
- **THEN** the result is a non-empty set of page positions, each with a page index and a point

#### Scenario: Source cursor with no output yields an empty set
- **WHEN** a caller invokes the cursor-to-preview map for a source offset that produces no visible output
- **THEN** the result is an empty set of positions

### Requirement: Page geometry for coordinate transform
The system SHALL expose the page count and each page's size in points so that a downstream renderer can convert UI pixel coordinates to the page-local points the click-to-source map requires.

#### Scenario: Page sizes reported in points
- **WHEN** a caller queries the geometry of a compiled document
- **THEN** the page count and each page's width and height in points are returned

### Requirement: Retained compiled session
The system SHALL allow a compiled document and its world to be retained so that multiple export and mapping operations can be performed without recompiling.

#### Scenario: Multiple operations on one compilation
- **WHEN** a caller compiles a document once and then performs an export and a click-to-source query
- **THEN** both operations use the same retained compilation without a second compile pass

