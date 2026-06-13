## ADDED Requirements

### Requirement: Component creation
The system SHALL allow users to create named reusable components stored in a project-level catalogue.

#### Scenario: Component created successfully
- **WHEN** the user runs `ndoc component add <name>` with valid arguments
- **THEN** a new component entry is created in the catalogue and the process exits with code 0

#### Scenario: Duplicate component name is rejected
- **WHEN** the user attempts to create a component with a name that already exists in the catalogue
- **THEN** the command fails with a descriptive error and no catalogue entry is modified

### Requirement: Template creation
The system SHALL allow users to create named reusable Typst templates stored in the project-level catalogue.

#### Scenario: Template created successfully
- **WHEN** the user runs `ndoc template add <name>` with valid arguments
- **THEN** a new template entry is created in the catalogue and the process exits with code 0

#### Scenario: Duplicate template name is rejected
- **WHEN** the user attempts to create a template with a name that already exists in the catalogue
- **THEN** the command fails with a descriptive error and no catalogue entry is modified

### Requirement: Component and template editing
The system SHALL allow users to edit existing components and templates in the catalogue.

#### Scenario: Edit updates catalogue entry
- **WHEN** the user runs `ndoc component edit <name>` or `ndoc template edit <name>` on an existing entry
- **THEN** the catalogue entry is updated with the new content and the process exits with code 0

#### Scenario: Edit on non-existent entry fails
- **WHEN** the user attempts to edit a component or template that does not exist
- **THEN** the command fails with a descriptive error

### Requirement: Component and template reuse across documents
The system SHALL allow components and templates from the catalogue to be referenced from any document in the project.

#### Scenario: Document references a catalogue component
- **WHEN** a document references a named component from the catalogue
- **THEN** the render pipeline resolves the component and includes it in the compiled output

#### Scenario: Document references non-existent component fails gracefully
- **WHEN** a document references a component name that is not in the catalogue
- **THEN** the render pipeline reports a descriptive error identifying the missing component

### Requirement: Schema-backed catalogue validation
The system SHALL validate component and template entries against a defined schema at create/edit time.

#### Scenario: Valid component passes schema validation
- **WHEN** a component is created or edited with content that satisfies the schema
- **THEN** the operation succeeds

#### Scenario: Invalid component fails schema validation
- **WHEN** a component is created or edited with content that violates the schema
- **THEN** the operation fails with a descriptive validation error before writing to the catalogue
