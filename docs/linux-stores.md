# Flathub and the Snap Store

How Quillwright is prepared for the two Linux app stores, and what is left to publish it.
Written in October 2026; check each store's current rules before submitting.

## What the repository provides

| File | Purpose |
| --- | --- |
| `packaging/linux/io.github.fabiocmazzo.quillwright.desktop` | Menu entry. Its name, `Icon=`, and `StartupWMClass=` are the app id. |
| `packaging/linux/io.github.fabiocmazzo.quillwright.metainfo.xml` | The store listing (AppStream MetaInfo): summary, description, screenshots, license, releases. Install it to `share/metainfo/`. |
| `assets/icon/quillwright.svg`, `quillwright.png` (512 px) | The icon. Install it as `share/icons/hicolor/scalable/apps/io.github.fabiocmazzo.quillwright.svg` (and the PNG under `512x512/apps/`). |
| `LICENSE` | MIT. Flathub expects it in `share/licenses/io.github.fabiocmazzo.quillwright/`. |

The **app id** is `io.github.fabiocmazzo.quillwright`. Flathub requires `io.github.<user>.`
for projects hosted on GitHub, matching `https://github.com/fabiocmazzo/quillwright`. The
window uses the same id (`APP_ID` in `src/main.rs`), so docks match it to the desktop file.

Check the files with:

```bash
appstreamcli validate --pedantic packaging/linux/io.github.fabiocmazzo.quillwright.metainfo.xml
desktop-file-validate packaging/linux/io.github.fabiocmazzo.quillwright.desktop
```

Add a `<release>` to the MetaInfo file for every version (see `docs/RELEASING.md`).

## The sandbox

Both stores run apps in a sandbox. Two things in Quillwright depend on the rest of the
system:

1. **The assistant runs programs the user installed** (`claude`, `codex`, `ollama`, or any
   command), and so do the lint and format tools. Inside a Flatpak those programs are not
   visible. `process::user_program` detects a Flatpak (`/.flatpak-info`) and starts them on
   the host with `flatpak-spawn --host --watch-bus --directory=<project>`, so stdin, stdout,
   cancelling, and the project folder still work. This needs the permission
   `--talk-name=org.freedesktop.Flatpak`, which lets the app run host commands. Flathub
   reviewers ask why an app needs it; the reason is that the assistant is the user's own AI
   tool, which handles its own sign-in, and Quillwright stores no keys. A strictly confined
   snap cannot run host programs at all, so the snap should use **classic** confinement
   (which needs approval from the Snap Store) or ship without a working assistant.
2. **Projects are arbitrary folders** that the app opens, watches for changes made by
   agents, and saves into atomically (temporary file plus rename in the same folder). The
   file chooser portal only grants single files, so the app needs access to the home folder
   (`--filesystem=home` in Flatpak; in a strict snap, the `home` and `removable-media`
   plugs).

Other behavior inside a Flatpak:

- Settings, version history, and caches go to `~/.var/app/io.github.fabiocmazzo.quillwright/`
  instead of `~/.config` and `~/.local/share`, so they start empty.
- System spelling dictionaries (`/usr/share/hunspell`) are not visible. English is built in,
  and other dictionaries can be put in the app's dictionaries folder (**View → Spelling
  Language → Open Dictionaries Folder**).
- Lint tools run on an untitled buffer get a temporary file in the sandbox's `/tmp`, which
  the host cannot see. Saved files work.
- "Open with System Viewer" uses `xdg-open`, which goes through the OpenURI portal.

Not tested yet: the app has not been run inside a Flatpak. The snap is built and started in
CI (below).

## Flathub

Flathub's rules (October 2026) say that the **manifest must not contain AI-generated or
AI-assisted content**, that AI tools must not open or automate the submission pull request
or write its description, commits, or replies, and that AI-generated code, documentation,
or packaging in the app must be **disclosed** in the submission, with the parts affected and
roughly how much. Quillwright's code was written largely with AI assistance, and so were the
files above, so the submission must say so.

The maintainer therefore writes and submits the manifest. The steps:

