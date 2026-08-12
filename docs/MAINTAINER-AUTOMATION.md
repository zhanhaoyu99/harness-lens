# Maintainer automation

Automation should make evidence easier to review, not manufacture activity or hide release boundaries.

## Continuous integration

`.github/workflows/ci.yml` runs on pull requests and pushes to `main`:

- locked pnpm install;
- frontend unit tests;
- TypeScript and Vite production build;
- Rust formatting check;
- Clippy with warnings treated as errors;
- Rust tests.

A green workflow means these commands passed in the declared runner environment. It is not evidence of a full macOS interaction test, accessibility audit, privacy review, Apple notarization, or task-level Agent success.

`.github/workflows/pages.yml` builds and deploys the static synthetic demo from `main`. The Pages build has no Tauri command bridge, cannot read local files, and should never include real workspace fixtures.

## Dependency updates

Dependabot opens monthly pull requests for npm, Cargo, and GitHub Actions dependencies. Maintainers should:

1. read upstream release notes and security advisories;
2. verify lockfile changes are limited to the intended dependency family;
3. run CI and, for Tauri/Codex changes, a local app smoke test;
4. avoid grouping runtime-protocol changes with unrelated UI upgrades;
5. document compatibility changes in `CHANGELOG.md`.

## Release automation

`.github/workflows/release.yml` is manually dispatched for an existing `v*` tag. It validates the code, builds a macOS arm64 DMG, generates a SHA-256 file, uploads workflow artifacts, and creates a **draft** GitHub Release.

Manual review remains required because the current app is ad-hoc signed and not notarized. See [RELEASING.md](RELEASING.md).

## Issue-to-release maintenance loop

For real defects and compatibility reports:

1. Triage the issue and remove any private data.
2. Reduce it to a synthetic fixture or a documented manual reproduction.
3. State the expected evidence stage: Defined, Resolved, Observed, or Evaluated.
4. Implement the smallest fix with a regression test.
5. Link the pull request to the issue and record exact validation.
6. Ship it in a real release and confirm the public artifact.

Do not create fake users, stars, download claims, testimonials, or maintenance history. Usage and ecosystem value must come from verifiable external activity.

## Release and project health signals

Track evidence that helps maintainers improve the project:

- open/closed issues by failure category;
- time to acknowledge security and compatibility reports;
- CI pass rate and flaky-test causes;
- supported Codex CLI version fixtures;
- release downloads as reported by GitHub;
- independently submitted issues, pull requests, or discussions;
- verifier-backed use cases once evaluation support exists.

Avoid vanity metrics without context. In particular, run count and completed turns do not measure task correctness.

## Least-privilege defaults

- CI has read-only repository contents permission.
- Only the manual release job receives `contents: write`.
- Release automation creates drafts and uses an existing tag.
- No workflow receives long-lived personal access tokens.
- Actions and dependency updates should be reviewed before merge.
