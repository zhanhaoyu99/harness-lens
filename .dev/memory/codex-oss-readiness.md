# Codex for Open Source readiness

## Durable objective

Build Harness Lens into a genuinely useful, actively maintained open-source project with verifiable adoption and ecosystem value. One desired result is a stronger Codex for Open Source application, but user value and trustworthy maintenance remain the primary objective. Never buy, exchange, automate, or fabricate stars, downloads, contributors, testimonials, or usage.

## Published application signals

As verified on **2026-08-13**, OpenAI's [application page](https://openai.com/form/codex-for-oss/) and [Program Terms](https://learn.chatgpt.com/docs/codex-for-oss-terms) say it may consider repository usage, broad adoption or ecosystem importance, active maintenance, and the applicant's maintainer role or permissions. Stars and downloads are examples of supporting evidence on the form, not published thresholds or a disclosed scoring rubric.

## Application state

- Official application submitted on 2026-08-12; the form displayed its receipt confirmation.
- Applicant role: primary maintainer.
- Requested benefits: Codex Security and API credits.
- Do not submit duplicate applications unless OpenAI asks for one or publishes a supported update flow.
- Personal contact information and the OpenAI organization ID remain outside the repository.

## Evidence baseline — 2026-08-24

- Public MIT repository with 100% GitHub community-profile health.
- Public synthetic demo and checksummed macOS arm64 releases through v0.4.0.
- Main CI covers frontend tests/build, Rust format/Clippy/tests, Rust 1.88 MSRV, npm audit, and RustSec audit.
- Public maintenance loop exists: issue #10 -> PR #11 -> v0.1.1.
- Current public adoption signals: 0 stars, 0 forks, 0 watchers, 0 external human contributors, 0 external issues, and no completed compatibility reports. Release asset counters and clone traffic are too early and maintainer/automation-influenced to claim adoption.
- The latest public release remains v0.4.0; PR #20 remains open with five green CI/CodeQL checks but is blocked by the required-review rule. Do not bypass that protected-branch rule without explicit user authorization.
- The submitted ChatGPT-account mailbox contains no OpenAI or Codex for Open Source follow-up as of 2026-08-24. The official materials publish no review SLA or application-update flow.
- Primary risk: project age and lack of independently verifiable users, feedback, issues, or integrations.

## Workstreams

1. **Product value** — make Codex/Claude Harness state, run paths, changes, and later verifier evidence materially easier to understand.
2. **Adoption** — remove install friction, publish a short truthful demo, invite targeted feedback, and document real use cases.
3. **Discoverability** — maintain accurate GitHub description/topics, search-oriented README language, releases, and public examples.
4. **Maintenance evidence** — triage real issues, review PRs, ship tested releases, maintain compatibility/security evidence, and avoid manufactured activity.
5. **Application evidence** — keep the public application record current while preserving the exact boundary of what was submitted versus what changed later.

## Near-term priorities

1. Obtain a real approval for PR #20 or explicit authorization for the repository's administrator-bypass path, then publish the v0.5.0 candidate through green `main` CI, release verification, and installation evidence.
2. Verify the first real CodeQL run and the next release attestation before recording either as public evidence.
3. Update GitHub repository description and topics without making adoption claims.
4. Invite a small set of relevant coding-agent maintainers to try the public 10-minute workflow and submit time-to-first-value plus qualitative friction; do not ask only for stars.
5. Remove the largest remaining distribution barrier through Developer ID signing/notarization when credentials are available; otherwise prioritize execution-time run/snapshot binding based on real feedback.

## Current unpublished candidate — 2026-08-24

- Local commit `66e89e0` improves first-screen positioning, search/social metadata, a privacy-conscious compatibility issue form, and preserves submitted versus post-submission application evidence.
- The v0.5.0 source candidate adds a reproducible 31-second synthetic tour and refreshes screenshots from the v0.4 browser-only demo.
- A new `compatibility-report` CLI produces a source-attributed, versioned aggregate Markdown or JSON projection. Its allowlist excludes workspace/artifact names, paths, branch, content, previews, artifact/content hashes, sizes, timestamps, diagnostic text, and runtime payloads; fixture and Schema-contract tests lock the serialization boundary. Counts remain potentially sensitive and require manual review.
- The desktop Share candidate now removes the source-build prerequisite for feedback: a no-argument backend command fresh-scans the authorized workspace's saved files, previews the same schema-v1 report, and copies only after review. It does not persist the report, replace the live Inventory/allowlists, or discard/include unsaved Memory drafts; the browser example remains synthetic evidence only.
- The PR #20 head has green GitHub Actions, JavaScript/TypeScript, and Rust CodeQL checks. Release automation separates read-only builds from a minimally privileged, SHA-pinned attestation/draft job for future DMGs. CodeQL is not yet on `main`, and DMG provenance is not public evidence until the PR merges and the release workflow succeeds.
- The `glib 0.18.5` advisory is absent from the shipped `aarch64-apple-darwin` graph but present in the Linux GTK/WebKit graph. Keep the alert visible and block future Linux distribution until upgraded or otherwise remediated.
- Full local candidate verification: frontend 43 tests and production build passed; Rust format/strict Clippy passed; 68 library tests plus 3 compatibility-report CLI tests passed; workflow/issue YAML, generated Tauri permission schemas, JSON Schema, responsive Share checks, and diff checks passed. A local v0.5.0 arm64 app/DMG was built, its strict ad-hoc signature and bundle metadata were verified, and `hdiutil verify` passed; this is candidate evidence, not a public release.
- Repository discovery metadata now names both Codex and Claude configuration use cases and includes focused `claude-code`, `agent-harness`, `agent-observability`, `developer-tools`, `rust`, and `macos` topics. This improves discoverability but is not adoption evidence.
- [Discussion #21](https://github.com/zhanhaoyu99/harness-lens/discussions/21) publishes a privacy-safe 10-minute early-adopter workflow for the current v0.4.0 release. A focused issue form and README CTA are in the candidate branch. The invitation itself is not adoption evidence.
- External publication remains pending. Do not describe any candidate item above as shipped until its public PR, CI, and release evidence exist.

## Decision log

- Optimize for real usage and ecosystem importance, not a guessed star threshold.
- A reviewer may inspect the live repository, but there is no official guarantee that post-submission changes will be considered.
- Do not claim that release asset downloads represent independent users until external evidence supports that conclusion.
- Public promotion must be useful, targeted, and non-spammy; show the problem, workflow, and evidence boundary rather than asking only for stars.

## Next adoption experiment

From 2026-08-24 through 2026-08-31, run the public [10-minute early-adopter validation](https://github.com/zhanhaoyu99/harness-lens/discussions/21) with 5–8 developers who already maintain Codex or Claude project context. Collect time-to-first-value, one previously unknown finding (or none), the first blocking step, intent to reuse, and only privacy-safe public details. The public destination now exists; targeted invitations and independent completions remain pending. Success is at least two independent completed workflows or one feedback-driven issue-to-fix-to-release loop—not Stars, clones, or maintainer-generated downloads.

## Resume protocol

At the start of future work, read this file, `docs/CODEX-FOR-OSS-APPLICATION.md`, the latest release/CI state, and current GitHub adoption signals. Update the dated baseline only from live evidence. Record completed work and the next concrete action before ending a substantial iteration.
