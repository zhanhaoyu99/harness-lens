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

## v0.4 — Local snapshot history and Saved-to-Saved compare

- [x] Keep workspace selection and ordinary Rescan operations live-only; they must not write snapshot history.
- [x] Provide an explicit Capture action whose backend performs a fresh scan and atomically persists a capture referencing an immutable, content-addressed, metadata-only Harness snapshot.
- [x] Keep capture history isolated by workspace and retain the latest 50 explicit captures for each workspace.
- [x] Provide an explicitly confirmed action for clearing the selected workspace's saved history.
- [x] Record explicit local schema, Harness Lens, and scanner compatibility versions with saved snapshot evidence; reserve runtime-adapter versioning for later run binding.
- [x] Compare two saved snapshots from the same workspace for observed additions, removals, content-hash changes, resolution changes, and duplicate or same-name diagnostic changes.
- [x] Preserve incomplete-scan state in history and qualify absence-based comparison claims when either side is incomplete.
- [x] Exclude Harness content and previews, raw Memory text, absolute paths, and raw runtime data from persisted snapshot history.

## v0.5 — Adoption evidence and safe compatibility feedback

- [x] Add one versioned Rust projection for aggregate compatibility reports, with a published JSON Schema and source-attribution boundary.
- [x] Let the desktop Share flow fresh-scan saved disk state, preview the complete report, and copy only after explicit review without discarding unsaved Memory drafts.
- [x] Keep the browser demo's report synthetic and clearly outside real compatibility evidence.
- [ ] Validate the workflow with 5–8 independent Codex or Claude users and turn verified feedback into compatibility issues or an issue-to-fix-to-release loop.

Exit criterion: after a workspace changes, a user can reopen Harness Lens and explain the saved metadata differences between two explicit capture points without relying on mutable current files.

Evidence boundary: v0.4 compares Saved-to-Saved Harness context only. It does not bind a run to either snapshot, compare outcomes, or infer that the nearest capture was active for a run.

## M2 continuation — Reproducible run context

- Bind new runs only to an effective snapshot captured through an adapter-backed execution-time boundary.
- Record the provider CLI/runtime compatibility version at that binding point.
- Explain the defined graph versus observed path without deriving a graph from a linear trace.
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
