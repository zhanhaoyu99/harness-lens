## What changed

<!-- Describe one focused behavior or boundary. -->

## Why

<!-- Link the issue/user problem and the evidence supporting this change. -->

## Evidence stage

<!-- Defined / Resolved / Observed / Evaluated. Explain why this is the strongest justified stage. -->

## Validation

- [ ] `pnpm test`
- [ ] `pnpm build`
- [ ] Rust format check
- [ ] Rust Clippy
- [ ] `pnpm rust:test`
- [ ] Manual app validation, if UI/runtime behavior changed

Exact scope and results:

<!-- State what actually ran. Focused tests are not full-suite proof. -->

## Privacy and security

- [ ] No real secrets, private prompts, raw run payloads, or identifying absolute paths were added.
- [ ] New scan locations, native commands, runtime methods, persistence, exports, and network behavior are documented (or none were added).
- [ ] Redaction is still described as best-effort.
- [ ] Run completion is not presented as evaluation success.

## Screenshots

<!-- Use synthetic data and manually inspect the final image. Delete this section when not applicable. -->
