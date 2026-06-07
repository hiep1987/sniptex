#!/usr/bin/env bash
#
# generate-latest-json.sh — Build the Tauri-updater manifest `latest.json`
# from the .sig files alongside the release artifacts, and upload it to
# the matching GitHub release.
#
# Usage:
#   scripts/generate-latest-json.sh <tag> <artifact-dir>
#
# Expected layout under <artifact-dir>:
#   SnipTeX_aarch64.app.tar.gz                 (Mac ARM updater payload)
#   SnipTeX_aarch64.app.tar.gz.sig             (Mac ARM signature)
#   SnipTeX_<ver>_x64-setup.nsis.zip           (Windows updater payload)
#   SnipTeX_<ver>_x64-setup.nsis.zip.sig       (Windows signature)
#
# Skips a platform if its .sig file is missing. Writes `latest.json` into
# <artifact-dir> and uploads via `gh release upload`.
#
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: scripts/generate-latest-json.sh <tag> <artifact-dir>" >&2
  exit 64
fi

TAG="$1"
DIR="$2"
VER="${TAG#v}"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
BASE="https://github.com/hiep1987/sniptex/releases/download/${TAG}"

if [[ ! -d "$DIR" ]]; then
  echo "error: not a directory: $DIR" >&2
  exit 1
fi

read_sig() {
  local path="$1"
  [[ -f "$path" ]] || return 1
  tr -d '\r\n' < "$path"
}

mac_payload="SnipTeX_aarch64.app.tar.gz"
mac_sig_path="$DIR/${mac_payload}.sig"
win_payload="$(ls "$DIR"/SnipTeX_*_x64-setup.nsis.zip 2>/dev/null | head -1 || true)"
win_sig_path=""
[[ -n "$win_payload" && -f "${win_payload}.sig" ]] && win_sig_path="${win_payload}.sig"

mac_sig=""
if [[ -f "$mac_sig_path" ]]; then
  mac_sig="$(read_sig "$mac_sig_path")"
else
  echo "warning: missing $mac_sig_path — skipping mac entry" >&2
fi

win_sig=""
if [[ -n "$win_sig_path" ]]; then
  win_sig="$(read_sig "$win_sig_path")"
else
  echo "warning: missing windows .nsis.zip.sig — skipping windows entry" >&2
fi

if [[ -z "$mac_sig" && -z "$win_sig" ]]; then
  echo "error: no platform signatures found in $DIR; aborting" >&2
  exit 1
fi

OUT="$DIR/latest.json"

{
  printf '{\n'
  printf '  "version": "%s",\n' "$VER"
  printf '  "notes": "See the release notes for changes.",\n'
  printf '  "pub_date": "%s",\n' "$PUB_DATE"
  printf '  "platforms": {\n'

  first=1
  if [[ -n "$mac_sig" ]]; then
    [[ $first -eq 1 ]] || printf ',\n'
    printf '    "darwin-aarch64": {\n'
    printf '      "url": "%s/%s",\n' "$BASE" "$mac_payload"
    printf '      "signature": "%s"\n' "$mac_sig"
    printf '    }'
    first=0
  fi
  if [[ -n "$win_sig" ]]; then
    [[ $first -eq 1 ]] || printf ',\n'
    win_basename="$(basename "$win_payload")"
    printf '    "windows-x86_64": {\n'
    printf '      "url": "%s/%s",\n' "$BASE" "$win_basename"
    printf '      "signature": "%s"\n' "$win_sig"
    printf '    }'
    first=0
  fi
  printf '\n  }\n'
  printf '}\n'
} > "$OUT"

echo "wrote $OUT:"
cat "$OUT"

if command -v gh >/dev/null 2>&1; then
  gh release upload "$TAG" "$OUT" --clobber
else
  echo "gh CLI not available; skipping upload (latest.json still written locally)"
fi
