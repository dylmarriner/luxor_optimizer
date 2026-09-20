#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "[1/4] Installing frontend deps"
npm ci

echo "[2/4] Building frontend"
npm run build

echo "[3/4] Building and staging helper"
cargo build --release -p luxor-helper
./scripts/stage-helper.sh

echo "[4/4] Building Tauri AppImage"
(
  cd src-tauri
  cargo tauri build --bundles appimage
)

echo "Done. Check src-tauri/target/release/bundle/appimage"
