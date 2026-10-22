# Homebrew cask (macOS)

`Casks/quillwright.rb` installs the release `.dmg`. It is published in a separate
repository, a *tap*, named `fabiocmazzo/homebrew-tap`. Users install with:

```bash
brew install fabiocmazzo/tap/quillwright
```

The official `homebrew/cask` repository needs a more popular project and, under its newer
rules, apps signed and notarized with an Apple Developer ID. Until then the tap is the way.
The app is not signed, so the cask prints how to clear the quarantine flag.

## Setting up the tap

1. Create the public GitHub repository `fabiocmazzo/homebrew-tap`.
2. Copy `Casks/quillwright.rb` into it, at `Casks/quillwright.rb`, commit, and push.
3. On a Mac, check it and install it:

   ```bash
   brew tap fabiocmazzo/tap
   brew style fabiocmazzo/tap
   brew audit --cask --strict --online fabiocmazzo/tap/quillwright
   brew install --cask fabiocmazzo/tap/quillwright
   ```

## Each release

Update `version` and `sha256` (the `.dmg` line in the release's `SHA256SUMS.txt`) here,
then copy the file to the tap. `brew bump-cask-pr --version <version> fabiocmazzo/tap/quillwright`
can do the edit for you.
