// `u` — short alias binary for `ukrop`.
// Shares the same entry point; clap picks up the binary name automatically
// so `u --help` shows "u" and `u --version` shows the correct version.

fn main() -> anyhow::Result<()> {
    ukrop::run()
}
