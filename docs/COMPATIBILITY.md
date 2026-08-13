# Compatibility evidence

Harness Lens treats compatibility as versioned evidence, not as a broad promise inferred from one successful run. This page records the latest verified combinations and the claim each check can support.

Last verified: **2026-08-13**

| Surface | Verified version or scope | Evidence | Supported claim |
|---|---|---|---|
| Desktop release | Harness Lens `v0.4.0`; Apple Silicon; minimum deployment-target metadata: macOS 11.0 | Public GitHub arm64 DMG and checksum, exact public-download SHA-256 verification (`8f9ba140c804a1ff627146200adf20a44c37004defcc584ed90679406b481313`), DMG integrity verification, strict ad-hoc code-signature check, bundle version/build, architecture, and minimum deployment-target inspection, and background launch/quit from the mounted public DMG | The published arm64 DMG, checksum, bundle metadata, and signature are internally consistent and the exact public artifact launched and exited on the sampled Apple Silicon host; installation and runtime compatibility across macOS 11+ were not tested, and the documented Gatekeeper limitation applies |
| Codex integration | `codex-cli 0.147.0-alpha.6.5` on macOS | Local Harness scan plus read-only App Server initialize and workspace thread list/read, bounded normalization, and an unchanged historical `updatedAt` after read | Current declarations and persisted thread metadata can be inspected without resuming or mutating the sampled run |
| Claude Harness files | User and repository discovery only | Fixture-backed filesystem scanner tests | Supported files can be discovered as definitions; runtime precedence or usage is not proven |
| Browser demo | Current GitHub Pages build with synthetic fixtures | Frontend tests, production build, Pages deployment, and versioned asset verification | The presentation layer can be explored without access to local files or a local Codex runtime |

## Version policy

- An exact version in this table is an evidence point, not an implied minimum or maximum compatible range.
- Unknown runtime item types remain visible as unknown; they are not silently reinterpreted as success or evaluation evidence.
- A provider schema change must be covered by a synthetic fixture and a real read-only probe before this page is updated.
- Historical runs without a captured Harness snapshot continue to show `Harness context: not captured`.
- A completed run is never treated as an independently evaluated outcome.

## Reporting a compatibility problem

Open a bug report with the Harness Lens version, provider/CLI version, operating system, the smallest synthetic reproduction, and the visible error or unknown state. Do not attach private prompts, tool arguments, run content, secrets, or absolute user paths.

Security-sensitive reports should use the repository's private **Report a vulnerability** flow described in [SECURITY.md](../SECURITY.md).
