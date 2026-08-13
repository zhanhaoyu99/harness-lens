# Aggregate compatibility report

Harness Lens can produce a reviewable, aggregate-only report for a real Codex or Claude Harness setup without opening the desktop app:

```bash
pnpm compatibility-report -- /path/to/workspace
```

The default output is Markdown so it can be reviewed and pasted into the [compatibility-report issue form](https://github.com/zhanhaoyu99/harness-lens/issues/new?template=compatibility_report.yml). Use `--json` when reducing a provider compatibility problem to a fixture or another local tool:

```bash
pnpm compatibility-report -- --json /path/to/workspace
```

## Output contract

Report schema `1` contains only:

- Harness Lens version;
- validated Harness Lens source-checkout HEAD observed when the report runs, and whether that checkout was dirty;
- operating-system family and executable target architecture;
- total discovered-artifact count;
- aggregate counts by provider, Harness kind, and static resolution state;
- aggregate diagnostic counts by severity;
- whether the filesystem scan was complete.

It intentionally excludes:

- workspace name, absolute or relative paths, and Git branch;
- artifact names, descriptions, contents, previews, artifact/content hashes, sizes, and timestamps;
- warning identifiers, titles, details, or affected artifact identifiers;
- Memory text, prompts, reasoning, tool arguments, file diffs, and runtime payloads.

`sourceRevision` is the HEAD observed when the report runs in the Harness Lens source checkout located from the command's build manifest; it does not identify the scanned workspace. It is attribution context, not proof that the revision exactly built a previously compiled binary. `sourceDirty` describes that same checkout and makes local modifications explicit. Either value is `null` (shown as `unknown` in Markdown) when it cannot be established. Git errors, repository paths, branches, remotes, changed-file names, and diff content never enter the report.

Provider, kind, and resolution map keys use the stable lower-camel-case values enumerated in the JSON Schema rather than implementation debug names.

Tests serialize a snapshot containing representative private values and assert that neither Markdown nor JSON output contains them. They also compare the Rust serializer's required fields and stable label sets with the published JSON Schema. This is a narrow serialization guarantee, not a claim that aggregate metadata is anonymous.

## Privacy and evidence boundary

Counts can still reveal information about a developer setup. Review the complete output before posting it publicly, and do not append raw logs, screenshots, paths, prompts, Memory text, or secrets.

The report proves only what the selected scanner version discovered and could statically resolve during that scan. It does not prove that an Agent used an item, that a run had the same Harness context, or that a task succeeded.

## Versioning

`reportSchemaVersion` changes only when the serialized field contract changes. Additive or breaking schema changes require a fixture-backed privacy test, documentation update, and changelog entry. Provider compatibility remains versioned evidence rather than a blanket support claim.

The machine-readable contract is published as [JSON Schema](schemas/compatibility-report-v1.schema.json).
