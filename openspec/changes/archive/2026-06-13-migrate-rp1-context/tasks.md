## 1. Update Project Configuration

- [x] 1.1 Write `context` block into `openspec/config.yaml` covering the tech stack (Rust 2021, clap 4, typst 0.14, comrak 0.30, serde/anyhow/thiserror) and key conventions (embedded Typst compiler, single binary, no `unwrap` outside `#[cfg(test)]`)
- [x] 1.2 Verify `openspec/config.yaml` is valid YAML and the `schema: spec-driven` field is preserved

## 2. Verify Spec Completeness

- [x] 2.1 Review `specs/render-pipeline/spec.md` — confirm all Markdown GFM extensions (tables, task lists, strikethrough, footnotes) are covered by requirements
- [x] 2.2 Review `specs/fat-file-model/spec.md` — confirm compose, extract, and hash guard requirements match the fat-file implementation
- [x] 2.3 Review `specs/component-library/spec.md` — confirm component and template CRUD requirements cover the `ndoc component` and `ndoc template` command groups
- [x] 2.4 Review `specs/schema-validation/spec.md` — confirm JSON envelope contract and pre-render validation requirements are accurate
- [x] 2.5 Review `specs/cli-surface/spec.md` — confirm all `ndoc` subcommands and flags listed in `README.md` are represented

## 3. Sync Specs to Main Spec Directory

- [x] 3.1 Run `openspec sync-specs --change migrate-rp1-context` (or archive) to promote the five spec files from the change directory to `openspec/specs/`
- [x] 3.2 Confirm `openspec/specs/` contains the five capability directories: `render-pipeline`, `fat-file-model`, `component-library`, `schema-validation`, `cli-surface`
