# Maintainer automation

Automation should make evidence easier to review, not manufacture activity or hide release boundaries.

## Continuous integration

`.github/workflows/ci.yml` runs on pull requests and pushes to `main`:

- locked pnpm install;
- frontend unit tests;
- TypeScript and Vite production build;
- Rust formatting check;
- Clippy with warnings treated as errors;
- Rust tests;
- production JavaScript dependency audit;
- RustSec advisory audit of the complete Cargo lockfile.

A green workflow means these commands passed in the declared runner environment. It is not evidence of a full macOS interaction test, accessibility audit, privacy review, Apple notarization, or task-level Agent success.

The current `cargo audit` command fails vulnerability advisories, while RustSec warning categories such as `unmaintained` and `unsound` remain visible but do not fail the job unless an explicit `--deny` policy is added. Because the lockfile contains dependencies for every supported Cargo target, warning policy must not hide a real advisory or pretend that an unshipped target is present in the DMG; use the target-aware dependency triage below and record the residual risk.

`.github/workflows/codeql.yml` runs GitHub CodeQL over GitHub Actions workflows, JavaScript/TypeScript, and Rust on pull requests, pushes to `main`, and a weekly schedule. It uses the `security-extended` query suite and source-based `build-mode: none`, so it does not require release credentials or execute a product build. A green analysis job means CodeQL completed and uploaded its result; it does **not** mean the result contains zero alerts. Maintainers must inspect Code scanning alerts and configure any required severity threshold separately.

`.github/workflows/pages.yml` builds and deploys the static synthetic demo from `main`. The Pages build has no Tauri command bridge, cannot read local files, and should never include real workspace fixtures.

## Dependency updates

Dependabot opens monthly pull requests for npm, Cargo, and GitHub Actions dependencies. Maintainers should:

1. read upstream release notes and security advisories;
2. verify lockfile changes are limited to the intended dependency family;
3. run CI and, for Tauri/Codex changes, a local app smoke test;
4. avoid grouping runtime-protocol changes with unrelated UI upgrades;
5. document compatibility changes in `CHANGELOG.md`.

Cargo lockfiles include target-specific transitive dependencies, so a lockfile alert is not by itself proof that a published artifact contains the affected crate. Triage must preserve both facts: the advisory remains real, and the published target must be checked explicitly. For example:

```bash
sh scripts/with-rust.sh cargo tree --manifest-path src-tauri/Cargo.toml --locked \
  --target aarch64-apple-darwin -i glib@0.18.5
sh scripts/with-rust.sh cargo tree --manifest-path src-tauri/Cargo.toml --locked \
  --target x86_64-unknown-linux-gnu -i glib@0.18.5
```

As checked on 2026-08-13, `glib 0.18.5` and [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g) are present through Tauri's Linux GTK/WebKit graph, while the current `aarch64-apple-darwin` graph prints no matching package. Harness Lens currently publishes only the macOS arm64 target. This target boundary does not resolve or dismiss the alert: keep it visible, re-check it after lockfile changes, and treat it as release-blocking before adding Linux distribution unless the dependency graph is upgraded or otherwise remediated.

## Release automation

`.github/workflows/release.yml` is manually dispatched from `main` for an existing `v*` tag. A read-only macOS job rejects other workflow refs, checks out the fully qualified tag without persisting checkout credentials, records and verifies the peeled tag commit is reachable from `main`, validates the code, builds a macOS arm64 DMG, generates a SHA-256 file, and uploads the workflow artifact. A separate, minimally privileged job downloads and verifies those exact assets, rejects a tag that moved after the build, records GitHub build provenance for the DMG, and creates a **draft** GitHub Release whose notes include the expected commit.

Manual review remains required because the current app is ad-hoc signed and not notarized. A provenance attestation binds the DMG digest to the GitHub workflow; it does not provide an Apple Developer ID signature, notarization, reproducibility, or Gatekeeper trust. See [RELEASING.md](RELEASING.md).

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
- The release build job remains read-only and does not persist checkout credentials.
- Only the manual attestation/draft job receives `contents: write`, `id-token: write`, `attestations: write`, and `artifact-metadata: write`; it does not check out repository code.
- Release automation creates drafts and uses an existing tag.
- Release provenance uses GitHub OIDC and short-lived workflow identity instead of a signing secret.
- No workflow receives long-lived personal access tokens.
- Actions and dependency updates should be reviewed before merge.
