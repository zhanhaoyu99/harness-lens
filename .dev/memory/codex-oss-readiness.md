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

## Evidence baseline — 2026-08-13

- Public MIT repository with 100% GitHub community-profile health.
- Public synthetic demo and five checksummed macOS arm64 releases through v0.4.0.
- Main CI covers frontend tests/build, Rust format/Clippy/tests, Rust 1.88 MSRV, npm audit, and RustSec audit.
- Public maintenance loop exists: issue #10 -> PR #11 -> v0.1.1.
- Current public adoption signals: 0 stars, 0 forks, 0 watchers, 0 external contributors; release downloads are too early and maintainer-influenced to claim adoption.
- Primary risk: project age and lack of independently verifiable users, feedback, issues, or integrations.

## Workstreams

1. **Product value** — make Codex/Claude Harness state, run paths, changes, and later verifier evidence materially easier to understand.
2. **Adoption** — remove install friction, publish a short truthful demo, invite targeted feedback, and document real use cases.
3. **Discoverability** — maintain accurate GitHub description/topics, search-oriented README language, releases, and public examples.
4. **Maintenance evidence** — triage real issues, review PRs, ship tested releases, maintain compatibility/security evidence, and avoid manufactured activity.
5. **Application evidence** — keep the public application record current while preserving the exact boundary of what was submitted versus what changed later.

## Near-term priorities

1. Publish the refreshed application evidence and README first-screen positioning through review and green CI.
2. Update GitHub repository description and topics without making adoption claims.
3. Produce a 30–60 second synthetic demo and a focused feedback invitation.
4. Invite a small set of relevant coding-agent maintainers to try one concrete workflow and report friction; do not ask only for stars.
5. Choose the next product slice based on adoption leverage: lower distribution friction or execution-time run/snapshot binding.

## Decision log

- Optimize for real usage and ecosystem importance, not a guessed star threshold.
- A reviewer may inspect the live repository, but there is no official guarantee that post-submission changes will be considered.
- Do not claim that release asset downloads represent independent users until external evidence supports that conclusion.
- Public promotion must be useful, targeted, and non-spammy; show the problem, workflow, and evidence boundary rather than asking only for stars.

## Resume protocol

At the start of future work, read this file, `docs/CODEX-FOR-OSS-APPLICATION.md`, the latest release/CI state, and current GitHub adoption signals. Update the dated baseline only from live evidence. Record completed work and the next concrete action before ending a substantial iteration.
