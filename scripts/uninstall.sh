#!/usr/bin/env bash
set -euo pipefail

rm -f "$HOME/.local/share/applications/luxor-optimizer.desktop"
rm -f "$HOME/.local/opt/luxor/LuxorOptimizer.AppImage"
rmdir --ignore-fail-on-non-empty "$HOME/.local/opt/luxor" || true

echo "Luxor Optimizer removed from user profile. Audit logs under ~/.local/share/luxor-optimizer are preserved by default."
