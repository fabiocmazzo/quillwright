#!/usr/bin/env bash
# Package Quillwright.app as a universal (Apple Silicon + Intel) .dmg in dist/.
# Run on macOS after building both targets:
#   cargo build --release --target aarch64-apple-darwin
#   cargo build --release --target x86_64-apple-darwin
# Usage: packaging/macos/build.sh <version>
set -euo pipefail

version=${1:?usage: packaging/macos/build.sh <version>}
root=$(cd "$(dirname "$0")/../.." && pwd)
dist="$root/dist"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$dist"

app="$work/dmg/Quillwright.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create \
  "$root/target/aarch64-apple-darwin/release/quillwright" \
  "$root/target/x86_64-apple-darwin/release/quillwright" \
  -output "$app/Contents/MacOS/quillwright"
sed "s/@VERSION@/$version/g" "$root/packaging/macos/Info.plist" > "$app/Contents/Info.plist"

# Icon: build an .iconset from the 512 px PNG, then convert it to .icns.
iconset="$work/Quillwright.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z $size $size "$root/assets/icon/quillwright.png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
done
for size in 16 32 128 256; do
  double=$((size * 2))
  sips -z $double $double "$root/assets/icon/quillwright.png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/Quillwright.icns"

# Ad-hoc signature: not notarized, but lets the app run on Apple Silicon.
codesign --force --deep --sign - "$app"

ln -s /Applications "$work/dmg/Applications"
hdiutil create -volname Quillwright -srcfolder "$work/dmg" -ov -format UDZO \
  "$dist/Quillwright-$version-macos-universal.dmg"

ls -l "$dist"
