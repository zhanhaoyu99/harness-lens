# App issues

## Open

- Claude precedence and runtime usage evidence are not yet backed by a stable adapter; discovery must not be labelled effective by assumption.
- The public macOS package is arm64-only, ad-hoc signed, and not Apple-notarized.
- Historical runs are not bound to the exact immutable Harness snapshot used at execution time.
- Run completion is not independent verifier evidence; comparison metrics remain intentionally unavailable.
- The project is new and has no independently verified users, external contributors, stars, forks, or testimonials yet.

## Blocking

- None.

## Resolved

- Avoided using raw rollout JSONL as the primary runtime contract; Codex App Server is the planned adapter.
- Added a project-local Rust launcher so normal package scripts can find the installed toolchain.
- Kept source opening behind the current scan's canonical-path allowlist.
- Published a reproducible, checksummed v0.1.0 release and synthetic-only Pages demo.
- Added a metadata-only Codex Run Flight Recorder without exposing prompts, reasoning, tool arguments, or diffs.
- Completed the first public maintenance loop in v0.1.1 and removed Node.js 20 workflow annotations.
- Added explicit metadata-only snapshot history and Saved-to-Saved comparison in v0.4.0.
- Refreshed the public Codex for Open Source evidence record through v0.4.0 while preserving the original submitted text.
