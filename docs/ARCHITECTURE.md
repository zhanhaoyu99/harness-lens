# Architecture

## Decision: Tauri 2 desktop shell

The first version uses Tauri 2 with a React/TypeScript frontend and a Rust backend.

Reasons:

- The end state depends on interactive topology, timelines, diffs and shareable reports; the Web UI ecosystem fits those surfaces well.
- Tauri uses the system WebView and does not bundle Chromium, keeping the desktop app aligned with the lightweight positioning.
- Rust provides a narrow local boundary for filesystem scanning, app-managed persistence and Codex App Server stdio.
- React views and normalized schemas can later be reused for static share reports or another desktop platform.
- The app remains a normal desktop window; it does not require a local server or cloud account.

## Layers

```text
React UI
  -> Typed frontend client
    -> Narrow Tauri commands
      -> Normalized Rust model
        -> Provider adapters
          -> Local files / Codex App Server / future trace inputs
```

### Rust backend

- normalized entities and resolution states;
- local discovery and content loading;
- provider adapters;
- redaction, hashing and duplicate detection;
- experimental Codex App Server transport and run normalization;
- narrow Tauri command/capability boundary;
- v0.4: immutable metadata-only context snapshots, bounded local capture history, and Saved-to-Saved differences;
- later: execution-time run binding, evaluations and share bundles.

### React frontend

- workspace selection and recent workspace state;
- Overview Map/List;
- item inspector;
- aggregate Share snapshot;
- metadata-only Codex Runs and linear replay;
- v0.4: Saved-to-Saved Harness history and comparison;
- later: bound-run and verifier evidence views.

## Runtime integration boundary

The app does not treat Codex's on-disk rollout JSONL as a permanent public contract. The current Codex adapter uses the experimental App Server protocol over local stdio for current skills/hooks, workspace thread listing, and read-only stored thread reads. Responses are bounded and normalized through an explicit metadata allowlist; raw responses are not persisted or logged.

The flight recorder shows a linear sequence of turns and normalized item types. It excludes raw prompts, reasoning, tool arguments and file diffs. A completed turn is runtime activity, not verifier evidence or proof of success.

Historical thread reads do not include a trustworthy historical instruction-source snapshot. Therefore v0.4 does not claim that current skills/hooks are the exact Harness used by an older run. Binding an immutable snapshot at a provider-backed execution boundary remains an M2 continuation requirement.

v0.4 local capture history does not relax that boundary. A saved snapshot and a run may have nearby timestamps without proving that the snapshot was active when the run executed. Runs remain unbound until a provider adapter supplies a trustworthy execution-time capture point; the UI must not offer a “nearest snapshot” association as evidence.

Claude and other runtimes get separate adapters. Shared UI depends only on normalized entities.

## Security and privacy

- All scanning is opt-in through a chosen workspace plus documented user-level Harness locations.
- Repository discovery follows only the ancestor chain from the selected Git root to the selected workspace. This makes nested-project scope visible without scanning unrelated sibling projects.
- The app is read-only by default. A separate narrow command path can load an already-scanned Memory file on demand and save only eligible existing Markdown files after confirmation and revision checks.
- Files are size-limited before loading.
- Sensitive patterns are redacted on a best-effort basis before entering UI previews.
- Claude project memory is filtered to the selected workspace rather than scanning unrelated projects.
- Opening a source file goes through a backend allowlist populated by the current scan.
- Memory text is excluded from the regular snapshot and Share model. An explicit load uses an artifact identifier rather than a frontend-supplied path; an edit token binds a save to the scanned file revision.
- Memory saves reject stale revisions, symbolic-link or multiply hard-linked sources, files owned by another user, special permission bits, byte-order marks or non-LF line endings, and unsupported formats; use a same-directory atomic replacement; and never provide create, rename, delete, force-save, backup, or autosave operations.
- Session content and evidence are read only when the user opens a specific run.
- Runtime normalization keeps allowlisted metadata and excludes raw prompts, reasoning, tool arguments and diffs.
- The current Share snapshot contains aggregate counts only; it excludes content and absolute paths.
- The headless compatibility report has its own versioned aggregate projection and allowlist. It includes a validated Harness Lens source revision when available, but excludes workspace and artifact names, paths, branches, content, artifact/content hashes, sizes, timestamps, and diagnostic details; tests assert that representative private fixture values cannot cross that serializer.
- Future non-aggregate exports require a separate redaction pass and preview.
- Unknown runtime trust or precedence is shown as unknown rather than guessed.

## v0.4 snapshot persistence

Before v0.4, choosing a workspace and scanning remained entirely in memory. In v0.4, choosing a workspace and using Rescan remain transient live-scan operations: they update the current allowlists and UI but do not create history. Persistence is entered only through a separate, explicit Capture command. That backend command performs its own fresh scan and atomically writes the resulting metadata-only capture; it must not accept a frontend-supplied `HarnessSnapshot` as historical evidence. A fresh-scan or write failure creates no partial capture.

The v0.4 persistence model uses two separate concepts behind narrow Tauri commands:

- `ContextSnapshot`: an immutable, content-addressed payload derived from a deterministic ordering of normalized Harness item metadata and resolution evidence;
- `SnapshotCapture`: a workspace-scoped observation that records capture time, Git branch, completeness, schema version, app/scanner compatibility versions, and the referenced snapshot identifier. Runtime adapter compatibility belongs to a later execution-time binding record, not the v0.4 static capture.

Separating payload identity from capture time means two identical explicit captures can reference the same immutable snapshot without manufacturing a configuration change. Each explicit Capture remains a capture record for retention and audit purposes. Compare accepts two saved captures from the same workspace; it never compares an unsaved mutable frontend object with historical evidence.

The persisted projection is deliberately narrower than the live `HarnessSnapshot`. It can contain stable item identifiers, safe names and relative source labels, provider, kind, scope, content hash, size, resolution, and normalized diagnostic relationships. It must exclude:

- Harness file content and redacted previews;
- raw Memory text;
- absolute paths;
- prompts, reasoning, tool arguments, file diffs, and raw runtime responses.

Capture history is isolated by workspace and retains the latest 50 explicit captures for each workspace. Store access is serialized across cooperating application processes with a workspace-scoped advisory file lock, and the workspace store, object, and lock directories are checked without following symbolic links before use. Removing an older capture through retention does not mutate its referenced snapshot payload; unreferenced payload cleanup is retried on later access, and a committed capture remains successful while any pending cleanup or directory-sync warning is surfaced separately. Clearing the selected workspace's history is a destructive local-data action and requires explicit confirmation. Capture writes must be atomic, schema-bounded, and surface fresh-scan, corruption, or pre-commit persistence failures rather than saving stale live state or silently substituting current frontend data.

If either saved capture represents an incomplete scan, comparison can state field changes for items observed on both sides, but absence-based claims must remain qualified as “observed only in” rather than unconditional additions or removals.

## Later durable model

The longer-term durable schema additionally centers on:

- `Run` and `RunStep`
- `Evidence`
- `Evaluation`

Each bound run must reference an immutable context snapshot captured through adapter-backed execution-time evidence; otherwise comparisons cannot attribute changes to model, Harness or workflow revisions. v0.4 intentionally creates no run-to-snapshot references.
