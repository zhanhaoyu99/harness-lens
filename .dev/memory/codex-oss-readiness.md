# Codex for Open Source readiness

## Durable objective

Build Harness Lens into a genuinely useful, actively maintained open-source project with verifiable adoption and ecosystem value. One desired result is a stronger Codex for Open Source application, but user value and trustworthy maintenance remain the primary objective. Never buy, exchange, automate, or fabricate stars, downloads, contributors, testimonials, or usage.

## Published application signals

As verified on **2026-08-28**, OpenAI's [Codex for Open Source page](https://developers.openai.com/community/codex-for-oss) and [Program Terms](https://learn.chatgpt.com/docs/codex-for-oss-terms) say it may consider repository usage, broad adoption or ecosystem importance, active maintenance, and the applicant's maintainer role or permissions. There is no published star threshold or disclosed scoring rubric.

## Application state

- Official application submitted on 2026-08-12; the form displayed its receipt confirmation.
- Applicant role: primary maintainer.
- Requested benefits: Codex Security and API credits.
- Do not submit duplicate applications unless OpenAI asks for one or publishes a supported update flow.
- Personal contact information and the OpenAI organization ID remain outside the repository.

## Evidence baseline — 2026-08-28

- Public MIT repository with 100% GitHub community-profile health.
- Public synthetic demo and checksummed macOS arm64 releases through v0.4.0.
- Main CI covers frontend tests/build, Rust format/Clippy/tests, Rust 1.88 MSRV, npm audit, and RustSec audit.
- Public maintenance loop exists: issue #10 -> PR #11 -> v0.1.1.
- Current public adoption signals: 0 stars, 0 forks, 0 watchers, 0 external human contributors, 0 external issues, and no completed compatibility reports. Release asset counters and clone traffic are too early and maintainer/automation-influenced to claim adoption.
- The latest public release remains v0.4.0; before the current local iteration, PR #20 head `b3eeae9` had five green CI/CodeQL checks but remained blocked by the required-review rule. The current single-maintainer CODEOWNERS/collaborator structure cannot satisfy a real independent approval. Do not bypass that protected-branch rule without explicit user authorization, weaken it for convenience, or use another account as a synthetic reviewer.
- The submitted ChatGPT-account mailbox contains no OpenAI or Codex for Open Source follow-up as of 2026-08-28. The official materials publish no review SLA or application-update flow.
- Public Discussions #21 and openai/codex #40309 still have no independent comments or reactions. Their only upvotes are attributable to the maintainer. Release download counts are unchanged and cannot be treated as independent use.
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
4. Invite relevant coding-agent maintainers only through public, opt-in GitHub/open-source channels to try the post-install 10-minute in-app workflow and submit time-to-first-value plus qualitative friction; do not use workplace/private channels or ask only for stars.
5. Remove the largest remaining distribution barrier through Developer ID signing/notarization when credentials are available; otherwise prioritize execution-time run/snapshot binding based on real feedback.

## Current unpublished candidate — 2026-08-28

