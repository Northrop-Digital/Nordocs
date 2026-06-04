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
