# Architecture decisions

## ADR-001: Start with inventory, not orchestration

Harness Lens first answers what exists, what it contains and what is effective. Loop/Graph execution and editing stay out of the MVP.

## ADR-002: Keep four evidence stages separate

Defined, Resolved, Observed and Evaluated are independent stages. The product does not infer later stages from earlier ones.

## ADR-003: Tauri 2 and React first

Use Tauri 2 with React/TypeScript and a Rust backend. The target product needs topology, timelines, diffs and reusable share views; using the system WebView keeps the package lightweight while preserving those UI advantages.

## ADR-004: Prefer official runtime adapters

Use Codex App Server for run history and live events. On-disk rollout JSONL is a versioned fallback importer, not the primary stable contract.

## ADR-005: Read-only and redacted by default, with one narrow Memory write path

Rules, Skills, Hooks, Agents, Config, Workflows, and runtime history remain read-only. An eligible, already-scanned Memory Markdown file may be edited only through an explicit revision-bound, conflict-checked, natively confirmed, atomic save. Sensitive values are redacted in previews and future exports.

## ADR-006: Bilingual UI follows the system

The UI supports Chinese and English, defaults to the system language and persists an explicit user choice. Harness source names and content are never translated.

## ADR-007: Share aggregate facts before run replay

The first Share view copies a redacted Markdown summary containing aggregate counts only. It excludes source content and absolute paths; richer images and replay bundles wait for runtime evidence and a separate export preview.

## ADR-008: Live scans are transient; history requires explicit Capture

Choosing a workspace or rescanning never writes history. Capture performs a fresh backend scan and persists a content-addressed, metadata-only snapshot through an explicit action. Saved-to-Saved comparison is configuration evidence and is never treated as run binding or verifier evidence.
