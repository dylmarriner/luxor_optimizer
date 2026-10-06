#!/usr/bin/env bash
# Stage the helper as a Tauri sidecar.
#
# Tauri resolves externalBin entries as <path>-<target-triple>, so the built
# binary has to be copied to that exact name before `tauri build` runs. Without
# this the build fails with "resource path ... doesn't exist".
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
SRC="$ROOT/target/release/luxor-helper"
DEST="$ROOT/src-tauri/binaries/luxor-helper-$TRIPLE"

if [[ ! -f "$SRC" ]]; then
  echo "helper not built; run: cargo build --release -p luxor-helper" >&2
  exit 1
fi

mkdir -p "$(dirname "$DEST")"
install -m 755 "$SRC" "$DEST"
echo "Staged $DEST"
