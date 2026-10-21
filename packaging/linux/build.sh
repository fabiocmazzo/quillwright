#!/usr/bin/env bash
# Package target/release/quillwright as an AppImage and a .tar.gz in dist/.
# Usage: packaging/linux/build.sh <version>
set -euo pipefail

version=${1:?usage: packaging/linux/build.sh <version>}
arch=x86_64
root=$(cd "$(dirname "$0")/../.." && pwd)
dist="$root/dist"
binary="$root/target/release/quillwright"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

[ -x "$binary" ] || { echo "Build first: cargo build --release" >&2; exit 1; }
mkdir -p "$dist"
id=io.github.fabiocmazzo.quillwright
desktop="$root/packaging/linux/$id.desktop"
metainfo="$root/packaging/linux/$id.metainfo.xml"

# --- .tar.gz: the binary plus desktop integration files ---
tarball="quillwright-$version-linux-$arch"
mkdir -p "$work/$tarball"
cp "$binary" "$work/$tarball/"
cp "$desktop" "$metainfo" "$root/README.md" "$root/LICENSE" "$work/$tarball/"
cp "$root/assets/icon/quillwright.png" "$work/$tarball/$id.png"
tar -C "$work" -czf "$dist/$tarball.tar.gz" "$tarball"

# --- AppImage ---
appdir="$work/Quillwright.AppDir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" "$appdir/usr/share/metainfo" \
         "$appdir/usr/share/icons/hicolor/512x512/apps" \
         "$appdir/usr/share/icons/hicolor/scalable/apps"
install -m 755 "$binary" "$appdir/usr/bin/quillwright"
install -m 644 "$desktop" "$appdir/usr/share/applications/"
install -m 644 "$desktop" "$appdir/"
install -m 644 "$metainfo" "$appdir/usr/share/metainfo/"
install -m 644 "$root/assets/icon/quillwright.png" "$appdir/usr/share/icons/hicolor/512x512/apps/$id.png"
install -m 644 "$root/assets/icon/quillwright.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/$id.svg"
install -m 644 "$root/assets/icon/quillwright.png" "$appdir/$id.png"
ln -s "$id.png" "$appdir/.DirIcon"
ln -s usr/bin/quillwright "$appdir/AppRun"

tool=${APPIMAGETOOL:-}
if [ -z "$tool" ]; then
  tool="$work/appimagetool"
  curl -fsSL -o "$tool" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
  chmod +x "$tool"
fi
# Extract-and-run avoids needing FUSE on build machines.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=$arch "$tool" --no-appstream "$appdir" \
  "$dist/Quillwright-$version-$arch.AppImage"

ls -l "$dist"
