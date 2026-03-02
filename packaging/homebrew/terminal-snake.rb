class TerminalSnake < Formula
  desc "Retro cross-platform terminal Snake game"
  homepage "https://github.com/tfmalt/terminal-snake"
  license "MIT"
  RELEASE = "v0.8.16"
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "07272bc9a968c6c76cec11f76bd091c36b6ba5bb7ac464d7fdc981e32020cec3"
    else
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "a4c892a5c6f4d1aec4f2c8028356750648381208cb3039ced7b5f347affd0f97"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "4201e4e7d88d7df939ffd1e572a7b7e5d703087032c953271b62e672fc5e6868"
    else
      url "https://github.com/tfmalt/terminal-snake/releases/download/#{RELEASE}/terminal-snake-#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "5d7bfad62a7cde86ebedd5492af358a5f44cee58609e352ce338c03a2f2b9065"
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
