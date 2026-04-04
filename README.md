# ukrop - Quick Directory Jumping & Command Execution

A fast Rust CLI tool with interactive fuzzy TUI for jumping to recent/favorite directories and re-running commands.
Tracks usage via shell hooks with a frecency-scored SQLite database.

## Features

- **Three-panel TUI** — cd, run, and ssh panels shown simultaneously with a shared fuzzy search bar
- **Two-tier search** powered by nucleo — substring matches first, fuzzy fallback, with matched characters highlighted
- **Frecency scoring** with exponential decay — recent and frequent entries rank higher
- **Shell integration** for zsh, bash, fish, and PowerShell with automatic directory and command tracking
- **Ctrl+R replacement** — replaces default reverse history search with the ukrop TUI
- **SSH host picker** — fuzzy search SSH hosts from `~/.ssh/config` and shell history
- **CWD filter** — filter commands to show only those run in the current directory
- **Favorites** — pin frequently used directories and commands to the top
- **Import from shell history** — bootstrap your database from existing bash, zsh, fish, or PowerShell history
- **Configuration file** — optional `~/.config/ukrop/config.toml` for ignore patterns, scoring weights, cleanup
  settings, themes, and layout
- **In-TUI config editor** — press F2 or run `ukrop config` to edit settings with live preview
- **Non-interactive mode** — `ukrop cd <query>` prints best match when stdout is not a TTY (for scripts)
- **Auto-cleanup** — stale missing directories are automatically pruned

```text
┌─ ukrop ──────────────────────────────────────────────────┐
│ > search query_                                          │
├──────────────┬───────────────────────────────────────────┤
│  cd (3/12)   │  run (5/20)                               │
│ > * ~/myapp  │ > git push origin main                    │
│   ~/work     │   cargo build --release                   │
│   ~/old      │   npm install                             │
│              │   docker compose up                       │
├──────────────┤                                           │
│  ssh (2/4)   │                                           │
│   prod-srv   │                                           │
│   dev-box    │                                           │
├──────────────┴───────────────────────────────────────────┤
│  Path: ~/myapp  │  Visits: 42  │  Last: 2h ago  │ exists │
├──────────────────────────────────────────────────────────┤
│ F1 help  F2 config  Tab  Enter run  ^F fav  ^Del  Esc    │
└──────────────────────────────────────────────────────────┘
```

## Install

### Homebrew (macOS/Linux)

```sh
brew tap gupalo/tap https://github.com/gupalo/homebrew-tap
brew install gupalo/tap/ukrop
```

### From source

```sh
cargo install --path .    # installs both `ukrop` and `u` binaries
```

### Debian/Ubuntu

```sh
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/ukrop_0.1.0_*.deb
```

## Quick Start

Run the interactive setup wizard:

```sh
ukrop setup
```

This imports your shell history, adds shell integration to your rc file, and enables the `u` shortcut. Each step asks
for confirmation.

Then restart your shell and use:

```sh
u               # jump to directory
u run           # run a command from history
u ssh           # connect to SSH host
u search docker # search commands with pre-filled query
u cd projects   # jump to directory with pre-filled query
u --help        # show help
```

Press **Ctrl+R** in your shell to open the TUI with the run panel active.

### Export / Import / Demo

```sh
ukrop export --file backup.jsonl   # back up entire database
ukrop import --file backup.jsonl   # restore from backup (replaces current data)
ukrop demo                         # generate demo data for screencasts
```

For manual setup and detailed usage, see [doc/usage.md](doc/usage.md).

## TUI Keys

| Key          | Action                               |
|--------------|--------------------------------------|
| Type         | Filter all panels (fuzzy search)     |
| Up / Down    | Move in active panel                 |
| Page Up/Down | Jump one page                        |
| Left / Right | Move cursor in search bar            |
| Home / End   | Cursor to start/end of search        |
| Enter        | Select entry                         |
| Shift+Enter  | Paste to terminal for editing        |
| Tab          | Switch active panel (cd → run → ssh) |
| Ctrl+F       | Toggle favorite                      |
| Ctrl+Del     | Delete entry (with confirmation)     |
| Ctrl+W       | Delete word backward                 |
| Ctrl+U       | Clear search input                   |
| Ctrl+Y       | Copy to clipboard                    |
| F1           | Show help                            |
| F2           | Open config editor                   |
| Esc / Ctrl+C | Cancel                               |

## License

MIT

## Help Ukraine

If you find this project useful, please consider supporting Ukraine:
🇺🇦 [Donate](https://commission.europa.eu/topics/eu-solidarity-ukraine/donate_en)
