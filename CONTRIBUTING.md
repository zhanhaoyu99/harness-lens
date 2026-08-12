# Contributing to Harness Lens

Thank you for helping make agent tooling more observable and trustworthy. Harness Lens is early-stage software, so small, evidence-backed changes are especially valuable.

## Good contributions

- A minimal fixture that reproduces a Codex or Claude Harness discovery problem.
- A compatibility report that includes the runtime version, platform, expected behavior, and safely redacted output.
- A focused improvement to redaction, path containment, resource limits, or permission boundaries.
- A test that makes a runtime schema change visible without depending on private run content.
- Documentation that clarifies a verified boundary rather than expanding a claim.

Please do not submit real secrets, prompts, model output, customer data, private repository paths, or raw Codex thread payloads. Reduce examples to synthetic fixtures first.

## Before opening an issue

1. Search existing issues and the [roadmap](docs/ROADMAP.md).
2. Use the relevant issue template.
3. Reproduce the problem on the latest `main` branch when possible.
4. Include app, Codex CLI, macOS, Node.js, pnpm, and Rust versions that matter.
5. Redact screenshots and logs manually. The built-in redactor is best-effort, not a sharing guarantee.

Security vulnerabilities belong in a private GitHub Security Advisory, not a public issue. See [SECURITY.md](SECURITY.md).

## Development setup

Requirements are listed in the [README](README.md#build-from-source).

```bash
git clone https://github.com/zhanhaoyu99/harness-lens.git
cd harness-lens
corepack enable
pnpm install --frozen-lockfile
pnpm tauri dev
```

The main areas are:

- `src/`: React/TypeScript UI and normalized frontend types.
- `src-tauri/src/`: read-only scanning, redaction, runtime adapter, and Tauri commands.
- `docs/`: product boundaries, architecture, privacy, and maintainer runbooks.

## Pull request workflow

1. Create a branch from `main`.
2. Keep one pull request focused on one behavior or boundary.
3. Add or update tests for behavior changes.
4. Update user-facing documentation and `CHANGELOG.md` when appropriate.
5. Run the checks below.
6. Complete the pull request checklist and explain the evidence behind the change.

```bash
pnpm test
pnpm build
sh scripts/with-rust.sh cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
sh scripts/with-rust.sh cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm rust:test
```

If a full check cannot run in your environment, state exactly what ran and why the rest did not.

## Design principles

- Keep the desktop app local-first and read-only unless a future proposal explicitly changes that contract.
- Prefer narrow provider adapters over inferred cross-runtime semantics.
- Keep **Defined**, **Resolved**, **Observed**, and **Evaluated** distinct in types, labels, and documentation.
- A run, completed turn, or tool call is activity evidence—not proof of task success.
- Normalize only the minimum metadata needed for the UI. Do not persist or log raw runtime responses.
- Fail closed on path containment, invocation permissions, export contents, and unsupported runtime data.
- Make compatibility failures visible. Do not fabricate “effective” or “evaluated” states.

## Commit messages

Use `Type: Description`, with a concise English description. Common types are `Feat`, `Fix`, `Refactor`, `Build`, `Chore`, and `Style`.

Examples:

```text
Feat: Add Codex hook resolution metadata
Fix: Reject workspace symlink escapes
Chore: Document unsigned release boundary
```

## License and conduct

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE). All contributors must follow the [Code of Conduct](CODE_OF_CONDUCT.md).
