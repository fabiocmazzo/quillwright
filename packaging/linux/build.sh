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

# Libraries that winit loads at run time (dlopen) and that minimal systems may lack, such
# as libxkbcommon-x11 for the keyboard on X11. Without them the app panics at startup.
# They go in usr/lib, found through RUNPATH ($ORIGIN) rather than LD_LIBRARY_PATH, which
# would leak into the programs the app starts (the assistant, lint tools, xdg-open).
# libX11, libxcb, GL, GTK, and glibc come from the host, as AppImages expect.
bundled="libxkbcommon.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1 libX11-xcb.so.1
         libXcursor.so.1 libXrandr.so.2 libXi.so.6 libXrender.so.1 libXfixes.so.3 libXext.so.6"
command -v patchelf >/dev/null || { echo "patchelf is needed to build the AppImage" >&2; exit 1; }
mkdir -p "$appdir/usr/lib"
for lib in $bundled; do
  path=$(/sbin/ldconfig -p | awk -v l="$lib" '$1 == l && /x86-64/ { print $NF; exit }')
  [ -n "$path" ] || { echo "Missing $lib on the build machine" >&2; exit 1; }
  cp -L "$path" "$appdir/usr/lib/$lib"
  chmod 644 "$appdir/usr/lib/$lib"
  patchelf --set-rpath '$ORIGIN' "$appdir/usr/lib/$lib"
done
patchelf --set-rpath '$ORIGIN/../lib' "$appdir/usr/bin/quillwright"

tool=${APPIMAGETOOL:-}
if [ -z "$tool" ]; then
  tool="$work/appimagetool"
  curl -fsSL -o "$tool" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
  chmod +x "$tool"
fi
# Update information: AppImageUpdate and similar tools look for the newest GitHub release
# and download only the changed blocks, using the .zsync file that appimagetool writes
# next to the AppImage (the release workflow attaches everything in dist/).
update="gh-releases-zsync|fabiocmazzo|quillwright|latest|Quillwright-*-$arch.AppImage.zsync"
# Extract-and-run avoids needing FUSE on build machines.
(cd "$dist" && APPIMAGE_EXTRACT_AND_RUN=1 ARCH=$arch "$tool" --no-appstream -u "$update" \
  "$appdir" "Quillwright-$version-$arch.AppImage")
[ -f "$dist/Quillwright-$version-$arch.AppImage.zsync" ] ||
  { echo "appimagetool did not write the .zsync file" >&2; exit 1; }

ls -l "$dist"
