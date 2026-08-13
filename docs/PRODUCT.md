# Harness Lens product direction

## Why this exists

When a developer maintains rules, skills, hooks, agents, memory and reusable loops across global and project scopes, the first problem is not orchestration. It is visibility: **what do I maintain, what is each item, and what is effective for this task?**

Harness Lens turns that personal pain into an Agent DevTools product. It is not another chat window and it does not execute arbitrary workflows in the MVP.

## Core questions

1. What exists in my Harness?
2. What is effective for this workspace, runtime and working directory?
3. What path did a real task actually take, and where is the evidence?
4. After changing the Harness, did success rate, cost, duration or failure mode improve?

## Product model

Every claim belongs to one of four stages:

| Stage | Meaning | Evidence |
|---|---|---|
| Defined | The item was discovered | File, runtime declaration or adapter result |
| Effective | The runtime resolves it in the current context | Precedence and trust rules from the runtime adapter |
| Observed | A run loaded or used it | Runtime events or trace data |
| Evaluated | The result was independently judged | Verifier, test, environment state or eval |

The UI must keep these states separate. A file being present is not proof that it was active or used.

Diagnostics are a separate axis. “Same name, different content” means two providers define the same kind and name in the same concrete user or project layer but their file hashes differ. It is a comparison hint, not a configuration error, historical drift, or resolution state.

## Current product journey

1. Choose a local workspace.
2. See a dynamic Harness map and searchable list.
3. Open any item to read its redacted content, scope, source and resolution reason; load Memory text only on explicit request.
4. Identify duplicate definitions, ambiguous names and unknown states.
5. Open the original file in the editor when a change is needed, or explicitly edit an eligible existing Memory Markdown file in place.
6. Connect to the local Codex App Server and inspect recent workspace runs.
7. Replay a selected run as a linear, metadata-only turn/item timeline.
8. Copy an aggregate-only, redacted snapshot for a discussion or progress update.

## v0.4 candidate journey

1. Choose or rescan a workspace to update the live view without writing history.
2. Explicitly choose Capture; the backend performs a fresh scan and atomically saves a local capture backed by an immutable, content-addressed, metadata-only snapshot.
3. Revisit the latest 50 explicit captures retained for that exact workspace after files have changed or the app has restarted.
4. Choose two saved snapshots from that workspace and inspect observed additions, removals, content-hash changes, resolution changes, and diagnostic changes.
5. Clear that workspace's local capture history only through an explicit confirmation.

The saved-history model records configuration evidence, not file backups. It excludes Harness content and previews, raw Memory text, absolute paths, and runtime payloads. A historical item cannot be opened or edited as if the current file were the saved revision.

## Information architecture

- **Overview**: map, counts, conflicts and recently changed items.
- **Items**: searchable inventory and inspector.
- **Runs**: experimental metadata-only Codex thread timeline and observed item types.
- **Compare**: Saved-to-Saved Harness revision differences in the v0.4 candidate; bound-run and outcome comparisons only after execution-time capture and verifier evidence exist.
- **Share**: aggregate-only redacted snapshot today; image and static replay bundles later.

The current public v0.3 release implements Overview, Items, the aggregate Share snapshot, and a read-only Codex flight recorder. The local v0.4 candidate activates Compare for two saved Harness snapshots only. Outcome comparison still requires bound runs plus verifier evidence and remains later work.

## Scope boundaries

### MVP

- macOS first and local-only; Harness sources remain read-only except for explicit edits to eligible existing Memory Markdown files. Explicit Capture and confirmed clear-history actions may write or delete metadata-only records in the app-managed data directory.
- Codex and Claude Harness discovery.
- Codex precedence backed by published runtime rules.
- Map/List exploration and content inspection.
- Content hashing and duplicate detection.
- Secret redaction by default.
- Chinese and English UI, following the system language by default.
- Headless workspace scan for non-intrusive validation.
- Aggregate-only Markdown sharing without file content or absolute paths.
- Experimental Codex App Server inspection of current skills/hooks and recent workspace threads.
- Linear, metadata-only run replay without raw prompts, reasoning, tool arguments or file diffs.
- On-demand Memory viewing plus confirmed, conflict-checked saves for a narrow Markdown allowlist.

### Next

- Explicit, fresh-scan metadata-only Harness capture with atomic persistence and a fixed latest-50-capture retention policy per workspace; ordinary live scans remain transient.
- Saved-to-Saved Harness comparison within one workspace, with incomplete-scan evidence kept explicit.
- Adapter-backed execution-time snapshot binding for newly observed runs after the v0.4 storage foundation is validated.
- Defined graph versus actual path, without inferring a graph from a linear trace.
- Evidence and verifier attachment.
- Two-run and outcome comparison across execution-time-bound snapshots.
- Redacted PNG/static HTML run sharing.

The current runtime view is explicitly limited: current runtime declarations cannot be presented as the historical effective Harness for an older thread, and completion status cannot be presented as task success. v0.4 does not change that boundary and must not associate a run with the nearest capture by timestamp.

### Explicitly later

- Visual workflow editing or orchestration.
- Cloud sync, teams, RBAC and comments.
- A marketplace or public gallery.
- Automatic prompt/skill optimization.
- Claiming cross-runtime semantic equivalence.

## Propagation loop

The sharing primitive is not a cloud workspace. It is a safe artifact:

- a redacted Harness Map image for chat or email;
- a redacted static Run Replay;
- an importable bundle with hashes and evidence metadata, excluding secrets by default.

This lets the product spread through useful explanations before it needs a hosted service.
