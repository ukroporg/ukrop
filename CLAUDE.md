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
- `src/db/` — SQLite layer (model.rs, store.rs, migrate.rs, transitions.rs)
- `src/demo.rs` — Demo data generation for screencasts
- `src/config.rs` — TOML config file (`~/.config/ukrop/config.toml`), ignore patterns, scoring weights, theme/layout config, save support
- `src/history/` — Shell history parsers (bash.rs, zsh.rs, fish.rs, powershell.rs)
- `src/shell/` — Shell init script templates (bash.rs, zsh.rs, fish.rs, powershell.rs)
- `src/tui/` — Terminal UI: a single locality-ranked list mixing cd/run/ssh rows (`/`, `$`, `@` sigils) behind one shared search bar, with a cycling type filter (All/cd/run/ssh) and cursor-enabled input (app.rs state machine, ranking.rs scoring + diversity re-rank, ui.rs rendering, input.rs keys, fuzzy.rs matcher, tty_reader.rs input parsing, theme.rs color theming, config_dialog.rs in-TUI config editor, edit_dialog.rs in-TUI command editor)
- `tests/` — Integration tests (cli, frecency, history parsing)
- `tests/fixtures/` — Sample history files for tests
- `packaging/` — Homebrew formula and deb postinst
- `doc/website/` — Static multipage site for ukrop.org (pure HTML/CSS/JS, no build step). Deployable via GitHub Pages with the bundled `CNAME`. Gitignored (`/doc/website` in `.gitignore`) — the directory may exist on a developer's disk but is deliberately untracked in this repository; it's maintained and deployed outside version control.

## Rules

- Bump the version in `Cargo.toml` on each change (0.1.0 → 0.2.0 → 0.3.0 → …). Use minor version increments.
- Always update documentation (README.md, CLAUDE.md, doc/) when changing features, architecture, or behavior.
- `doc/usage.md` — detailed usage, shell integration, database schema, how it works
- `doc/dev.md` — development workflow, project structure, packaging
- `doc/search.md` — search modes (fuzzy vs substring) and ranking formula

## Key Architecture

- TUI renders to `/dev/tty`, not stdout. Stdout is captured by shell wrapper for cd/eval.
- SQLite DB at `~/Library/Application Support/ukrop/ukrop.db` (macOS) or `~/.local/share/ukrop/ukrop.db` (Linux) with WAL mode. Override via `UKROP_DB_PATH` env var.
- Shell integration: `eval "$(ukrop init zsh)"` installs a precmd hook, a `ukrop` wrapper function, and a Ctrl+R binding that calls `ukrop search` (unfiltered `All` list — not `ukrop run`). Shell hooks capture command duration (via `$SECONDS` in bash/zsh, `$CMD_DURATION` in fish, `Get-History` Duration in PowerShell).
- Optional config at `~/.config/ukrop/config.toml` — ignore patterns, scoring weights, cleanup settings, theme presets (12 built-in), `[layout]` (deprecated as of 0.20.0: parsed for back-compat, ignored, not shown in the config editor). Override via `UKROP_CONFIG_PATH`.
- In-TUI command editor accessible via F2 key — opens a 10-line dialog pre-filled with the selected command for editing before execution. Enter inserts newline, F5 executes, Esc cancels. Supports Up/Down arrow navigation between lines.
- In-TUI config editor accessible via F9 key or `ukrop config` subcommand. Live preview of theme changes; Esc cancels, F9/Ctrl+S saves.
- Ranking: one formula scores every row (cd/run/ssh) — match quality, frecency, mutually-exclusive recency tiers, locality (`cwd_bonus` for run, decayed `transitions`-table score for cd/ssh), brevity, favorites, plus a position-dependent type-diversity bonus applied by a three-way merge so the `All` view doesn't get dominated by one type. Fuzzy-tier rows additionally get a contiguity bonus (`sum(run_len^2) - match_len`, weighted and capped) so `seo`+`2` outranks a scattered `s`…`e`…`o`…`2`; substring rows skip it, being a single run by definition. See `doc/search.md`.
- `transitions` table (schema v5) records directory→directory and directory→host jumps, read once at TUI startup. Three capture paths: picker-initiated picks (synchronous, exact), manual `cd` via the `hook --shell-id` prompt hook, manual `ssh` via `hook-cmd` detecting the `ssh ` prefix. `ukrop import` backfills it from shell history.
- Non-interactive mode: `ukrop cd <query>` prints best match when stdout is not a TTY.
- Auto-cleanup removes stale missing directories, transition rows, and per-shell PWD bookkeeping (configurable, default 90 days).

## Tests

192 tests total. Run with `cargo test`. No special setup needed — integration tests use tempfile for isolated DB instances.

## Packaging

- `cargo deb` — builds .deb package with both `ukrop` and `u` binaries (requires `cargo install cargo-deb`)
- `packaging/homebrew/ukrop.rb` — Homebrew formula template
