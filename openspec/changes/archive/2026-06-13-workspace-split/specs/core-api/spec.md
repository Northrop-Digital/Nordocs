## ADDED Requirements

### Requirement: Binding-agnostic service façade
The `nordocs-core` crate SHALL expose every document operation (render, build, validate, authoring, introspection) as a callable function that returns structured data, with no terminal output, process exit, or external-viewer side effects, so that any front end (CLI, FFI, WASM) can invoke the same logic.

#### Scenario: Core operation returns structured result
- **WHEN** a front end calls a `nordocs-core` operation with valid inputs
- **THEN** the function returns a serialisable result value (or a typed error) and writes nothing to stdout or stderr

#### Scenario: Core operation never terminates the process
- **WHEN** a `nordocs-core` operation encounters an error
- **THEN** it returns a typed `Error` value and does not call `std::process::exit` or panic across the API boundary

#### Scenario: No engine logic depends on the CLI
- **WHEN** `nordocs-core` is compiled as a dependency
- **THEN** it builds without `clap` or any terminal/argument-parsing dependency

### Requirement: Content and path forms for compilation
The façade SHALL provide both a content-based compile entry point (taking source text and resolved inputs) and a path-based entry point (resolving relative resources against the file's directory), mirroring the reference `CompileToPdf` / `CompileFileToPdf` split.

#### Scenario: Compile from in-memory content
- **WHEN** a caller invokes the content form with Typst source text
- **THEN** the document is compiled without any filesystem path being required

#### Scenario: Compile from a file path
- **WHEN** a caller invokes the path form with a `.typ` file path
- **THEN** relative resource references (e.g. `image()`) resolve against the file's directory

### Requirement: CLI is a thin adapter over the façade
The `ndoc` binary SHALL perform only input gathering (file/stdin reads, argument parsing) and output presentation (human-readable text, `--json` envelope, exit codes), delegating all engine work to `nordocs-core`.

#### Scenario: Command behaviour is preserved after extraction
- **WHEN** any `ndoc` subcommand is run after the façade extraction
- **THEN** its output, default paths, exit codes, and `--json` envelope are identical to the pre-refactor behaviour
