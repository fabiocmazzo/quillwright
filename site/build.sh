#!/bin/sh
# Assembles the website in _site/ (or the folder given): the pages in site/ plus the
# screenshots, icon, and fonts from the repository, so they are not kept twice.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-"$root/_site"}
rm -rf "$out"
mkdir -p "$out/img" "$out/fonts"
cp -R "$root/site/." "$out/"
rm -f "$out/build.sh"
cp "$root/screenshots/paper.png" "$root/screenshots/night.png" "$root/screenshots/agent-sync.gif" "$out/img/"
cp "$root/assets/icon/quillwright.svg" "$out/img/icon.svg"
cp "$root/assets/icon/quillwright.png" "$out/img/icon.png"
for font in Literata-Regular Literata-Italic AtkinsonHyperlegible-Regular AtkinsonHyperlegible-Bold; do
  cp "$root/assets/fonts/$font.ttf" "$out/fonts/"
done
cp "$root/assets/fonts/licenses/"* "$out/fonts/" 2>/dev/null || true
echo "Website assembled in $out"
