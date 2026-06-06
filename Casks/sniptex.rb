cask "sniptex" do
  version "0.1.0"
  sha256 "13a5ea48b26fea2e5aba14bade0ef0c833c52e4f5bc1d8425e2e3e13e3515124"

  url "https://github.com/hiep1987/sniptex/releases/download/v#{version}/SnipTeX_#{version}_aarch64.dmg",
      verified: "github.com/hiep1987/sniptex/"
  name "SnipTeX"
  desc "Free OCR snip tool for LaTeX and Markdown"
  homepage "https://github.com/hiep1987/sniptex"

  depends_on macos: :monterey

  app "SnipTeX.app"

  zap trash: [
    "~/Library/Application Support/com.sniptex",
    "~/Library/Application Support/com.sniptex.app",
    "~/Library/Caches/com.sniptex.app",
    "~/Library/Logs/com.sniptex.app",
    "~/Library/Preferences/com.sniptex.app.plist",
    "~/Library/Saved Application State/com.sniptex.app.savedState",
  ]
end
