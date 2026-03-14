#!/usr/bin/env bash
set -euo pipefail

ARTIFACT="${1:-}"
if [[ -z "$ARTIFACT" ]]; then
  echo "usage: $0 path/to/artifact"
  exit 1
fi

sha256sum "$ARTIFACT" > "$ARTIFACT.sha256"
echo "Release digest written to $ARTIFACT.sha256"
