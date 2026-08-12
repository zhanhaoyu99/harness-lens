# Threat model

## Scope

This threat model covers the Harness Lens v0.1 desktop app, filesystem scanner, Tauri command boundary, Share snapshot, and experimental Codex App Server adapter on macOS.

## Assets to protect

- Secrets and proprietary text in Harness configuration.
- Absolute paths and repository names that identify private work.
- Codex prompts, tool arguments, file changes, and model output.
- Integrity of claims shown as Defined, Resolved, Observed, or Evaluated.
- The user's filesystem and Codex thread history.

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
| Shared report leaks content | v0.1 Share output is aggregate-only; absolute paths and content excluded | Screenshots and manual copying remain outside the export boundary |
| Activity is mistaken for success | Four-stage model; run completion is not evaluation | UI wording regressions can reintroduce misleading claims |
| Compromised dependency or build | Lockfiles, CI, Dependabot, checksums | v0.1 artifacts are not notarized and the build is not yet reproducible |

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
2. Is the operation read-only? If not, is it limited to the documented Memory edit allowlist, explicitly confirmed, conflict-checked, and strictly scoped?
3. What size, time, path, and schema bounds apply?
4. Which fields reach the frontend, logs, disk, clipboard, or network?
5. How can a test prove that secrets and unsupported states fail closed?

Report vulnerabilities through [SECURITY.md](../SECURITY.md).
