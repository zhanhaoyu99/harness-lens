# Architecture

## Decision: Tauri 2 desktop shell

The first version uses Tauri 2 with a React/TypeScript frontend and a Rust backend.

Reasons:

- The end state depends on interactive topology, timelines, diffs and shareable reports; the Web UI ecosystem fits those surfaces well.
- Tauri uses the system WebView and does not bundle Chromium, keeping the desktop app aligned with the lightweight positioning.
- Rust provides a narrow local boundary for filesystem scanning, SQLite and Codex App Server stdio.
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
- later: immutable context snapshots, evaluations and share bundles.

### React frontend

- workspace selection and recent workspace state;
- Overview Map/List;
- item inspector;
- aggregate Share snapshot;
- metadata-only Codex Runs and linear replay;
- later: Compare and verifier evidence views.

## Runtime integration boundary

The app does not treat Codex's on-disk rollout JSONL as a permanent public contract. The v0.1 Codex adapter uses the experimental App Server protocol over local stdio for current skills/hooks, workspace thread listing, and read-only stored thread reads. Responses are bounded and normalized through an explicit metadata allowlist; raw responses are not persisted or logged.

The flight recorder shows a linear sequence of turns and normalized item types. It excludes raw prompts, reasoning, tool arguments and file diffs. A completed turn is runtime activity, not verifier evidence or proof of success.

Historical thread reads do not currently include a trustworthy historical instruction-source snapshot. Therefore v0.1 does not claim that current skills/hooks are the exact Harness used by an older run. Capturing and binding immutable context snapshots is an M2 requirement.

Claude and other runtimes get separate adapters. Shared UI depends only on normalized entities.

## Security and privacy

- All scanning is opt-in through a chosen workspace plus documented user-level Harness locations.
- The MVP is read-only.
- Files are size-limited before loading.
- Sensitive patterns are redacted on a best-effort basis before entering UI previews.
- Claude project memory is filtered to the selected workspace rather than scanning unrelated projects.
- Opening a source file goes through a backend allowlist populated by the current scan.
- Session content and evidence are read only when the user opens a specific run.
- Runtime normalization keeps allowlisted metadata and excludes raw prompts, reasoning, tool arguments and diffs.
- The current Share snapshot contains aggregate counts only; it excludes content and absolute paths.
- Future exports require a separate redaction pass and preview.
- Unknown runtime trust or precedence is shown as unknown rather than guessed.

## Data evolution

The current release remains in-memory. Local persistence should be added only when immutable snapshots and explicit retention controls arrive. The durable schema should center on:

- `ContextSnapshot`
- `HarnessItemRevision`
- `ItemResolution`
- `Run` and `RunStep`
- `Evidence`
- `Evaluation`

Each run must reference an immutable context snapshot; otherwise comparisons cannot attribute changes to model, Harness or workflow revisions.
