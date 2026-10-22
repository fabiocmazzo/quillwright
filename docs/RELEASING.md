# Releasing Quillwright

Releases are built by GitHub Actions ([`.github/workflows/release.yml`](../.github/workflows/release.yml)) when a version tag is pushed.

1. Update `version` in `Cargo.toml` (for example `0.2.0`) and run `cargo build` so `Cargo.lock` picks it up.
   Add a section for the version to `CHANGELOG.md`; it is used as the release notes.
   Add a `<release version="..." date="...">` with a short summary at the top of
   `<releases>` in `packaging/linux/io.github.fabiocmazzo.quillwright.metainfo.xml` (software
   stores show it), and check the file with `appstreamcli validate --pedantic`.
   For the first release after 0.4.0, also update the AUR `PKGBUILD`s: the desktop file is
   now `io.github.fabiocmazzo.quillwright.desktop`, the icons are named after the app id,
   and the `.tar.gz` contains `io.github.fabiocmazzo.quillwright.png` instead of
   `quillwright.png` (see `packaging/linux/build.sh`). Install the MetaInfo file to
   `/usr/share/metainfo/` too.
2. Commit, then tag and push. The tag must match the version in `Cargo.toml`, or the workflow stops:

   ```bash
   git commit -am "Release 0.2.0"
   git tag v0.2.0
   git push && git push --tags
   ```

3. The workflow builds:

   | File | Built on |
   | --- | --- |
   | `Quillwright-<version>-x86_64.AppImage` | Ubuntu 22.04 |
   | `quillwright-<version>-linux-x86_64.tar.gz` | Ubuntu 22.04 |
   | `quillwright-<version>-windows-x86_64.zip` | Windows |
   | `Quillwright-<version>-macos-universal.dmg` | macOS (Apple Silicon + Intel in one app) |
   | `SHA256SUMS.txt` | |

4. The files are attached to a **draft** release with generated notes. Review it on the Releases page, then publish it.
5. After publishing, update the package managers with the new download links and hashes
   (from `SHA256SUMS.txt`): winget ([packaging/winget](../packaging/winget/README.md)) and
   the Homebrew tap ([packaging/homebrew](../packaging/homebrew/README.md)).

## Building packages locally

```bash
# Linux (AppImage and .tar.gz in dist/)
cargo build --release
packaging/linux/build.sh 0.2.0

# macOS (.dmg in dist/)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
packaging/macos/build.sh 0.2.0
```

`packaging/linux/build.sh` downloads `appimagetool` unless `APPIMAGETOOL` points to a copy.

## Flathub and the Snap Store

See [docs/linux-stores.md](linux-stores.md).

## Code signing

The Windows and macOS builds are not signed with a paid certificate, so users see a warning on first launch (the README explains how to continue). The macOS app is ad-hoc signed, which lets it run on Apple Silicon. Signing and notarization can be added to the workflow later with an Apple Developer ID and a Windows code-signing certificate.

## The icon

`assets/icon/quillwright.svg` is the source. After editing it, regenerate the other files:

```bash
rsvg-convert -w 512 assets/icon/quillwright.svg -o assets/icon/quillwright.png
rsvg-convert -w 128 assets/icon/quillwright.svg -o assets/icon/quillwright-128.png   # window icon
for s in 16 24 32 48 64 128 256; do rsvg-convert -w $s assets/icon/quillwright.svg -o /tmp/icon-$s.png; done
magick /tmp/icon-{16,24,32,48,64,128,256}.png assets/icon/quillwright.ico            # Windows
```
