# Codex for Open Source — application draft

This is a maintainer worksheet, not proof of acceptance or adoption. The official form and current eligibility rules remain authoritative. Do not submit until the repository and release links are public and the maintainer has reviewed the terms.

## Required information

- Maintainer name: Zane
- Public GitHub profile: https://github.com/zhanhaoyu99
- Primary repository: https://github.com/zhanhaoyu99/harness-lens
- Role: Primary/core maintainer
- Email: **TODO — maintainer must provide**
- OpenAI organization ID: **TODO — maintainer must provide**
- Public release: https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1
- Current evidence: [repository](https://github.com/zhanhaoyu99/harness-lens), [CI](https://github.com/zhanhaoyu99/harness-lens/actions/workflows/ci.yml), [synthetic demo](https://zhanhaoyu99.github.io/harness-lens/), [v0.1.1 release](https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1), [first maintenance loop](https://github.com/zhanhaoyu99/harness-lens/issues/10), and [public roadmap issues](https://github.com/zhanhaoyu99/harness-lens/issues?q=is%3Aissue%20state%3Aopen%20-label%3Adependencies). Add contributors, downloads, or dependent projects only as they exist.

## Why this project may qualify (form-ready, under 500 characters)

> Harness Lens is a new MIT-licensed, local-first Agent DevTool that makes coding-agent configuration and runtime behavior inspectable. It separates what is defined, resolved, observed, and independently evaluated, and provides a read-only, metadata-only Codex run recorder. We are not claiming broad adoption yet; we are applying through the ecosystem-importance path because reproducible, privacy-conscious agent observability is missing infrastructure.

## How API credits would be used (form-ready, under 500 characters)

> Credits would support an open, reproducible compatibility and evaluation suite: synthetic repositories with varied rules, skills, hooks, and graphs; Codex-driven issue reproductions; verifier-backed comparisons across Harness revisions; and privacy tests that confirm raw prompts and secrets never enter published fixtures. Results, fixtures, failure categories, and supported-version evidence would be published in the repository.

## Honest readiness checklist

- [x] Public MIT-licensed repository with a clear product boundary.
- [x] Green CI on `main`.
- [x] Public, checksummed macOS arm64 release with signing/notarization limitations disclosed.
- [x] Security policy, threat model, contribution guide, roadmap, and maintainer runbook.
- [x] Real public issue-to-fix-to-release loop: [#10](https://github.com/zhanhaoyu99/harness-lens/issues/10) → [#11](https://github.com/zhanhaoyu99/harness-lens/pull/11) → [v0.1.1](https://github.com/zhanhaoyu99/harness-lens/releases/tag/v0.1.1).
- [x] Public maintenance evidence through reviewed dependency PRs, green workflows, and two tested releases.
- [x] Concise explanation of ecosystem importance without implying adoption; real usage signals remain future evidence.
- [ ] Maintainer has reviewed the official terms and supplied email/org ID.

## Suggested supporting narrative

Harness Lens grew from a concrete developer problem: as Agent Harnesses accumulate repository instructions, user rules, skills, hooks, memory, and loops, it becomes difficult to know what is present and what each part contains. The project turns that into inspectable evidence, then extends the same boundary to runtime activity. Its purpose is not to add another Agent chat or orchestrator, but to make Agent systems easier to understand, reproduce, evaluate, and improve.

The initial release is intentionally narrow. It provides useful local inspection and experimental Codex thread forensics while clearly documenting what it cannot prove. Historical runs are not yet bound to immutable context snapshots, and completed turns are not treated as task success. The next milestone adds that binding, followed by independent verifier evidence and meaningful comparisons.

## Submission boundary

The maintainer must personally review and submit the official application because it requires personal contact information, an OpenAI organization ID, and acceptance of current program terms. Never fill missing information with guesses, and never describe repository age, users, stars, downloads, or adoption without a public source.
