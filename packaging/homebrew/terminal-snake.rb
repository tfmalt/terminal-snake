class TerminalSnake < Formula
  desc "Retro cross-platform terminal Snake game"
  homepage "https://github.com/tfmalt/terminal-snake"
  version "0.6.26-rc1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tfmalt/terminal-snake/releases/download/v0.6.26-rc1/terminal-snake-v0.6.26-rc1-aarch64-apple-darwin.tar.gz"
      sha256 "e2fac528f3ea4bcf1c0cda1450aae186bb97ff872376147f504ed779d4779e6d"
    else
      url "https://github.com/tfmalt/terminal-snake/releases/download/v0.6.26-rc1/terminal-snake-v0.6.26-rc1-x86_64-apple-darwin.tar.gz"
      sha256 "db040627e7306884a0e186f41fa1836015fe7f40e34a7644acd4affce5ad8a6c"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/tfmalt/terminal-snake/releases/download/v0.6.26-rc1/terminal-snake-v0.6.26-rc1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "6b94d2e2b4afb4b983d5091c10969df6cd2f27227f993a83177e131e4ea83ad0"
    else
      url "https://github.com/tfmalt/terminal-snake/releases/download/v0.6.26-rc1/terminal-snake-v0.6.26-rc1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "4f10236337e72ed9d20c2f731e290bacb97eae115cd58344fff8f51a88cde181"
    end
  end

  def install
    binary = Dir["**/terminal-snake"].first
    odie "terminal-snake binary not found in release archive" if binary.nil?

    bin.install binary => "terminal-snake"
  end

  test do
    system "#{bin}/terminal-snake", "--help"
  end
end
