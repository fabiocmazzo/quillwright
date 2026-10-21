# Arch Linux (AUR) packages

Two packages, each in its own folder with a `PKGBUILD` and a generated `.SRCINFO`:

| Package | Builds from | Notes |
| --- | --- | --- |
| `quillwright` | the source archive of the `vX.Y.Z` tag | Compiles with cargo (about 8 minutes). Works on x86_64 and aarch64. |
| `quillwright-bin` | the release `.tar.gz` | Installs the prebuilt binary. x86_64 only. Conflicts with `quillwright`. |

Each AUR package is its own Git repository on `aur.archlinux.org`. These folders are the
source of truth; copy them into the AUR clones when publishing.

## Publishing for the first time

1. Create an account at <https://aur.archlinux.org/register> and add your **SSH public key**
   in "My Account".
2. Add to `~/.ssh/config`:

   ```
   Host aur.archlinux.org
     IdentityFile ~/.ssh/id_ed25519
     User aur
   ```

3. For each package (cloning a name that does not exist yet creates an empty repository):

   ```bash
   git clone ssh://aur@aur.archlinux.org/quillwright.git aur-quillwright
   cp packaging/arch/quillwright/{PKGBUILD,.SRCINFO} aur-quillwright/
   cd aur-quillwright
   makepkg -si            # test: build and install
   namcap PKGBUILD *.pkg.tar.zst   # optional lint (pacman -S namcap)
   git add PKGBUILD .SRCINFO
   git commit -m "Initial upload: quillwright 0.4.0"
   git push
   ```

   The AUR repository's branch must be `master`, and it may contain only the files the
   package needs (no built packages; add a `.gitignore` if you like).

## Updating for a new release

After the GitHub release is published:

```bash
cd packaging/arch/quillwright-bin      # and the same in quillwright/
sed -i 's/^pkgver=.*/pkgver=X.Y.Z/; s/^pkgrel=.*/pkgrel=1/' PKGBUILD
updpkgsums                              # from pacman-contrib: refreshes sha256sums
makepkg --printsrcinfo > .SRCINFO
makepkg -f                              # test the build
```

Then copy `PKGBUILD` and `.SRCINFO` into the AUR clone, commit ("Update to X.Y.Z"), and
push. Bump only `pkgrel` when the packaging changes but the version does not.
