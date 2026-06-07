# Releasing SnipTeX

This is the maintainer guide for cutting a new SnipTeX release. End users
should read `docs/install-guide.md` instead.

---

## Prerequisites (one-time)

1. **Signing key generated.** Run
   `npx tauri signer generate -p "" -w ~/.tauri/sniptex.key` once.
   Keep `~/.tauri/sniptex.key` safe — back it up to a password manager.
   Losing it means users can never auto-update from existing installs
   without you shipping a new pubkey + telling everyone to manually
   reinstall.
2. **GitHub Secrets configured** on `hiep1987/sniptex`:
   - `TAURI_SIGNING_PRIVATE_KEY` — full contents of `~/.tauri/sniptex.key`
     (the file, not the path).
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the password you set during
     `tauri signer generate`. Empty string if you used `-p ""`.
3. **Updater pubkey baked in.** The pubkey in
   `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) must match the
   `~/.tauri/sniptex.key.pub` file generated alongside the private key.
   It already is — don't touch it once shipped to users.
4. **Updater is "always live".** Tauri 2's updater config has no
   `active` master switch — once `endpoints` + `pubkey` are set, the
   plugin will try to hit the configured URL whenever the JS code calls
   `check()`. Until the first release tag is published, that URL 404s
   and the in-app **Check for updates** button surfaces "Update check
   unavailable" (expected). After the first release lands, the button
   starts returning real `Update` objects automatically.

---

## Cutting a release

### 1. Bump versions

Three files must agree:

- `package.json` → `"version": "X.Y.Z"`
- `src-tauri/tauri.conf.json` → `"version": "X.Y.Z"`
- `src-tauri/Cargo.toml` → `version = "X.Y.Z"` (under `[package]`)

```bash
# Edit all three to the new version, then:
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "chore(release): bump to vX.Y.Z"
```

### 2. Push the tag

```bash
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

### 3. Watch CI

`.github/workflows/release.yml` triggers on the tag. The matrix runs
three parallel build jobs:

| Runner | Target | Artifact |
|--------|--------|----------|
| `macos-14` | `aarch64-apple-darwin` | `SnipTeX_X.Y.Z_aarch64.dmg` + `.app.tar.gz` + `.app.tar.gz.sig` |
| `macos-13` | `x86_64-apple-darwin` | `SnipTeX_X.Y.Z_x64.dmg` + `.app.tar.gz` + `.app.tar.gz.sig` |
| `windows-latest` | `x86_64-pc-windows-msvc` | `SnipTeX_X.Y.Z_x64-setup.msi` + `.msi.zip` + `.msi.zip.sig` |

Then `checksums` job downloads all of the above and uploads
`checksums.txt` to the same draft release.

Total wall time is typically 12–18 minutes per job; the Mac jobs run in
parallel.

### 4. Review the draft release

The workflow creates the release as a **draft**. Visit
<https://github.com/hiep1987/sniptex/releases> →
**Draft** → edit the release notes (replace the auto-generated body),
verify all artifacts are present:

- 2× DMG (aarch64 + x64)
- 2× `.app.tar.gz` + `.app.tar.gz.sig` (Mac updater payload, per arch)
- 1× MSI
- 1× `.msi.zip` + `.msi.zip.sig` (Windows updater payload)
- 1× `latest.json` (the updater manifest)
- 1× `checksums.txt`

Click **Publish release**.

### 5. Verify the updater feed

```bash
curl -L https://github.com/hiep1987/sniptex/releases/latest/download/latest.json | jq
```

Should print a JSON object with `version`, `platforms.{darwin-aarch64,darwin-x86_64,windows-x86_64}`,
each containing a signed download URL. If `latest.json` 404s, the release
isn't published (still draft) or `includeUpdaterJson: true` was missed in
the workflow.

### 6. (Optional) Hand-test the upgrade path

On a machine running the previous version, open Settings → About →
**Check for updates**. The UpdateDialog should appear; clicking
**Update now** downloads the signed bundle, applies it, and prompts to
restart.

---

## Signing-key rotation

If the private key is compromised or lost, you cannot recover
auto-updates for existing installs — you'd be shipping a new pubkey,
and Tauri rejects signatures from a key the installed pubkey doesn't
verify.

Mitigation: ship a new release with the **old** binary path (manual
reinstall) and post-launch notice in release notes telling users to
download fresh from GitHub Releases. Then proceed with a new keypair:

1. `rm ~/.tauri/sniptex.key{,.pub}` (after backing up the compromised
   key to a forensics archive if relevant).
2. `npx tauri signer generate -p "" -w ~/.tauri/sniptex.key` — generate
   a fresh keypair.
3. Replace `pubkey` in `src-tauri/tauri.conf.json` with the new
   `~/.tauri/sniptex.key.pub` contents.
4. Update both GitHub Secrets (`TAURI_SIGNING_PRIVATE_KEY` and
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).
5. Tag + release as normal. Users on the previous pubkey must manually
   download once before the auto-updater works again.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Release workflow fails on `tauri-action` step with "signing key missing" | GitHub Secrets not set | Add the two `TAURI_SIGNING_PRIVATE_KEY*` Secrets and re-run |
| Mac Intel job (`macos-15`) deprecated by GitHub | Image deprecation cycle | Bump to the next macos-NN Intel image; macos-26 is next |
| Mac ARM job (`macos-15-arm64`) deprecated by GitHub | Image deprecation cycle | Bump to the next macos-NN-arm64 image; macos-26-arm64 is next |
| Windows build fails on WiX install step | Missing WiX prerequisites on `windows-latest` | Add `wix-toolset` action before tauri-action |
| `latest.json` missing from release | `includeUpdaterJson: true` flag removed | Re-add to `release.yml` |
| In-app "Check for updates" shows "Update check unavailable" before first release | Endpoint URL 404s because no release published yet | Expected pre-launch; goes away after first release tag |
| Release tag pushed but no workflow run | Tag pushed before workflow file landed in main | Push the workflow first, then re-tag |
