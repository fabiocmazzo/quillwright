# Windows Package Manager (winget)

`manifests/f/FabioCovoloMazzo/Quillwright/<version>/` holds the three manifest files for
[microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs), in the same layout as
that repository. Users install with:

```powershell
winget install FabioCovoloMazzo.Quillwright
```

The package is the release `.zip`, installed as a *portable* app: winget extracts it,
adds the `quillwright` command to the PATH, and lists it for upgrades and removal. It does
**not** create a Start menu shortcut (portable packages never do); that needs a real
installer (MSI or MSIX), which Quillwright does not have yet.

## Submitting a version

1. Validate on Windows: `winget validate --manifest <version folder>`, then test with
   `winget install --manifest <version folder>` (needs `winget settings --enable
   LocalManifestFiles` in an administrator shell once).
2. Fork microsoft/winget-pkgs, copy the version folder to the same path, and open a pull
   request titled `New package: FabioCovoloMazzo.Quillwright version <version>` (later:
   `New version: ...`). Bots validate it and scan the download; a moderator merges it.

For later versions, [`wingetcreate`](https://github.com/microsoft/winget-create) writes and
submits the update from the new download link:

```powershell
wingetcreate update FabioCovoloMazzo.Quillwright --version <version> `
  --urls https://github.com/fabiocmazzo/quillwright/releases/download/v<version>/quillwright-<version>-windows-x86_64.zip `
  --submit
```

Copy the result back here so this folder stays the source of truth. `RelativeFilePath`
contains the version, because the zip has a versioned top folder.
