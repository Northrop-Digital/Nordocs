## ADDED Requirements

### Requirement: Native compile binding
The system SHALL expose the engine's compilation operations over a C-ABI / .NET binding, including PDF compilation from source content and from a file path, with optional `sys.inputs` variables.

#### Scenario: Compile source to PDF across the binding
- **WHEN** a .NET caller invokes the PDF compile function with valid Typst source
- **THEN** PDF bytes are returned across the boundary

#### Scenario: Compile a file to PDF across the binding
- **WHEN** a .NET caller invokes the file-based PDF compile function with a `.typ` path
- **THEN** relative resources resolve against the file's directory and PDF bytes are returned

### Requirement: Multi-format compile result
The binding SHALL expose a multi-format compile function returning a result that carries the output format and one buffer per page (a single buffer for PDF or merged output), matching the multi-buffer shape the reference consumer expects.

#### Scenario: SVG compile returns one buffer per page
- **WHEN** a .NET caller compiles an N-page document to SVG
- **THEN** the result reports the SVG format and contains N buffers in page order

#### Scenario: PDF compile returns a single buffer
- **WHEN** a .NET caller compiles a document to PDF
- **THEN** the result reports the PDF format and contains exactly one buffer

### Requirement: Markdown, preview, authoring, and validation bindings
The binding SHALL expose markdown-to-Typst conversion, component and document preview rendering, document authoring operations, and validation, returning the same structured results the engine façade provides.

#### Scenario: Markdown conversion across the binding
- **WHEN** a .NET caller converts Markdown to Typst through the binding
- **THEN** the equivalent Typst markup string is returned

#### Scenario: Validation across the binding
- **WHEN** a .NET caller validates a document through the binding
- **THEN** the structured validation result (violations and summary) is returned

### Requirement: Source-map session handle
The binding SHALL expose a compiled document as an opaque session handle supporting per-page SVG and PNG export, click-to-source and cursor-to-preview mapping, and page geometry queries, with an explicit release operation.

#### Scenario: Session export and jump
- **WHEN** a .NET caller creates a session, exports a page to SVG, and queries click-to-source for a point
- **THEN** the SVG bytes and the resolved source location are returned from the same session without recompilation

#### Scenario: Session release
- **WHEN** a .NET caller releases a session handle
- **THEN** the underlying resources are freed and the handle is no longer used

### Requirement: Safe error marshalling
The binding SHALL NOT allow a Rust panic to unwind across the FFI boundary; every fallible call SHALL return a structured error (code and message) that the .NET wrapper surfaces as an exception.

#### Scenario: Compile error returns a structured error
- **WHEN** a .NET caller compiles invalid Typst source
- **THEN** a structured error with a non-empty message is returned and the process does not crash

#### Scenario: Panic is contained
- **WHEN** an engine operation panics internally
- **THEN** the panic is caught at the boundary and reported as a structured error rather than unwinding into the .NET runtime

### Requirement: Generated binding parity and freshness
The system SHALL generate the .NET binding from the Rust surface, keep the generated binding under version control, and verify in CI that it is regenerated without differences; the binding SHALL cover the reference compiler, markdown, and preview operations.

#### Scenario: Stale binding fails CI
- **WHEN** CI regenerates the binding and the result differs from the committed copy
- **THEN** the CI step fails

#### Scenario: Reference operations are covered
- **WHEN** the parity check enumerates the reference compiler, markdown, and preview operations
- **THEN** each has a corresponding function in the generated binding
