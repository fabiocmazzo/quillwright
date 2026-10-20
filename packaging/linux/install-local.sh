#!/usr/bin/env bash
# Install Quillwright for the current user, with its menu entry and icon.
# Wayland desktops ignore a window's own icon: they look up the .desktop file
# that matches the app id ("quillwright") and its icon in the icon theme.
#
# Usage: packaging/linux/install-local.sh              # install target/release/quillwright
#        packaging/linux/install-local.sh --uninstall  # remove what was installed
# Set PREFIX to install somewhere other than ~/.local.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
prefix=${PREFIX:-$HOME/.local}
bin="$prefix/bin/quillwright"
desktop="$prefix/share/applications/quillwright.desktop"
icons="$prefix/share/icons/hicolor"
png="$icons/512x512/apps/quillwright.png"
svg="$icons/scalable/apps/quillwright.svg"

refresh() {
  if command -v update-desktop-database >/dev/null; then
    update-desktop-database "$prefix/share/applications" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -f -t "$icons" 2>/dev/null || true
  fi
}

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$bin" "$desktop" "$png" "$svg"
  refresh
  echo "Removed Quillwright from $prefix."
  exit 0
elif [ -n "${1:-}" ]; then
  echo "usage: $0 [--uninstall]" >&2
  exit 2
fi

binary="$root/target/release/quillwright"
[ -x "$binary" ] || { echo "Build first: cargo build --release" >&2; exit 1; }

install -Dm755 "$binary" "$bin"
install -Dm644 "$root/assets/icon/quillwright.png" "$png"
install -Dm644 "$root/assets/icon/quillwright.svg" "$svg"
# Launchers do not always have ~/.local/bin on their PATH, so point the menu
# entry at the installed binary.
mkdir -p "$(dirname "$desktop")"
sed -e "s|^Exec=quillwright|Exec=\"$bin\"|" -e "/^Exec=/a TryExec=$bin" \
  "$root/packaging/linux/quillwright.desktop" >"$desktop"
chmod 644 "$desktop"
refresh

echo "Installed Quillwright in $prefix."
case ":$PATH:" in
  *":$prefix/bin:"*) ;;
  *) echo "Note: $prefix/bin is not on your PATH, so 'quillwright' only works from the app menu." ;;
esac
echo "If the icon does not show yet, log out and back in."
