# Homebrew formula for Knightingale.
#
#   brew tap ShatilKhan/tap
#   brew install knightingale
#
# Maintained at https://github.com/ShatilKhan/homebrew-tap (planned).
# The placeholder SHAs are updated by the release workflow.

class Knightingale < Formula
  desc "Voice dictation daemon that minds its own business"
  homepage "https://shatilkhan.github.io/knightingale/"
  license "MIT"
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/ShatilKhan/knightingale/releases/download/v#{version}/knightingale-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/ShatilKhan/knightingale/releases/download/v#{version}/knightingale-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/ShatilKhan/knightingale/releases/download/v#{version}/knightingale-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/ShatilKhan/knightingale/releases/download/v#{version}/knightingale-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "knightingale"
    bin.install "knightingale-daemon"
  end

  service do
    run [opt_bin/"knightingale-daemon"]
    keep_alive true
    log_path var/"log/knightingale.log"
    error_log_path var/"log/knightingale.err.log"
  end

  test do
    assert_match "knightingale", shell_output("#{bin}/knightingale --version")
  end
end
