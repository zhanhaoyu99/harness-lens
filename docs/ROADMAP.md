# Roadmap

Harness Lens follows an evidence-first roadmap. Dates are intentionally omitted until an issue has an owner and a validated implementation plan.

## Product contract

The core model is:

```text
Defined -> Resolved -> Observed -> Evaluated
```

Each transition requires stronger evidence. A later stage must never be inferred only from an earlier one.

## v0.1 — Local inventory and Codex flight recorder

- [x] Local, read-only discovery for Codex and Claude Harness sources.
- [x] Map, List, Inspector, Overview, and aggregate-only Share views.
- [x] Resolution and duplicate metadata for discovered artifacts.
- [x] Best-effort redaction before content enters frontend previews.
- [x] Experimental Codex App Server connection for current skills/hooks and workspace threads.
- [x] Metadata-only linear replay of turns and item types.
- [x] macOS arm64 source build and ad-hoc-signed package.

Known evidence gap: current runtime declarations and historical thread activity can be inspected, but the current release cannot prove that the current declarations are the exact ones used by an older thread.

## v0.2 — Memory and project scope

- [x] On-demand viewing and narrow, explicit editing of existing recognized Memory Markdown files.
- [x] Separate project, nested-project, project-bound, and user-global scope labels.
- [x] Scan project and nested-project Harness sources along the active workspace ancestor chain.
- [x] Present same-name cross-tool content differences as informational diagnostics rather than resolution states.

## v0.3 — Provider lens and compact-window usability

- [x] Filter the inventory directly by Codex, Claude, shared, or another discovered provider without changing evidence semantics.
- [x] Keep provider, scope, type, search, List, Map, result counts, and warning navigation consistent.
- [x] Keep every page and sidebar destination reachable at the minimum supported window size.

## M2 — Reproducible context snapshots

- Persist immutable, content-addressed Harness snapshots locally.
- Bind new runs to the effective snapshot captured at execution time.
- Record adapter and CLI compatibility versions.
- Explain additions, removals, shadowing, duplicates, and resolution changes between snapshots.
- Add a privacy-reviewed export format with explicit schema versioning.

Exit criterion: a user can explain the exact known context for a newly captured run without relying on mutable current files.

## M3 — Verifier-driven evaluation

- Attach tests, simulator/device state, or other verifier evidence to a run.
- Keep runtime completion status separate from evaluation outcome.
- Model failure categories and evidence provenance.
- Compare success rate, duration, and failure modes across Harness revisions.
- Show token/cost metrics only when the provider supplies complete, attributable data.

Exit criterion: an evaluation result links to independent, inspectable evidence and cannot be produced by run completion alone.

## M4 — Safe collaboration

- Previewable redacted PNG and static replay exports.
- Portable bundles that exclude raw prompts and secrets by default.
- Cross-machine comparison without requiring a Harness Lens cloud account.
- Optional, separately designed team features only after privacy review.

## Distribution and ecosystem work

- Apple Developer ID signing and notarization.
- Intel macOS or universal binaries based on demonstrated demand.
- Provider fixture suite and compatibility matrix.
- Documented extension contract for new runtime adapters.
- Real issue-to-fix-to-release maintenance loops and public release notes.

## Explicit non-goals for the current roadmap

- Executing arbitrary agent workflows from the desktop app.
- Editing Rules, Skills, Hooks, Agents, Config, Workflows, generated Memory, or arbitrary files. Only the narrow existing-Memory-Markdown editor is in scope.
- Claiming semantic equivalence between Codex and Claude Harness concepts.
- Scoring task quality without an independent verifier.
- Uploading private run content to a hosted service by default.

Roadmap proposals should be opened as issues with a user problem, evidence boundary, privacy impact, and test plan. See [CONTRIBUTING.md](../CONTRIBUTING.md).