1. Write `io.github.fabiocmazzo.quillwright.yml`. Points to decide:
   - The runtime must provide GTK 3 (the file dialogs use it), so the GNOME runtime
     (`org.gnome.Platform` / `org.gnome.Sdk`) fits; the Rust toolchain comes from the
     `org.freedesktop.Sdk.Extension.rust-stable` extension.
   - Builds have no network access. Generate `cargo-sources.json` from `Cargo.lock` with
     `flatpak-cargo-generator.py` from
     [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools) and build with
     `cargo --offline build --release`.
   - The source is the release tag archive (or the Git tag and commit).
   - Permissions: Wayland and X11 fallback, GPU (`--device=dri`, the app uses OpenGL),
     `--filesystem=home`, and `--talk-name=org.freedesktop.Flatpak`, as explained above.
   - Install the binary, desktop file, MetaInfo, icons, and license as in the table above.
2. Build and test locally:

   ```bash
   flatpak install -y flathub org.flatpak.Builder
   flatpak run --command=flathub-build org.flatpak.Builder --install io.github.fabiocmazzo.quillwright.yml
   flatpak run io.github.fabiocmazzo.quillwright
   flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest io.github.fabiocmazzo.quillwright.yml
   flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo
   ```

   Test the assistant (it must find `claude` or `ollama` on the host), opening and saving a
   project, autosave, and changes made by an agent appearing in the editor.
3. Fork `flathub/flathub` with "Copy the master branch only" **unchecked**, clone the
   `new-pr` branch, add the manifest and `cargo-sources.json` on a new branch, and open a
   pull request against **`new-pr`** titled "Add io.github.fabiocmazzo.quillwright".
4. Answer the reviewers on the same pull request; comment `bot, build` for a test build.
   After approval, accept the invitation to the new `flathub/io.github.fabiocmazzo.quillwright`
   repository (GitHub two-factor authentication is required). Updates are pull requests
   there.

Submission guide: <https://docs.flathub.org/docs/for-app-authors/submission>;
requirements: <https://docs.flathub.org/docs/for-app-authors/requirements>.

## Snap Store

The snap is defined in `snap/snapcraft.yaml`:

- **Classic confinement**, so the assistant can run the user's AI tools. Publishing it needs
  the Snap Store's approval (step 4 below).
- `base: core24` (Ubuntu 24.04) and the `rust` plugin. The launcher starts the binary
  with the core24 loader and `--library-path`, so it uses the snap's and core24's libraries,
  never the host's (snapcraft's `enable-patchelf` did not patch the binary). GTK 3 (file dialogs) and Mesa (OpenGL)
  are bundled from Ubuntu 24.04; machines with the proprietary NVIDIA driver fall back to
  Mesa's software rendering.
- `snap/local/launch` sets the library and data paths for GTK and Mesa. Those variables
  would break the host programs the app starts (the assistant, lint tools, `xdg-open`), so
  `process::command` removes them for child processes (listed in `QUILLWRIGHT_SNAP_ENV`)
  and restores the host's `XDG_DATA_DIRS`.
- The version comes from `Cargo.toml`; the store listing text is in the YAML and mirrors the
  MetaInfo file.

`.github/workflows/snap.yml` builds the snap on Ubuntu 24.04 (on changes to the snap files,
or by hand from the Actions tab), installs it, starts the app under Xvfb, and uploads the
`.snap`, a screenshot, and the app's log as an artifact.

To publish:

1. Create a Snapcraft account (Ubuntu One), then `snapcraft login` and
   `snapcraft register quillwright`.
2. Download the `.snap` from the workflow's artifact (or build it with `snapcraft` on an
   Ubuntu machine with LXD).
3. `snapcraft upload --release=edge quillwright_<version>_amd64.snap`. The first upload of a
   classic snap waits for the request in step 4.
4. Ask for classic confinement on the Snapcraft forum (<https://forum.snapcraft.io>, category
   "store-requests"): the app is a writing editor whose assistant runs the command-line AI
   tools the user installed (Claude Code, Codex, Ollama, or any command), and it opens and
   watches project folders anywhere in the home folder and on removable drives.
5. Test from the edge channel (`sudo snap install quillwright --edge --classic`), then
   promote to `stable` (`snapcraft release quillwright <revision> stable`).

Later, the release workflow can upload each tagged version with
`snapcore/action-publish` and a `SNAPCRAFT_STORE_CREDENTIALS` secret
(`snapcraft export-login`).
