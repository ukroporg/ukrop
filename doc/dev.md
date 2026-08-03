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
  config.rs        — TOML configuration file, scoring/theme/layout structs, save support
  demo.rs          — demo data generation for screencasts
  util.rs          — XDG paths, path helpers
  frecency.rs      — frecency scoring (exponential decay)
  db/
    migrate.rs     — schema migrations (v1-v5) with column-existence checks
    model.rs       — DirEntry, CmdEntry, SshHostEntry structs
    store.rs       — SQLite operations with transactions, batch imports, stale cleanup
    transitions.rs — transitions table CRUD, per-shell PWD tracking, pruning
  history/
    bash.rs        — bash history parser
    zsh.rs         — zsh history parser
    fish.rs        — fish history parser (YAML-like format)
    powershell.rs  — PowerShell history parser
  shell/
    bash.rs        — bash init script template
    zsh.rs         — zsh init script template
    fish.rs        — fish init script template
    powershell.rs  — PowerShell init script template
  ssh/
    config.rs      — ~/.ssh/config parser
  tui/
    app.rs         — TUI state machine (UnifiedList), event loop
    ranking.rs      — scoring formula and the type-diversity three-way merge (see doc/search.md)
    ui.rs          — ratatui rendering: single ranked list with type sigils (/, $, @), preview, match highlighting, config overlay
    input.rs       — key bindings (cursor movement, type-filter cycling, cwd-filter toggle, config dialog keys)
    fuzzy.rs       — nucleo two-tier matcher (substring + fuzzy) with match positions
    tty_reader.rs  — direct /dev/tty input reader (Home/End/Delete key support)
    theme.rs       — color theme resolution from ThemePreset to ratatui styles, incl. per-type sigil colors
    config_dialog.rs — in-TUI config editor modal (field types, navigation, validation)
    edit_dialog.rs — in-TUI command editor modal (F2)
tests/             — integration tests
packaging/         — Homebrew formula and deb packaging
doc/website/       — static site for ukrop.org (plain HTML/CSS/JS, no build step);
                     gitignored (`/doc/website` in .gitignore) — not tracked in this
                     repository, maintained and deployed outside version control
```

## Testing as a real user (Homebrew)

To switch from a cargo-installed dev build to the Homebrew package and test like an end user:

```sh
# 1. Uninstall the cargo-built version (removes both `ukrop` and `u`)
cargo uninstall ukrop

# 2. Verify they're gone
ls -la ~/.cargo/bin/ukrop ~/.cargo/bin/u 2>/dev/null

# 3. Install via Homebrew
brew install ukroporg/tap/ukrop
# or, to test the local formula:
brew install --build-from-source ./packaging/homebrew/ukrop.rb

# 4. Open a new shell (or `hash -r`) so PATH lookup re-resolves
which -a ukrop u
```

Notes:

- The shell function installed via `eval "$(ukrop init zsh)"` keeps working as long as `ukrop` resolves on PATH. No re-init needed unless the init script itself changed.
- `~/Library/Application Support/ukrop/ukrop.db` and `~/.config/ukrop/config.toml` are not touched by either install method. For a true clean-slate test:
  ```sh
  rm -rf ~/Library/Application\ Support/ukrop ~/.config/ukrop
  ```
- Check PATH order with `echo $PATH | tr ':' '\n' | grep -nE 'cargo|homebrew|/usr/local'` to make sure the Homebrew bin directory comes before `~/.cargo/bin`.

## Packaging

### Homebrew

```sh
brew tap ukroporg/tap https://github.com/ukroporg/homebrew-tap
brew install ukroporg/tap/ukrop
```

The formula installs **prebuilt binaries** from the GitHub release rather than
building from source. This avoids pulling in `rust` + `llvm` + `python` (~600 MB
of build dependencies) on the user's machine — install becomes a few-MB download.

Formula template at `packaging/homebrew/ukrop.rb` uses `__VERSION__`,
`__SHA_DARWIN_ARM__`, `__SHA_DARWIN_X86__`, `__SHA_LINUX__` placeholders that CI
substitutes when bumping the tap.

Release pipeline in `.github/workflows/release-deb.yml` runs on every `v*` tag:

1. **`build-deb`** — runs tests, builds the .deb, uploads to the GitHub release.
2. **`build-binaries`** — matrix job (macOS arm64, macOS x86_64, Linux x86_64).
   For each target it builds with `cargo build --release --target <triple>`,
   tars `ukrop` + `u` into `ukrop-<tag>-<triple>.tar.gz`, computes a
   `.tar.gz.sha256` sidecar, and uploads both to the release.
3. **`bump-homebrew`** — depends on `build-binaries`. Downloads the three
   `.sha256` files from the release, renders the formula by `sed`-substituting
   the placeholders, clones the tap, writes `Formula/ukrop.rb`, commits and
   pushes.

Requires repo secret `HOMEBREW_TAP_TOKEN` — a fine-grained PAT (or GitHub App
token) with `contents: write` on `ukroporg/homebrew-tap`. The default
`GITHUB_TOKEN` cannot push cross-repo.

Adding a new platform: append a row to the `build-binaries` matrix, add a
matching `on_macos`/`on_linux` block + placeholder in
`packaging/homebrew/ukrop.rb`, and extend the loop and env exports in
`bump-homebrew`.

### Debian/Ubuntu

```sh
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/ukrop_0.1.0_*.deb
```

The deb package includes both `ukrop` and `u` binaries.
