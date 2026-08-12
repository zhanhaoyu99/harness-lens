# App issues

## Open

- Claude precedence and runtime usage evidence are not yet backed by a stable adapter; discovery must not be labelled effective by assumption.
- A local unsigned `.app` is sufficient for dogfooding but not external distribution.
- Runs, actual execution paths, evidence and version comparison still require runtime adapters.

## Blocking

- None.

## Resolved

- Avoided using raw rollout JSONL as the primary runtime contract; Codex App Server is the planned adapter.
- Added a project-local Rust launcher so normal package scripts can find the installed toolchain.
- Kept source opening behind the current scan's canonical-path allowlist.
