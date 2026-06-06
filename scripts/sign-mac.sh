#!/usr/bin/env bash
#
# sign-mac.sh — Ad-hoc codesign a Tauri-built SnipTeX.app bundle and verify.
#
# Ad-hoc signing (`--sign -`) eliminates the "app is damaged" Gatekeeper
# dialog but does NOT bypass quarantine. End users still need
# "Right-click > Open" the first time, or `xattr -cr /Applications/SnipTeX.app`.
# Full notarization is deferred until the Apple Developer Program is funded
# (see plans/260520-0603-sniptex-tauri-mvp-v1/phase-11-...md).
#
# Usage:
#   scripts/sign-mac.sh                     # signs target/release/bundle/macos/SnipTeX.app
#   scripts/sign-mac.sh path/to/SnipTeX.app # signs the provided bundle
#
set -euo pipefail

APP_PATH="${1:-src-tauri/target/release/bundle/macos/SnipTeX.app}"

if [[ ! -d "$APP_PATH" ]]; then
  echo "error: app bundle not found at: $APP_PATH" >&2
  echo "hint:  run \`npx tauri build --target aarch64-apple-darwin\` first" >&2
  exit 1
fi

echo "==> ad-hoc signing: $APP_PATH"
codesign --sign - --deep --force --timestamp=none "$APP_PATH"

echo "==> verifying signature"
codesign --verify --verbose=2 "$APP_PATH"

echo "==> displaying signing identity"
codesign -dvv "$APP_PATH" 2>&1 | grep -E '^(Identifier|Authority|TeamIdentifier|Signature)' || true

echo
echo "done. ad-hoc signed. Gatekeeper still requires first-launch user consent."
