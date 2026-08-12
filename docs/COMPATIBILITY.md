# Compatibility evidence

Harness Lens treats compatibility as versioned evidence, not as a broad promise inferred from one successful run. This page records the latest verified combinations and the claim each check can support.

Last verified: **2026-08-12**

| Surface | Verified version or scope | Evidence | Supported claim |
|---|---|---|---|
| Desktop release | Harness Lens `v0.1.1`; Apple Silicon; macOS 11+ | GitHub-hosted arm64 build, DMG checksum, DMG integrity, strict code-signature check, bundle version and minimum-OS inspection | The published arm64 artifact is internally consistent and installable subject to the documented Gatekeeper limitation |
| Codex integration | `codex-cli 0.147.0-alpha.6.5` on macOS | Local Harness scan plus read-only App Server initialize and workspace thread list/read, bounded normalization, and an unchanged historical `updatedAt` after read | Current declarations and persisted thread metadata can be inspected without resuming or mutating the sampled run |
| Claude Harness files | User and repository discovery only | Fixture-backed filesystem scanner tests | Supported files can be discovered as definitions; runtime precedence or usage is not proven |
| Browser demo | Current GitHub Pages build with synthetic fixtures | Frontend tests, production build, Pages deployment and HTTP 200 check | The presentation layer can be explored without access to local files or a local Codex runtime |

## Version policy

- An exact version in this table is an evidence point, not an implied minimum or maximum compatible range.
- Unknown runtime item types remain visible as unknown; they are not silently reinterpreted as success or evaluation evidence.
- A provider schema change must be covered by a synthetic fixture and a real read-only probe before this page is updated.
- Historical runs without a captured Harness snapshot continue to show `Harness context: not captured`.
- A completed run is never treated as an independently evaluated outcome.

## Reporting a compatibility problem

Open a bug report with the Harness Lens version, provider/CLI version, operating system, the smallest synthetic reproduction, and the visible error or unknown state. Do not attach private prompts, tool arguments, run content, secrets, or absolute user paths.

Security-sensitive reports should use the repository's private **Report a vulnerability** flow described in [SECURITY.md](../SECURITY.md).
