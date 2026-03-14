#!/usr/bin/env bash
set -euo pipefail

APPIMAGE_PATH="${1:-}"
if [[ -z "$APPIMAGE_PATH" ]]; then
  echo "usage: $0 /path/to/LuxorOptimizer.AppImage"
  exit 1
fi

INSTALL_DIR="$HOME/.local/opt/luxor"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
mkdir -p "$INSTALL_DIR" "$APP_DIR" "$ICON_DIR"

cp "$APPIMAGE_PATH" "$INSTALL_DIR/LuxorOptimizer.AppImage"
chmod +x "$INSTALL_DIR/LuxorOptimizer.AppImage"

cat > "$APP_DIR/luxor-optimizer.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Luxor Optimizer
Exec=$INSTALL_DIR/LuxorOptimizer.AppImage
Icon=utilities-system-monitor
Categories=Utility;System;
Terminal=false
EOF

echo "Installed Luxor Optimizer to $INSTALL_DIR"
