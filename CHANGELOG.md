# Changelog

All notable changes to Harness Lens will be documented in this file. The project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) while the public API and data model are still pre-1.0.

## [Unreleased]

### Changed

- Clarify the README around concrete Codex and Claude Harness workflows, add direct demo/download/feedback actions, and preserve the Defined/Resolved/Observed/Evaluated evidence boundary.
- Add search, Open Graph, Twitter Card, and SoftwareApplication metadata to the synthetic demo.

### Added

- Add a privacy-conscious compatibility-report issue form so real provider support can be documented without collecting Harness content.
- Add a versioned, source-attributed aggregate compatibility-report command that excludes workspace paths, Harness content, names, artifact/content hashes, branches, and diagnostic details.
- Add a desktop Share flow that performs a fresh read-only disk scan, previews the complete schema-v1 aggregate report, and copies only after explicit review; unsaved Memory drafts and the synthetic browser demo remain outside real compatibility evidence.
- Add a reproducible 31-second synthetic README tour covering Harness inventory, metadata-only Codex run replay, and Saved-to-Saved snapshot comparison.

### Security

- Add CodeQL analysis for GitHub Actions, JavaScript/TypeScript, and Rust while keeping alert review distinct from workflow success.
- Separate read-only DMG builds from the privileged draft-release job and add GitHub provenance attestation for future release assets.

## [0.4.0] - 2026-08-13

### Added

- Keep ordinary workspace selection and Rescan operations live-only, without writing snapshot history.
- Add an explicit Capture action that performs a fresh backend scan and atomically persists a capture referencing an immutable, content-addressed, metadata-only Harness snapshot.
- Keep the latest 50 explicit capture records for each workspace and provide an explicitly confirmed action for clearing that workspace's history.
- Enable Saved-to-Saved comparison within one workspace for observed additions, removals, content-hash changes, resolution changes, and diagnostic changes.
- Record schema and compatibility metadata with every saved snapshot while keeping incomplete-scan evidence visible in history and comparison.

### Evidence and privacy boundaries

- Persisted snapshots exclude Harness file content and previews, raw Memory text, absolute paths, prompts, reasoning, tool arguments, file diffs, and raw runtime responses.
- v0.4.0 compares saved Harness context only. It does not bind current or historical runs to a snapshot, compare run outcomes, or infer that the nearest snapshot was active for a run.
- A Saved-to-Saved difference is configuration evidence, not verifier evidence or proof that either task result succeeded.

### Security

- Serialize snapshot-store access across app processes, reject symbolic-link store directories, verify content-addressed objects before use, and roll back uncommitted objects when an index write fails.
- Surface post-commit durability and expired-metadata cleanup warnings without misreporting a committed capture as failed; retry pending cleanup on later access.

### Changed

- Raise the source-build minimum supported Rust version to 1.88 to match the locked dependency graph.
- Upgrade the direct SHA-256 dependency to `sha2` 0.11 while preserving the existing digest-based identities.

### Deferred beyond 0.4.0

- Adapter-backed execution-time snapshot binding for newly captured runs.
- Verifier evidence and evaluation outcomes.
- Trustworthy comparison of success rates, costs, duration, and failure modes across bound Harness revisions.
- Notarized and broader macOS distribution.

## [0.3.0] - 2026-08-12

### Added

- A direct Harness-source lens for separating Codex, Claude, shared, and discovered future provider content, with counts that compose with scope, type, and search filters.
- Result counts and visible active-filter state across List and Map exploration.

### Changed

- Kept Map and List modes truthful while filters are active, preserved existing source and scope filters when drilling into a content type, and routed diagnostics to a currently visible artifact when possible.
- Made the explorer toolbar wrap cleanly in narrower supported windows.

### Fixed

- Restored full-page and sidebar scrolling when minimum-height content previously expanded the app grid beyond a non-maximized window.

## [0.2.0] - 2026-08-12

### Added

- On-demand Memory viewing and explicit editing for eligible existing Markdown files, with revision conflict detection and native confirmation.
- Project and nested-project Harness discovery along the selected workspace chain.

### Changed

- Reframed provider comparison diagnostics as “same name, different content” instead of a resolution status, and stopped comparing entries across different scopes.

### Security

- Limited Memory writes to eligible, already-scanned Markdown files with revision-bound tokens, native confirmation, conflict detection, and atomic replacement.

### Known limitations

- Memory replacement preserves POSIX permissions but does not yet preserve macOS ACLs or extended attributes; hostile processes already running as the same user remain outside the isolation boundary.
- Release artifact remains macOS arm64 only, ad-hoc signed, and not Apple-notarized.

## [0.1.1] - 2026-08-12

### Changed

- Upgraded the GitHub Actions used by CI, Pages, and Release to their Node.js 24-based major versions.

### Fixed

- Removed the Node.js 20 deprecation annotations from the maintained GitHub workflows.

## [0.1.0] - 2026-08-12

### Added

- Local-first, read-only Codex and Claude Harness inventory.
- Map, List, Inspector, Overview, and aggregate-only Share views.
- Scope, resolution, duplicate, truncation, and redacted-preview metadata.
- Experimental Codex App Server connection for current skills/hooks and recent workspace threads.
- Metadata-only linear replay of Codex run turns and item types.
- English and Chinese interface.
- Headless workspace summary command.
- GitHub Pages workflow for a synthetic, non-scanning browser demo.
- Initial privacy, threat-model, security, contribution, CI, and release documentation.

### Known limitations

- Release artifact is macOS arm64 only, ad-hoc signed, and not Apple-notarized.
- Historical runs are not bound to the exact effective Harness snapshot used at execution time.
- Completed runs and turns are not independently evaluated for correctness.
- Redaction is best-effort and requires human review before sharing.

[Unreleased]: https://github.com/zhanhaoyu99/harness-lens/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.4.0
[0.3.0]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.3.0
[0.2.0]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.2.0
[0.1.1]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1
[0.1.0]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.0
