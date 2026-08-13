# Threat model

## Scope

This threat model covers the current Harness Lens desktop app, filesystem scanner, Tauri command boundary, Share snapshot, experimental Codex App Server adapter, and the v0.4 candidate's local metadata-only snapshot history and Saved-to-Saved comparison on macOS.

## Assets to protect

- Secrets and proprietary text in Harness configuration.
- Absolute paths and repository names that identify private work.
- Codex prompts, tool arguments, file changes, and model output.
- Integrity of claims shown as Defined, Resolved, Observed, or Evaluated.
- The user's filesystem and Codex thread history.
- Local snapshot metadata, capture history, and the integrity of historical comparison claims.

## Trust boundaries

```text
Untrusted selected repository
  -> bounded Rust scanner
    -> normalized/redacted model
      -> Tauri command allowlist
        -> React UI and aggregate Share output

Local Codex CLI / experimental App Server
  -> bounded JSONL transport
    -> metadata allowlist
      -> run timeline UI

Explicit Capture request
  -> fresh normalized Harness scan
    -> metadata-only persistence projection
      -> app-managed, workspace-scoped local history
        -> Saved-to-Saved comparison UI
```

User-level Harness files and the locally installed Codex binary are trusted more than the selected repository, but can still contain sensitive or malformed input.

## Primary threats and controls

| Threat | Current controls | Residual risk |
|---|---|---|
| Repository symlink escapes the selected root | Canonical path and scope containment checks | Same-user filesystem changes can race a read; platform behavior needs continued testing |
| Huge or recursive input exhausts memory/time | Candidate, file-size, frame, and preview bounds | Very large directory trees can still cause latency |
| Secret appears in preview | Server-side best-effort pattern redaction and bounded content | Unknown token formats or ordinary-language secrets may remain |
| Runtime response contains private run data | Read-only methods, per-thread opt-in, metadata allowlist, no raw response persistence | Adapter defects or schema drift could expose an unexpected field |
| Frontend invokes an unintended native command | Explicit Tauri command manifest/capability and runtime allowlist | Future commands can expand authority if review is weak |
| Malicious path is opened | Only artifacts from the current scan can be opened | Same-user time-of-check/time-of-use replacement remains possible |
| A Memory edit overwrites the wrong or newer file | Artifact-ID allowlist, short-lived revision-bound edit token, unsupported ownership/link/permission cases made view-only, explicit native confirmation, conflict detection, and same-directory atomic replacement | A malicious process already running as the same user remains outside the isolation boundary; macOS ACLs and extended attributes are not preserved by the current editor |
| Raw Memory text leaks through a normal snapshot or Share | Memory is metadata-only in normal scans and loaded only on explicit request into transient editor state | The user can still copy or screenshot sensitive editor text |
| Shared report leaks content | Current Share output is aggregate-only; absolute paths and content excluded | Screenshots and manual copying remain outside the export boundary |
| Workspace selection or ordinary Rescan unexpectedly writes durable history | Live-scan commands remain transient; a separate explicit Capture command performs its own fresh backend scan and never accepts frontend snapshot content for persistence | Users may still misunderstand the Capture label without clear UI copy |
| v0.4 history persists sensitive Harness content or an absolute path | A dedicated metadata-only persistence projection excludes all content and previews, raw Memory, absolute paths, and runtime payloads; serialization tests inspect the durable representation | Names, branch names, relative source labels, hashes, and change timing can still reveal project structure |
| A malformed or tampered local snapshot creates false historical evidence | Versioned bounded schema, content-address verification, atomic writes, cross-process workspace lock, symlink-rejecting store checks, deterministic diffing, and visible failure instead of fallback to current state | A malicious process running as the same user can still modify or delete local application data outside the lock protocol |
| Snapshot history accumulates indefinitely | The index retains the latest 50 explicit captures per workspace; unreferenced-object cleanup is retried and failures are surfaced; an explicitly confirmed clear-history action is available | Failed cleanup can retain expired metadata until a later retry; copies may remain in operating-system backups or filesystem snapshots |
| An incomplete scan is presented as definitive addition or removal evidence | Completeness is persisted; comparisons involving absence use qualified “observed only in” language when either side is incomplete | Users may still overlook the visible qualification |
| A run is associated with the nearest snapshot by timestamp | v0.4 stores no run-to-snapshot binding and continues to show Harness context as not captured | Exact binding remains unavailable until a runtime adapter supplies an execution-time capture boundary |
| Activity is mistaken for success | Four-stage model; run completion is not evaluation | UI wording regressions can reintroduce misleading claims |
| Compromised dependency or build | Lockfiles, CI, Dependabot, checksums | Current artifacts are not notarized and the build is not yet reproducible |

## Runtime adapter constraints

The Codex adapter should use fixed read-only methods such as list/read operations. It must not resume, start, fork, archive, delete, or otherwise mutate threads. Input and output frames are bounded, raw responses are not logged or persisted, and normalized UI fields are explicitly allowlisted.

Codex App Server is experimental. Unknown schema values should remain unknown or produce a visible compatibility error; they must not be reinterpreted as success, trust, or evaluation evidence.

## Out of scope

- A malicious process already running as the same macOS user.
- Compromise of the operating system, editor, Codex CLI, package registry, or GitHub account.
- Social engineering that convinces a user to share a screenshot or copy sensitive text.
- Privacy behavior of third-party tools invoked outside Harness Lens.

## Security review checklist

Any change that adds a scan location, Tauri command, runtime method, persisted field, network request, or export format must answer:

1. What new data or authority crosses a trust boundary?
2. Is the operation read-only? If not, is it limited either to the documented Memory edit allowlist or the app-managed snapshot store, explicitly initiated or confirmed as appropriate, and strictly scoped?
3. What size, time, path, and schema bounds apply?
4. Which fields reach the frontend, logs, disk, clipboard, or network?
5. How can a test prove that secrets and unsupported states fail closed?
6. If data is persisted, what is the schema version, retention and deletion policy, and how are corrupted or unsupported records surfaced?
7. Does a comparison distinguish stored configuration evidence from observed runtime activity and independently evaluated outcomes?

Report vulnerabilities through [SECURITY.md](../SECURITY.md).
