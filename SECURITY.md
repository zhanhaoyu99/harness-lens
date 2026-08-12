# Security policy

Harness Lens reads local developer configuration and runtime metadata, so privacy and path safety are release-critical concerns.

## Supported versions

| Version | Supported |
|---|---|
| Latest `0.1.x` release | Yes |
| Older pre-release builds | No |

Until the project reaches 1.0, security fixes are shipped in the newest patch release rather than backported.

## Report a vulnerability privately

Please use **Report a vulnerability** in the repository's Security tab to open a private GitHub Security Advisory. Do not include secrets or private run data in a public issue.

Include only the minimum information needed to reproduce the problem:

- affected version and macOS architecture;
- threat scenario and expected boundary;
- synthetic reproduction steps or fixture;
- observed impact;
- any suggested mitigation.

You should receive an acknowledgement within 7 days and an initial assessment within 14 days. These are goals, not a paid support SLA.

## Important product boundaries

- Redaction is best-effort and is not a data-loss-prevention guarantee.
- Current release artifacts are ad-hoc signed and not Apple-notarized.
- The Codex App Server adapter is experimental and may encounter schema changes.
- Harness Lens is designed to be read-only, but it still processes files chosen by the user; a malicious repository should be treated as untrusted input.
- The app does not promise isolation from another malicious process running as the same operating-system user.

See [Privacy](docs/PRIVACY.md) and the [Threat model](docs/THREAT-MODEL.md) for details.
