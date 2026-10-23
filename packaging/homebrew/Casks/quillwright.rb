cask "quillwright" do
  version "0.6.0"
  sha256 "65d1eda4c666ba7dc12dabc4fc7fbe12cb8d99f37b1ab3e0a006b8d3e06c9184"

  url "https://github.com/fabiocmazzo/quillwright/releases/download/v#{version}/Quillwright-#{version}-macos-universal.dmg"
  name "Quillwright"
  desc "Writing app for long documents in plain Markdown, alongside AI agents"
  homepage "https://fabiocmazzo.github.io/quillwright/"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :big_sur"

  app "Quillwright.app"

  zap trash: "~/Library/Application Support/com.quillwright.quillwright"

  caveats <<~EOS
    Quillwright is not signed with an Apple Developer ID yet. If macOS says the app
    is damaged or cannot be opened, run once:
      xattr -dr com.apple.quarantine "#{appdir}/Quillwright.app"
  EOS
end
