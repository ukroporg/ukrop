# Development

## Build & Run

```sh
make build      # debug build
make release    # release build
make test       # run all tests
make install    # install to ~/.cargo/bin
make setup      # install + run ukrop setup --force
make clean      # remove target/
```

Or use cargo directly:

```sh
cargo build              # debug build
cargo test               # run all tests
cargo build --release    # optimized build
```

For a one-command dev rebuild that reloads your shell, add this to your `~/.zshrc`:

```sh
ukrop-dev() {
    cargo install --path . --force && source ~/.zshrc && ukrop setup --force
}
```

## Project Structure

```text
src/
  main.rs          — entry point (thin wrapper calling ukrop::run())
  bin/u.rs         — `u` shortcut binary (same entry point)
  cli.rs           — clap CLI definition
  lib.rs           — library: module re-exports and run() with subcommand dispatch
  config.rs        — TOML configuration file, theme/layout structs, save support
  util.rs          — XDG paths, path helpers
  frecency.rs      — frecency scoring (exponential decay)
  db/
    migrate.rs     — schema migrations (v1, v2, v3) with column-existence checks
    model.rs       — DirEntry, CmdEntry, SshHostEntry structs
    store.rs       — SQLite operations with transactions, batch imports, stale cleanup
  history/
    bash.rs        — bash history parser
    zsh.rs         — zsh history parser
    fish.rs        — fish history parser (YAML-like format)
  shell/
    bash.rs        — bash init script template
    zsh.rs         — zsh init script template
    fish.rs        — fish init script template
  ssh/
    config.rs      — ~/.ssh/config parser
  tui/
    app.rs         — TUI state machine, panel state, event loop
    ui.rs          — ratatui rendering (3-panel layout, preview, match highlighting, config overlay)
    input.rs       — key bindings (cursor movement, word delete, Home/End, config dialog keys)
    fuzzy.rs       — nucleo two-tier matcher (substring + fuzzy) with match positions
    tty_reader.rs  — direct /dev/tty input reader (Home/End/Delete key support)
    theme.rs       — color theme resolution from ThemePreset to ratatui styles
    config_dialog.rs — in-TUI config editor modal (field types, navigation, validation)
tests/             — integration tests
packaging/         — Homebrew formula and deb packaging
```

## Packaging

### Homebrew

```sh
brew tap gupalo/tap https://github.com/gupalo/homebrew-tap
brew install gupalo/tap/ukrop
```

Formula template at `packaging/homebrew/ukrop.rb`.

### Debian/Ubuntu

```sh
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/ukrop_0.1.0_*.deb
```

The deb package includes both `ukrop` and `u` binaries.
