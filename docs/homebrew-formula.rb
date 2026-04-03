# This file goes in a separate repo: SingggggYee/homebrew-tap
# Users install with: brew install SingggggYee/tap/ccwhy

class Ccwhy < Formula
  desc "Claude Code usage debugger — tells you why your tokens burned"
  homepage "https://github.com/SingggggYee/ccwhy"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/SingggggYee/ccwhy/releases/download/v0.1.0/ccwhy-macos-aarch64.tar.gz"
      # sha256 "UPDATE_AFTER_RELEASE"
    else
      url "https://github.com/SingggggYee/ccwhy/releases/download/v0.1.0/ccwhy-macos-x86_64.tar.gz"
      # sha256 "UPDATE_AFTER_RELEASE"
    end
  end

  on_linux do
    url "https://github.com/SingggggYee/ccwhy/releases/download/v0.1.0/ccwhy-linux-x86_64.tar.gz"
    # sha256 "UPDATE_AFTER_RELEASE"
  end

  def install
    bin.install "ccwhy"
  end

  test do
    # Just check it runs (will exit 1 if no claude data, but that's ok)
    system "#{bin}/ccwhy", "--version"
  end
end