- Local commit `66e89e0` improves first-screen positioning, search/social metadata, a privacy-conscious compatibility issue form, and preserves submitted versus post-submission application evidence.
- The v0.5.0 source candidate adds a reproducible 31-second synthetic tour and refreshes screenshots from the v0.4 browser-only demo.
- The local candidate now adds deterministic, local-only configuration comprehension: exact full-file physical line counts, conservative position and purpose summaries, a dedicated diagnostics column, and Inspector evidence/recommendations. Quality diagnostics cover guidance strictly over 200 lines, missing Skill descriptions, empty non-Memory files, truncated previews, and Codex repository/nested instruction files at or above the documented default 32 KiB combined project-instruction budget. The 200-line rule is explicitly a Harness Lens maintainability heuristic, not a provider limit, quality score, or success-rate prediction.
- Final review narrowed missing-description checks to actual `SKILL.md` manifests, excluded shadowed Codex instructions from the 32 KiB warning, normalized common YAML block-scalar descriptions, made cross-scanner snapshot diagnostic changes explicitly attribution-unknown, and made each scan artifact derive size/hash/line count from one verified open-file revision; concurrent replacement now marks the scan incomplete.
- Static workspace scans and explicit Capture persistence now finish without waiting for the independent Codex runtime refresh. Runtime loading/error ownership remains isolated by its existing sequence and workspace-path guards.
- Product positioning now names searchable Codex and Claude Code configuration-inspection use cases. The complete desktop app remains the primary workbench; CLI/Doctor, Codex plugin, DeepSeek Harness plugin, and macOS menu-bar/widget surfaces are documented as future complementary entry points, not shipped features. DeepSeek Harness remains a developer-preview compatibility target.
- A new `compatibility-report` CLI produces a source-attributed, versioned aggregate Markdown or JSON projection. Its allowlist excludes workspace/artifact names, paths, branch, content, previews, artifact/content hashes, sizes, timestamps, diagnostic text, and runtime payloads; fixture and Schema-contract tests lock the serialization boundary. Counts remain potentially sensitive and require manual review.
- The desktop Share candidate now removes the source-build prerequisite for feedback: a no-argument backend command fresh-scans the authorized workspace's saved files, previews the same schema-v1 report, and copies only after review. It does not persist the report, replace the live Inventory/allowlists, or discard/include unsaved Memory drafts; the browser example remains synthetic evidence only.
- The PR #20 head has green GitHub Actions, JavaScript/TypeScript, and Rust CodeQL checks. Release automation separates read-only builds from a minimally privileged, SHA-pinned attestation/draft job for future DMGs. CodeQL is not yet on `main`, and DMG provenance is not public evidence until the PR merges and the release workflow succeeds.
- The `glib 0.18.5` advisory is absent from the shipped `aarch64-apple-darwin` graph but present in the Linux GTK/WebKit graph. Keep the alert visible and block future Linux distribution until upgraded or otherwise remediated.
- Full local candidate verification: frontend 50 tests and production build passed; Rust format/strict Clippy passed; 76 library tests plus 3 compatibility-report CLI tests passed. A headless 980 px visual check confirmed all four core inventory columns remain visible without horizontal scrolling and that the Inspector independently scrolls through longer diagnostics. Earlier workflow/issue YAML, generated Tauri permission schemas, JSON Schema, Share checks, and local v0.5.0 app/DMG verification remain candidate evidence, not a public release.
- Repository discovery metadata now names both Codex and Claude configuration use cases and includes focused `claude-code`, `agent-harness`, `agent-observability`, `developer-tools`, `rust`, and `macos` topics. This improves discoverability but is not adoption evidence.
- [Discussion #21](https://github.com/zhanhaoyu99/harness-lens/discussions/21) publishes a privacy-safe 10-minute in-app early-adopter workflow for the current v0.4.0 release; installation and first launch are outside the measurement. A focused issue form and README CTA are in the candidate branch. The invitation itself is not adoption evidence.
- [openai/codex Show and tell #40309](https://github.com/openai/codex/discussions/40309) presents the v0.4.0 visual inventory → Codex runtime metadata → explicit Capture → Saved-to-Saved Compare loop with the same evidence, privacy, distribution, and no-Star boundaries. It is the first public, opt-in community publication for the experiment; publication alone is not adoption evidence.
- Candidate merge and release publication remain pending. Do not describe any candidate item above as shipped until its public PR, `main` CI, and release evidence exist.

## Decision log

- Optimize for real usage and ecosystem importance, not a guessed star threshold.
- A reviewer may inspect the live repository, but there is no official guarantee that post-submission changes will be considered.
- Do not claim that release asset downloads represent independent users until external evidence supports that conclusion.
- Public promotion must be useful, targeted, and non-spammy; show the problem, workflow, and evidence boundary rather than asking only for stars. Workplace/private-channel outreach, including Slack invitations to colleagues, is explicitly out of scope.
- Do not submit to an ecosystem directory when its hard inclusion rules do not fit, its submission requires a human action the agent cannot truthfully perform, or the directory is inactive enough that a PR would only manufacture activity.

## Next adoption experiment

From 2026-08-24 through 2026-08-31, run the public [10-minute early-adopter validation](https://github.com/zhanhaoyu99/harness-lens/discussions/21) with developers who already maintain Codex or Claude project context. The clock starts only after Harness Lens is installed and launched. Collect in-app time-to-first-value, one previously unknown finding (or none), the first blocking step, intent to reuse, and only privacy-safe public details. Promotion stays in public, opt-in GitHub/open-source channels; workplace and private-channel outreach are out of scope. The project Discussion and [official Codex community post](https://github.com/openai/codex/discussions/40309) now exist; independent completions remain pending. Success is at least two independent completed workflows or one feedback-driven issue-to-fix-to-release loop—not Stars, clones, or maintainer-generated downloads.

## Resume protocol

At the start of future work, read this file, `docs/CODEX-FOR-OSS-APPLICATION.md`, the latest release/CI state, and current GitHub adoption signals. Update the dated baseline only from live evidence. Record completed work and the next concrete action before ending a substantial iteration.
