# Release process

Harness Lens uses Semantic Versioning and publishes intentionally conservative macOS artifacts. The current distribution targets Apple Silicon and is ad-hoc signed, not Apple-notarized.

## Preconditions

- All intended changes are merged to `main`.
- CI is green on the release commit.
- The latest CodeQL analysis completed successfully, and every Code scanning alert that reaches the release boundary has been explicitly triaged. A green workflow run only means analysis and result upload succeeded; it does not mean CodeQL reported zero alerts.
- `CHANGELOG.md` contains the release date, user-visible changes, known limitations, and privacy/security changes.
- Versions match in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- The release boundary in both READMEs is still accurate.
- A clean checkout can build the app with the locked dependencies.

## Local validation

```bash
pnpm install --frozen-lockfile
pnpm test
pnpm build
sh scripts/with-rust.sh cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
sh scripts/with-rust.sh cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm rust:test
pnpm tauri build
```

Inspect the produced `.app` and `.dmg` on a clean Apple Silicon Mac. Validate the exact signing state rather than assuming a successful bundle means a trusted distribution:

```bash
codesign --verify --deep --strict --verbose=2 "path/to/Harness Lens.app"
codesign -dv --verbose=4 "path/to/Harness Lens.app"
spctl --assess --type execute --verbose=4 "path/to/Harness Lens.app"
```

An ad-hoc-signed, non-notarized build is expected to fail normal Gatekeeper distribution assessment. That limitation must remain visible in release notes.

## Tag and draft release

1. Commit the version and changelog update.
2. Create a signed or annotated tag: `git tag -a vX.Y.Z -m "Harness Lens vX.Y.Z"`.
3. Push the commit and tag.
4. Run the **Draft macOS release** workflow from `main` with that existing tag. The workflow rejects dispatches from other refs, checks out the fully qualified tag, records and verifies that `HEAD` resolves to its peeled commit, requires that commit to be reachable from `main`, and rejects a tag that moves between build and draft creation.
5. Download the workflow artifact, verify its SHA-256 file, and verify the DMG's GitHub build-provenance attestation.
6. Test the exact DMG on a clean machine.
7. Review generated notes, supported architecture, minimum macOS version, signing/notarization state, and known limitations.
8. Publish the draft release manually only after those checks pass.

The workflow deliberately creates a **draft** release. It does not auto-publish an untested desktop binary.

## Checksums

For a local artifact:

```bash
version="$(node -p 'require("./package.json").version')"
asset="Harness-Lens_${version}_aarch64.dmg"
shasum -a 256 "$asset" > "$asset.sha256"
```

Keep the filename in the checksum file identical to the uploaded asset name.

## Build provenance

The release workflow creates a GitHub artifact attestation for each DMG it builds. After downloading the DMG, verify that its digest was produced by this repository's release workflow:

```bash
gh attestation verify "$asset" \
  --repo zhanhaoyu99/harness-lens \
  --signer-workflow zhanhaoyu99/harness-lens/.github/workflows/release.yml \
  --source-ref refs/heads/main \
  --deny-self-hosted-runners
```

This verification constrains the repository, signer-workflow identity, and `main` workflow source ref recorded by GitHub, and rejects attestations from self-hosted runners. It does not make the build reproducible, sign it with an Apple Developer ID, notarize it, or make Gatekeeper trust it. Releases created before the attestation step was introduced do not retroactively gain provenance.

## Post-release

- Install from the public release and repeat the smoke test.
- Confirm the README download link and checksum instructions.
- Create a milestone for the next version.
- Close only issues that the released artifact actually fixes.
- Open a follow-up issue for every known release limitation not already tracked.
- Record any runtime compatibility range supported by the tested Codex CLI versions.

Apple Developer ID signing/notarization and additional architectures require their own tracked work; do not silently relabel an ad-hoc build as signed for distribution. Trusted macOS distribution will require an Apple Developer Program identity, the Developer ID certificate and private key, and protected notarization credentials. Keep those credentials out of the repository and introduce them only through a reviewed release environment.
