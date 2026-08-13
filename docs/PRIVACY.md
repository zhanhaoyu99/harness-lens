# Privacy

Harness Lens is designed around local inspection of sensitive developer context. This document describes the current release data boundary; it is not a promise about unrelated tools such as Codex CLI, Claude Code, editors, or the operating system.

## What Harness Lens reads

After a user selects a workspace, the app may read:

- recognized repository-level Harness files under that workspace;
- documented user-level Codex and Claude configuration locations;
- metadata exposed by the local Codex App Server for the selected working directory;
- a selected Codex thread through a read-only request when the user opens it.
- the bytes of recognized Memory files locally during scanning to calculate bounded metadata and a content hash; their raw text crosses from the Rust backend into the UI only after the user explicitly opens one in the Memory viewer/editor.

The scanner records normalized metadata such as type, provider, scope, path, size, modification time, content hash, resolution state, and a bounded redacted preview. It reads Memory bytes locally to calculate their hash, but Memory sources remain metadata-only in the regular snapshot and their raw text is not sent to React. If the user explicitly opens one, its original, potentially unredacted text is loaded into transient editor state and is not added to Share output.

## What Harness Lens does not do in the current release

- No Harness Lens account, cloud sync, analytics, advertising, or telemetry.
- No remote upload of scanned files or Codex run content.
- No edit, start, resume, fork, archive, or delete operation on Codex threads.
- No raw runtime response persistence or logging by design.
- No raw prompt, model reasoning, tool argument, or file-diff display in the run recorder.
- No automatic screenshot or report upload.
- No creation, rename, deletion, force-save, background save, or automatic save of Harness files. Rules, Skills, Hooks, Agents, Config, Workflows, generated Memory summaries, and runtime history remain read-only.

Harness Lens invokes a locally installed Codex CLI for its experimental App Server API. The Codex CLI remains governed by its own version, configuration, authentication, and privacy terms.

The GitHub Pages demo is a static browser build with synthetic examples. Browsers do not expose the Tauri filesystem or local Codex commands to that build, so it cannot scan the visitor's machine or load local threads.

## Redaction is best-effort

Recognized secrets are redacted before text previews are sent to the frontend. This reduces accidental exposure; it cannot identify every password, personal detail, proprietary string, or novel token format.

Therefore:

- treat every preview as potentially sensitive;
- do not use screenshots as a security boundary;
- inspect any copied or exported material before sharing;
- prefer synthetic fixtures in bug reports;
- use a private Security Advisory for vulnerabilities.

The Share view is intentionally aggregate-only and excludes file contents and absolute paths. Memory editor content is never copied into the regular snapshot or Share model.

The optional source-build `compatibility-report` command produces a separate aggregate projection for manual sharing. It includes the Harness Lens version, the validated HEAD observed from the Harness Lens source checkout when the report runs, its dirty/unknown state, platform family and executable target architecture, counts by provider/kind/resolution, diagnostic severity counts, and scan completeness. This source attribution is not build provenance. Source detection serializes no Git errors, repository paths, branches, remotes, changed-file names, or diff content. The aggregate projection also excludes scanned workspace and artifact names, paths, branch, content, previews, artifact/content hashes, sizes, timestamps, diagnostic text, and runtime payloads. Aggregate counts and a source revision can still reveal information about a developer setup, so the complete output must be reviewed before publication.

## Local persistence

Before v0.4, Harness Lens did not maintain its own snapshot-history store. Browser/UI state and normal operating-system caches may still exist locally.

v0.4 adds app-managed local history only when the user explicitly chooses Capture. Choosing a workspace or using Rescan updates the live view without writing history. On Capture, the backend performs a fresh scan and atomically creates a workspace-scoped capture that references an immutable, content-addressed, metadata-only snapshot; it does not persist a snapshot object supplied by the frontend. The persisted projection may include workspace and artifact display names, Git branch, safe relative source labels, provider, kind, scope, content hashes, sizes, resolution states, normalized diagnostics, capture time, completeness, and compatibility versions. These metadata can still reveal project structure or change patterns, so the local store should be treated as sensitive developer data.

The v0.4 persisted projection must not include:

- Harness file content or redacted previews;
- raw Memory text;
- absolute paths;
- prompts, model reasoning, tool arguments, file diffs, or raw runtime responses.

History is isolated by workspace and its index keeps the latest 50 explicit captures for each workspace. Ordinary live scans do not count toward retention. Metadata objects no longer referenced by those captures are deleted on a best-effort basis; failed cleanup is surfaced after a successful capture and retried on later store access, so expired metadata can remain locally until cleanup succeeds. A process crash can also leave an app-managed transaction file until the store is inspected or the application data is removed. The user can clear the selected workspace's remaining history only through an explicitly confirmed action. Clearing app data or capture history does not guarantee removal from operating-system backups, filesystem snapshots, or forensic storage.

Saved history remains local: v0.4 adds no cloud sync, telemetry, automatic export, or network upload. The browser demo uses synthetic snapshot history and cannot access the desktop store.

Two saved snapshots from the same workspace can be compared, but they are not file backups and cannot restore historical content. A nearby capture timestamp does not prove that a snapshot was active for a Codex run; v0.4 persists no run-to-snapshot binding.

## Workspace boundaries

The selected repository is untrusted input. Path containment, file-size limits, bounded previews, and command allowlists are defense-in-depth measures. They do not isolate Harness Lens from another malicious process running under the same macOS user.

## Questions and changes

Privacy-impacting changes require documentation and review in the pull request. Security-sensitive questions should use the private reporting path in [SECURITY.md](../SECURITY.md).
