class Ukrop < Formula
  desc "Quick directory jumping & command execution with fuzzy TUI"
  homepage "https://github.com/ukroporg/ukrop"
  version "__VERSION__"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/ukroporg/ukrop/releases/download/v#{version}/ukrop-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA_DARWIN_ARM__"
    end
    on_intel do
      url "https://github.com/ukroporg/ukrop/releases/download/v#{version}/ukrop-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA_DARWIN_X86__"
    end
  end

  on_linux do
    url "https://github.com/ukroporg/ukrop/releases/download/v#{version}/ukrop-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "__SHA_LINUX__"
  end

  def install
    bin.install "ukrop", "u"
  end

  def caveats
    <<~EOS
      To activate ukrop, add the following to your shell config:

      # For bash (~/.bashrc):
      eval "$(ukrop init bash)"

      # For zsh (~/.zshrc):
      eval "$(ukrop init zsh)"
    EOS
  end

  test do
    assert_match "Quick directory jumping", shell_output("#{bin}/ukrop --help")
  end
end
