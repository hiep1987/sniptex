#!/usr/bin/env bash
#
# generate-checksums.sh — Emit SHA256 checksums for release artifacts.
#
# Scans a directory for SnipTeX release artifacts (.dmg, .msi, .app.tar.gz,
# .msi.zip, .nsis.zip) and writes `checksums.txt` containing one
# `<sha256>  <filename>` line per artifact (sorted by filename).
#
# Cross-platform: uses `shasum -a 256` on macOS, `sha256sum` on Linux. On
# Windows CI we run the Ubuntu post-step, so plain `sha256sum` is fine.
#
# Usage:
#   scripts/generate-checksums.sh <artifact-dir>
#
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: scripts/generate-checksums.sh <artifact-dir>" >&2
  exit 64
fi

DIR="$1"

if [[ ! -d "$DIR" ]]; then
  echo "error: not a directory: $DIR" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  HASH_CMD="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  HASH_CMD="shasum -a 256"
else
  echo "error: neither sha256sum nor shasum available" >&2
  exit 1
fi

OUT="$DIR/checksums.txt"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

# Match the artifacts tauri-action emits. Skip the checksums file itself
# in case this is re-run.
shopt -s nullglob
cd "$DIR"
for f in *.dmg *.msi *.app.tar.gz *.msi.zip *.nsis.zip; do
  [[ "$f" == "checksums.txt" ]] && continue
  $HASH_CMD "$f" >> "$TMP"
done

if [[ ! -s "$TMP" ]]; then
  echo "warning: no matching artifacts found in $DIR" >&2
  : > "$OUT"
  exit 0
fi

sort -k2 "$TMP" > "$OUT"
echo "wrote $(wc -l <"$OUT" | tr -d ' ') checksums to $OUT"
cat "$OUT"
