class Ukrop < Formula
  desc "Quick directory jumping & command execution with fuzzy TUI"
  homepage "https://github.com/ukroporg/ukrop"
  url "https://github.com/ukroporg/ukrop/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std.cargo_args, "--root", prefix, "--path", "."
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
