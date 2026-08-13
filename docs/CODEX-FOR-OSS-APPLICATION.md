# Codex for Open Source — application and live evidence record

Submitted: **2026-08-12**

Status: **The official form displayed its receipt confirmation; selection is not implied.**

This is a privacy-safe public record of the submitted project narrative plus clearly dated evidence added after submission. Personal contact information and the OpenAI organization ID were sent only through the [official form](https://openai.com/form/codex-for-oss/) and are intentionally not stored in this repository. The current eligibility rules and [Codex for Open Source Program Terms](https://learn.chatgpt.com/docs/codex-for-oss-terms) remain authoritative.

## Required information

- Maintainer identity: **submitted privately through the official form**
- Email associated with the maintainer's ChatGPT account: **submitted privately through the official form**
- GitHub username: `zhanhaoyu99`
- Public GitHub profile: https://github.com/zhanhaoyu99
- Primary repository: https://github.com/zhanhaoyu99/harness-lens
- Role: **Primary maintainer**
- Interests: **Codex Security** and **API credits for my project**
- OpenAI organization ID: **submitted privately through the official form**
- Public release at submission: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1
- Current public release: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.4.0
- Current evidence: [repository](https://github.com/zhanhaoyu99/harness-lens), [CI](https://github.com/zhanhaoyu99/harness-lens/actions/workflows/ci.yml), [synthetic demo](https://zhanhaoyu99.github.io/harness-lens/), [v0.4.0 release](https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.4.0), [compatibility evidence](COMPATIBILITY.md), [first maintenance loop](https://github.com/zhanhaoyu99/harness-lens/issues/10), and [public roadmap issues](https://github.com/zhanhaoyu99/harness-lens/issues?q=is%3Aissue%20state%3Aopen%20-label%3Adependencies). Add contributors, downloads, or dependent projects only as they exist.

## Live evidence added after submission

As of **2026-08-13**, the official form does not provide a published update workflow. The original submitted text below is therefore preserved verbatim; this section records later repository work without implying that OpenAI received an amended application.

As of **2026-08-13**:

- v0.2.0 added explicit Memory viewing/editing and project/nested-project scope evidence.
- v0.3.0 added composable Codex/Claude provider filtering and compact-window usability fixes.
- v0.4.0 added explicit, immutable, metadata-only Harness captures and Saved-to-Saved comparison.
- Main CI now covers frontend tests/build, Rust formatting/Clippy/tests, an explicit Rust 1.88 MSRV test, npm audit, and RustSec audit.
- Five checksummed macOS arm64 releases and a synthetic-only browser demo are public.
- Real adoption is still unproven: there are no public external contributors, forks, or documented third-party use cases yet. Stars and downloads must be reported only from current public evidence and with their limitations.

## Why this project may qualify (form-ready, under 500 characters)

Submitted text:

> Harness Lens is a new MIT-licensed, local-first Agent DevTool for inspecting coding-agent configuration and runtime behavior. It distinguishes defined, resolved, observed, and evaluated evidence, clearly marking evaluation as not yet implemented, and offers a read-only, metadata-only Codex run recorder. We do not claim broad adoption; we apply through the ecosystem-importance path because the project addresses a gap in reproducible, privacy-conscious agent observability.

## How API credits would be used (form-ready, under 500 characters)

Submitted text:

> Credits would support an open, reproducible compatibility and evaluation suite: synthetic repositories with varied rules, skills, hooks, and graphs; Codex-driven issue reproductions; verifier-backed comparisons across Harness revisions; and privacy tests that confirm raw prompts and secrets never enter published fixtures. Results, fixtures, failure categories, and supported-version evidence would be published in the repository.

## Why Codex Security is relevant (form-ready, under 500 characters)

Submitted text:

> Harness Lens inspects agent configuration, local filesystem metadata, hooks, and persisted run evidence—surfaces where prompt injection, malicious repository instructions, path traversal, secret exposure, and unsafe tool invocation can cross trust boundaries. Codex Security would help review the scanner, redaction, Tauri command ACLs, symlink containment, and future verifier adapters, and turn validated findings into public regression tests and threat-model updates.

## Anything else we should know? (form-ready, under 500 characters)

Submitted text:

> Harness Lens itself demonstrates the maintenance workflow this program supports: Codex helped audit and implement the app, triage a real GitHub Actions warning, review dependency updates, and ship a verified v0.1.1 release. The project is intentionally early, so we do not claim established adoption. Public CI, releases, issue history, roadmap, privacy model, and maintainer notes are available in the repository.

## Honest readiness checklist

- [x] Public MIT-licensed repository with a clear product boundary.
- [x] Green CI on `main`.
- [x] Public, checksummed macOS arm64 release with signing/notarization limitations disclosed.
- [x] Security policy, threat model, contribution guide, roadmap, and maintainer runbook.
- [x] Real public issue-to-fix-to-release loop: [#10](https://github.com/zhanhaoyu99/harness-lens/issues/10) → [#11](https://github.com/zhanhaoyu99/harness-lens/pull/11) → [v0.1.1](https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1).
- [x] Public maintenance evidence through reviewed dependency PRs, green workflows, and five tested releases.
- [x] Concise explanation of ecosystem importance without implying adoption; real usage signals remain future evidence.
- [x] Maintainer reviewed the official terms and supplied all required personal fields through the official form.
- [x] Official form displayed its submission receipt confirmation on 2026-08-12.

## Suggested supporting narrative

Harness Lens grew from a concrete developer problem: as Agent Harnesses accumulate repository instructions, user rules, skills, hooks, memory, and loops, it becomes difficult to know what is present and what each part contains. The project turns that into inspectable evidence, then extends the same boundary to runtime activity. Its purpose is not to add another Agent chat or orchestrator, but to make Agent systems easier to understand, reproduce, evaluate, and improve.

The current release remains intentionally narrow. It provides useful local inspection, experimental Codex thread forensics, and explicit Saved-to-Saved configuration comparison while clearly documenting what it cannot prove. Historical runs are not yet bound to the immutable snapshot used at execution time, and completed turns are not treated as task success. The next milestone adds execution-time binding for new runs, followed by independent verifier evidence and meaningful outcome comparisons.

## Submission boundary

Personal contact information and the OpenAI organization ID were submitted directly to OpenAI and must remain outside the public repository. This record documents the submitted narrative and observed receipt confirmation only; it does not claim selection, benefits, adoption, or independent verification by OpenAI. Never describe users, stars, downloads, or adoption without a public source.
