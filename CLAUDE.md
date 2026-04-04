# CLAUDE.md

## Build & Run

```sh
cargo build              # debug build (produces both `ukrop` and `u` binaries)
cargo build --release    # release build
cargo test               # run all tests
./target/debug/ukrop --help
./target/debug/u --help
```

Rust toolchain is installed via rustup. Source the env if cargo is not in PATH:

```sh
source "$HOME/.cargo/env"
```

## Project Structure

- `src/main.rs` — Entry point (thin wrapper calling `ukrop::run()`)
- `src/bin/u.rs` — `u` shortcut binary (same entry point, clap shows binary name in help)
- `src/lib.rs` — Library: public module re-exports and `run()` with all subcommand dispatch
- `src/cli.rs` — Clap derive CLI definition
- `src/util.rs` — XDG paths, path helpers, `UKROP_DB_PATH` env override
- `src/frecency.rs` — Frecency scoring (exponential decay, 1-week half-life)
- `src/db/` — SQLite layer (model.rs, store.rs, migrate.rs)
- `src/demo.rs` — Demo data generation for screencasts
- `src/config.rs` — TOML config file (`~/.config/ukrop/config.toml`), ignore patterns, scoring weights, theme/layout config, save support
- `src/history/` — Shell history parsers (bash.rs, zsh.rs, fish.rs, powershell.rs)
- `src/shell/` — Shell init script templates (bash.rs, zsh.rs, fish.rs, powershell.rs)
- `src/tui/` — Terminal UI: 3-panel layout (configurable ratios, default cd 25%×75%, run 75%×100%, ssh 25%×25%) with shared search bar and cursor-enabled input (app.rs state machine, ui.rs rendering, input.rs keys, fuzzy.rs matcher, tty_reader.rs input parsing, theme.rs color theming, config_dialog.rs in-TUI config editor)
- `tests/` — Integration tests (cli, frecency, history parsing)
- `tests/fixtures/` — Sample history files for tests
- `packaging/` — Homebrew formula and deb postinst

## Rules

- Bump the version in `Cargo.toml` on each change (0.1.0 → 0.2.0 → 0.3.0 → …). Use minor version increments.
- Always update documentation (README.md, CLAUDE.md, doc/) when changing features, architecture, or behavior.
- `doc/usage.md` — detailed usage, shell integration, database schema, how it works
- `doc/dev.md` — development workflow, project structure, packaging
- `doc/search.md` — search modes (fuzzy vs substring) and ranking formula

## Key Architecture

- TUI renders to `/dev/tty`, not stdout. Stdout is captured by shell wrapper for cd/eval.
- SQLite DB at `~/Library/Application Support/ukrop/ukrop.db` (macOS) or `~/.local/share/ukrop/ukrop.db` (Linux) with WAL mode. Override via `UKROP_DB_PATH` env var.
- Shell integration: `eval "$(ukrop init zsh)"` installs a precmd hook and `ukrop` wrapper function. Shell hooks capture command duration (via `$SECONDS` in bash/zsh, `$CMD_DURATION` in fish, `Get-History` Duration in PowerShell).
- Optional config at `~/.config/ukrop/config.toml` — ignore patterns, scoring weights, cleanup settings, theme presets (Default/Light/Nord/Solarized/Monochrome), layout ratios. Override via `UKROP_CONFIG_PATH`.
- In-TUI config editor accessible via F2 key or `ukrop config` subcommand. Live preview of theme/layout changes; Esc cancels, F2/Ctrl+S saves.
- Non-interactive mode: `ukrop cd <query>` prints best match when stdout is not a TTY.
- Auto-cleanup removes stale missing directories (configurable, default 90 days).

## Tests

72 tests total. Run with `cargo test`. No special setup needed — integration tests use tempfile for isolated DB instances.

## Packaging

- `cargo deb` — builds .deb package with both `ukrop` and `u` binaries (requires `cargo install cargo-deb`)
- `packaging/homebrew/ukrop.rb` — Homebrew formula template
