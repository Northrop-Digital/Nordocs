## Context

The nordocs project previously used RP1 as its context-tracking system. The RP1 database holds two files:
- `.rp1/context/charter.md` — vision, scope, success criteria, testing guidelines, phase plan
- `.rp1/context/preferences.md` — tech stack choices and project structure conventions

`openspec/specs/` is currently empty. This migration bootstraps it from the RP1 content, establishing the baseline capability catalogue against which future changes will be authored. No code changes are involved.

## Goals / Non-Goals

**Goals:**
- Populate `openspec/specs/` with one spec per core capability identified in the RP1 charter.
- Update `openspec/config.yaml` context with tech stack and conventions from RP1 preferences.
- Each spec is written at a level of detail sufficient to validate implementation and drive future delta specs.

**Non-Goals:**
- Exhaustively document every implementation detail (that belongs in code comments and AGENTS.md).
- Replicate the full RP1 charter prose verbatim — specs should be normative requirements, not narrative.
- Change any code, tests, or tooling.

## Decisions

### Capability boundaries from RP1 charter

The RP1 charter identifies four functional areas (render pipeline, fat-file format, component/template library, validation) plus the CLI surface. Each becomes one spec, keeping concerns cleanly separated. An alternative would be a single monolithic spec, but separate specs allow fine-grained change tracking per capability.

### Spec granularity: requirements over prose

Requirements are written as normative SHALL/MUST statements with WHEN/THEN scenarios rather than narrative prose. This makes each scenario a directly traceable test case, which aligns with the charter's 80% coverage requirement. RP1 uses prose; OpenSpec requires normative form — the translation is intentional.

### config.yaml context section

Tech stack details (Rust 2021, clap 4, typst 0.14, comrak 0.30, etc.) and key conventions (no external typst process, single binary, unwrap-free library code) are written into `openspec/config.yaml`'s `context` field. This gives all future artifact generation the right background without requiring agents to re-read the RP1 files.

### RP1 files retained, not deleted

The `.rp1/` directory is left in place. Removing it is a separate decision for the project owner. This migration is additive only.

## Risks / Trade-offs

- **Spec completeness** → The RP1 charter is high-level; requirements may not capture every edge case. Mitigation: write specs at the level the charter explicitly defines; gaps are filled by future changes as implementation reveals them.
- **Spec staleness** → If the RP1 files were updated recently, specs could lag. Mitigation: RP1 files were authored at project start and the codebase is now in a stable state (P4 complete); the risk is low.
- **config.yaml overwrite** → Writing context to config.yaml replaces the placeholder comment. The existing schema/format is preserved; only the `context` value is added.
