# Harness Lens development guide

## Product contract

- Harness Lens is a local-first Agent Harness inspector before it becomes an orchestration platform.
- Preserve the four-stage model: Defined -> Effective -> Observed -> Evaluated.
- Never present a discovered file as active, used, or successful without adapter-backed evidence.
- Unknown cost, usage, precedence, or runtime state must remain explicitly unknown.
- Default to redacting secrets and sensitive values in previews and exports.

## Engineering rules

- Keep scanning and provider normalization in the Tauri/Rust backend; React code must not know provider-specific paths.
- Provider-specific behavior belongs behind adapters so Codex and Claude can evolve independently.
- The app is read-only by default. The only write capability is an explicit, confirmed edit of an existing scanned Memory Markdown file; Rules, Skills, Hooks, Agents, Config, runtime history, rename, create, and delete operations remain read-only.
- Frontend filesystem access must go through narrow Tauri commands rather than broad filesystem permissions.
- Add fixture-based tests for every newly supported config or trace format.
- Run `pnpm test`, `pnpm build`, and `cargo test --manifest-path src-tauri/Cargo.toml` after relevant changes.
