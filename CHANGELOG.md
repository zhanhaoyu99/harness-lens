# Changelog

All notable changes to Harness Lens will be documented in this file. The project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) while the public API and data model are still pre-1.0.

## [Unreleased]

### Planned

- Immutable context snapshots bound to runs.
- Verifier evidence and evaluation outcomes.
- Trustworthy comparison of Harness revisions and failure modes.
- Notarized and broader macOS distribution.

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

[Unreleased]: https://github.com/zhanhaoyu99/harness-lens/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.2.0
[0.1.1]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1
[0.1.0]: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.0
