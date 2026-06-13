## Why

The project previously tracked context in the RP1 system (charter.md, preferences.md); migrating to OpenSpec makes specifications discoverable, dependency-aware, and ready for the change-driven workflow. Since `openspec/specs/` is currently empty, this bootstraps the full capability catalogue from the RP1 charter and preferences in one step.

## What Changes

- Create OpenSpec spec documents for each core capability identified in the RP1 charter.
- Update `openspec/config.yaml` with project context (tech stack, conventions) drawn from the RP1 preferences.
- No code changes; this is a documentation/process migration only.

## Capabilities

### New Capabilities

- `render-pipeline`: Markdown/data → Typst → PDF generation pipeline, including embedded compiler world and the core `ndoc render`/`build` commands.
- `fat-file-model`: The `.ndoc.typ` composed fat-file format — structure, sections, hash guard, compose/extract operations.
- `component-library`: Schema-backed component and template catalogue — CRUD, validation, and reuse across documents.
- `schema-validation`: Document structure validation (`ndoc validate`) including schema loading, error reporting, and the JSON `--json` envelope.
- `cli-surface`: The complete `ndoc` command surface — all subcommands, flags, default output paths, exit codes, and `--json` output contract.

### Modified Capabilities

<!-- None — specs/ is empty; every capability above is net-new. -->

## Impact

- `openspec/config.yaml`: `context` section updated with tech stack and conventions.
- `openspec/specs/`: Five new spec directories created, each with `spec.md`.
- No source code, tests, or build artefacts are affected.
