class TerminalSnake < Formula
  desc "Retro cross-platform terminal Snake game"
  homepage "https://github.com/tfmalt/terminal-snake"
  license "MIT"
  RELEASE = "v0.9.25"
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "39f13fb5ae9d20013d0d0ec5bd56ce11fdba7f864a5620f56b4758d89f2c2469"
    else
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "e9ca375872d7f721e419c051c27a640c2bca18042197d48520a4270a53892ebe"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "a64d89832f24a9c04797745ca48baa9ec8eba00e88dec9ba9c666474759b9e09"
    else
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "cf5b023d8779f5d2ae0fc241a31b3885f0d40278cb8c58a312a259d2325de223"
    end
  end

  def install
    binary = Dir["**/terminal-snake"].first
    odie "terminal-snake binary not found in release archive" if binary.nil?

    bin.install binary => "terminal-snake"
    bin.install_symlink "terminal-snake" => "tsnake"
  end

  test do
    system bin/"terminal-snake", "--help"
  end
end
