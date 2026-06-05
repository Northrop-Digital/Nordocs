# Project Charter: northdoc
**Version**: 1.0.0 | **Status**: Active | **Created**: 2026-06-04T06:50:43Z

## Vision
northdoc is a Rust-native re-implementation of the existing C# Typst document toolset that embeds Typst directly to deliver fast, single-binary document generation with a cleaner, refined CLI surface.

## Problem & Context
The current C# toolset has three core limitations:

1. **External process dependency** — it shells out to the Typst binary as an external process, which is slow, fragile, and hard to distribute. Rust can instead embed the native `typst` crate directly.
2. **Awkward tool/CLI surface** — the design grew organically and carries cruft and inconsistent commands worth redesigning.
3. **Startup/perf cost** — .NET startup and per-invocation overhead hurts short-lived CLI and agent-tool calls, whereas Rust starts and runs faster.

The goal is a port-and-refine: re-implement the core functionality in Rust while dropping cruft and improving where the C# design was awkward — solving these now rather than continuing to refactor C# around an external Typst process. The reference C# implementation lives at `.reference/Typst`.

## Target Users
Two primary user groups:

- **Human developers** who use the CLI directly to produce documents (the v1 focus).
- **AI agents** that invoke the tooling programmatically via an MCP-style AgentTools surface (this programmatic surface is deferred beyond v1).

Their core jobs are: generate documents from data and Markdown (including Markdown→Typst and rendering to PDF/output); manage a reusable library of components and templates across documents; and validate document structure and preview/render before producing the final build.

## Business Rationale
For developers and agents generating Typst-based documents, northdoc delivers correct end-to-end document output with at least the fidelity of the C# tool, while removing the friction of the old design. By embedding the native Typst crate instead of shelling out to an external process, it ships as a single binary, starts and renders faster, and exposes a cleaner, deliberately redesigned CLI that drops accumulated cruft — a clear improvement over the awkward, externally-dependent C# toolset it replaces.

## Scope Guardrails
### Will Do
- Provide a Markdown/data → Typst → PDF generation pipeline as the core capability
- Provide a component/template library to create, edit, store, and reuse document building blocks
- Provide document validation plus preview/render before final build
- Embed the native Typst crate (no external Typst process)
- Ship as a single binary with a refined, CLI-first surface

### Won't Do
- Be a drop-in replacement for the C# tool; it is a clean redesign and existing users migrate to the improved CLI/tool design
- Provide 1:1 parity with every legacy C# command
- Ship a GUI
- Support non-PDF output formats in v1
- Include a plugin system in v1
- Ship the AgentTools (MCP) programmatic surface in v1 (deferred to a later release)

## Success Criteria
The defining success measure for v1 is **correct end-to-end output**: a target set of documents must render correctly through the Markdown/data → Typst → PDF pipeline with fidelity at least equal to the C# tool. Single-binary distribution, faster startup/render, and a cleaner CLI are desirable but secondary to correctness.

**Failure signals:**
1. Embedding the native Typst crate proves impractical and forces a fallback to shelling out.
2. Output fidelity regresses versus the C# tool.
3. The redesigned CLI ends up harder to use than what it replaced.

## Testing Guidelines
All contributors and agents must follow these testing requirements. These apply to every PR and every feature branch.

### Mandatory rules
- `cargo test` must pass with zero failures before any commit lands.
- `cargo clippy --all-targets` must report zero warnings before any commit lands.
- No test may invoke an external `typst` binary — all rendering must go through the embedded compiler (`typst_world.rs` + `compiler.rs`).
- `unwrap()` and `expect()` are forbidden outside `#[cfg(test)]` blocks in library code (i.e. everything under `src/` except test modules). Use `anyhow`/`thiserror` error propagation instead.

### Test layer expectations

| Layer | Tool | When required |
|-------|------|---------------|
| Unit tests | `cargo test` (`#[cfg(test)]` in-module) | Every public function in `src/` modules |
| Snapshot tests | `insta` | Any Markdown-to-Typst or fat-file composed `.typ` output |
| CLI E2E tests | `assert_cmd` + `predicates` | Every public `ndoc` command |
| Release smoke test | `cargo build --release` + `assert_cmd` against `target/release/ndoc` | Added in P4; required before v1 tag |

### Coverage target
**Minimum: 80% line coverage**, enforced by `cargo tarpaulin` (config: `tarpaulin.toml`, output: HTML + Lcov). Install with `cargo install cargo-tarpaulin`. All public functions exposed via `lib.rs` modules must have at least one unit test. Snapshot tests must cover the happy path for every Markdown feature (tables, task lists, footnotes, strikethrough) that `markdown.rs` handles.

### Snapshot review
When `insta` snapshots change, review the diff carefully before accepting. A snapshot change is a signal to check whether the Typst output change is intentional. Never accept snapshot updates blindly.

## Delivery Phase Plan

**Current Phase Plan**: [charter-phase-plan.md](./charter-phase-plan.md)
**Last Updated**: 2026-06-04

### Phases
- [P1: Core Render Pipeline](./charter-phase-plan.md#p1-core-render-pipeline)
- [P2: Fat-File Authoring](./charter-phase-plan.md#p2-fat-file-authoring)
- [P3: Schema Validation and Preview](./charter-phase-plan.md#p3-schema-validation-and-preview)
- [P4: CLI Polish, Test Hardening, and Distribution](./charter-phase-plan.md#p4-cli-polish-test-hardening-and-distribution)
