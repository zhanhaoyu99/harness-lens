# Release process

Harness Lens uses Semantic Versioning and publishes intentionally conservative macOS artifacts. v0.1 targets Apple Silicon and is ad-hoc signed, not Apple-notarized.

## Preconditions

- All intended changes are merged to `main`.
- CI is green on the release commit.
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
4. Run the **Draft macOS release** workflow with that existing tag.
5. Download the workflow artifact and verify its SHA-256 file.
6. Test the exact DMG on a clean machine.
7. Review generated notes, supported architecture, minimum macOS version, signing/notarization state, and known limitations.
8. Publish the draft release manually only after those checks pass.

The workflow deliberately creates a **draft** release. It does not auto-publish an untested desktop binary.

## Checksums

For a local artifact:

```bash
shasum -a 256 "Harness-Lens_0.1.1_aarch64.dmg" > Harness-Lens_0.1.1_aarch64.dmg.sha256
```

Keep the filename in the checksum file identical to the uploaded asset name.

## Post-release

- Install from the public release and repeat the smoke test.
- Confirm the README download link and checksum instructions.
- Create a milestone for the next version.
- Close only issues that the released artifact actually fixes.
- Open a follow-up issue for every known release limitation not already tracked.
- Record any runtime compatibility range supported by the tested Codex CLI versions.

Apple Developer ID signing/notarization and additional architectures require their own tracked work; do not silently relabel an ad-hoc build as signed for distribution.
