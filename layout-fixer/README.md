# LangFix

LangFix is a macOS menu bar utility that fixes text typed in the wrong keyboard layout by a global hotkey.

## Development

```bash
npm run tauri dev
```

## Release Packaging

The project is prepared for:

- Tauri updater artifacts
- GitHub Releases based distribution
- unsigned macOS distribution without App Store

Updater configuration is committed in [tauri.conf.json](/Users/andreyka/Desktop/Programing/Util/layout-fixer/src-tauri/tauri.conf.json) and points to:

```text
https://github.com/an6esign/LangFix/releases/latest/download/latest.json
```

## GitHub Actions Release

The repository includes a release workflow at [.github/workflows/release.yml](/Users/andreyka/Desktop/Programing/Util/layout-fixer/.github/workflows/release.yml).

It builds signed macOS artifacts on tag push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Required Secrets

GitHub Actions needs these secrets:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `GITHUB_TOKEN` is provided automatically by GitHub Actions

## Notes

- The updater public key is already committed in the app config, so it does not need to be stored as a secret.
- Only the private updater key must stay secret.
- Without Apple signing and notarization, macOS may show an untrusted source warning on first launch. Auto-updates can still work.
