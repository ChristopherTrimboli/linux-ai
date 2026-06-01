#!/usr/bin/env bash
# Install a desktop entry + themed icons so the app shows its icon in the
# GNOME/Ubuntu dock (including when running `npm run tauri dev`).
#
# On Wayland, GNOME ignores window-set icons and instead matches a window's
# app_id to a .desktop file of the same name. The app sets its app_id to
# `dev.linux-ai.app` (via enableGTKAppId), so we install a matching desktop
# entry and icons named `dev.linux-ai.app`.
#
# Usage: desktop/install-desktop-entry.sh        (install for current user)
#        desktop/install-desktop-entry.sh --uninstall
set -euo pipefail

APP_ID="dev.linux-ai.app"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ICON_SRC="$SCRIPT_DIR/src-tauri/icons"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$DATA_HOME/applications"
ICON_DIR="$DATA_HOME/icons/hicolor"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "$APP_DIR/$APP_ID.desktop"
  for s in 32x32 128x128 256x256 512x512; do
    rm -f "$ICON_DIR/$s/apps/$APP_ID.png"
  done
  echo "Removed desktop entry and icons for $APP_ID."
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
  command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f "$ICON_DIR" 2>/dev/null || true
  exit 0
fi

# Pick the best available binary for the Exec line (prefer an installed one,
# then a locally built release/debug binary, else just the name on PATH).
BIN="linux-ai-desktop"
for candidate in \
  "$(command -v linux-ai-desktop 2>/dev/null || true)" \
  "$REPO_ROOT/target/release/linux-ai-desktop" \
  "$REPO_ROOT/target/debug/linux-ai-desktop"; do
  if [[ -n "$candidate" && -x "$candidate" ]]; then
    BIN="$candidate"
    break
  fi
done

# Install icons into the hicolor theme under the app-id name.
declare -A SIZES=(
  ["32x32"]="32x32.png"
  ["128x128"]="128x128.png"
  ["256x256"]="128x128@2x.png"
  ["512x512"]="icon.png"
)
for size in "${!SIZES[@]}"; do
  src="$ICON_SRC/${SIZES[$size]}"
  if [[ -f "$src" ]]; then
    mkdir -p "$ICON_DIR/$size/apps"
    cp -f "$src" "$ICON_DIR/$size/apps/$APP_ID.png"
  fi
done

# Install the desktop entry with the resolved Exec path.
mkdir -p "$APP_DIR"
sed "s|^Exec=.*|Exec=$BIN|" "$SCRIPT_DIR/$APP_ID.desktop" > "$APP_DIR/$APP_ID.desktop"
chmod +x "$APP_DIR/$APP_ID.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f "$ICON_DIR" 2>/dev/null || true

echo "Installed $APP_ID.desktop -> $APP_DIR"
echo "Exec=$BIN"
echo "If the dock icon doesn't update, log out/in or restart the app."
