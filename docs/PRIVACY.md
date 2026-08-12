# Privacy

Harness Lens is designed around local inspection of sensitive developer context. This document describes the v0.1 data boundary; it is not a promise about unrelated tools such as Codex CLI, Claude Code, editors, or the operating system.

## What Harness Lens reads

After a user selects a workspace, the app may read:

- recognized repository-level Harness files under that workspace;
- documented user-level Codex and Claude configuration locations;
- metadata exposed by the local Codex App Server for the selected working directory;
- a selected Codex thread through a read-only request when the user opens it.
- the bytes of recognized Memory files locally during scanning to calculate bounded metadata and a content hash; their raw text crosses from the Rust backend into the UI only after the user explicitly opens one in the Memory viewer/editor.

The scanner records normalized metadata such as type, provider, scope, path, size, modification time, content hash, resolution state, and a bounded redacted preview. It reads Memory bytes locally to calculate their hash, but Memory sources remain metadata-only in the regular snapshot and their raw text is not sent to React. If the user explicitly opens one, its original, potentially unredacted text is loaded into transient editor state and is not added to Share output.

## What Harness Lens does not do in v0.1

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

## Local persistence

v0.1 does not maintain a cloud database. Browser/UI state and normal operating-system caches may exist locally. Future local snapshot persistence will require an explicit retention and deletion design before release.

## Workspace boundaries

The selected repository is untrusted input. Path containment, file-size limits, bounded previews, and command allowlists are defense-in-depth measures. They do not isolate Harness Lens from another malicious process running under the same macOS user.

## Questions and changes

Privacy-impacting changes require documentation and review in the pull request. Security-sensitive questions should use the private reporting path in [SECURITY.md](../SECURITY.md).
