# ukrop - Quick Directory Jumping & Command Execution

A fast Rust CLI tool with interactive fuzzy TUI for jumping to recent/favorite directories and re-running commands.
Tracks usage via shell hooks with a frecency-scored SQLite database.

## Features

- **One unified list** — directories, commands, and SSH hosts ranked together in a single list behind one fuzzy
  search bar, tagged with `/` `$` `@` sigils; press Tab to cycle a type filter (All → cd → run → ssh) without
  leaving the picker
- **Locality-aware ranking** — the current directory boosts commands run here and directories/hosts you've jumped to
  from here, on top of match quality, recency, and frecency (see [doc/search.md](doc/search.md))
- **Two-tier search** powered by nucleo — substring matches first, fuzzy fallback, with matched characters highlighted
- **Frecency scoring** with exponential decay — recent and frequent entries rank higher
- **Shell integration** for zsh, bash, fish, and PowerShell with automatic directory, command, and transition tracking
- **Ctrl+R replacement** — replaces default reverse history search with the ukrop TUI
- **SSH host picker** — fuzzy search SSH hosts from `~/.ssh/config` and shell history
- **CWD filter** — narrow the list to rows tied to the current directory (commands run here, plus directories/hosts
  reached from here)
- **Favorites** — pin frequently used directories and commands to the top
- **Import from shell history** — bootstrap your database (including locality data) from existing bash, zsh, fish, or
  PowerShell history
- **Configuration file** — optional `~/.config/ukrop/config.toml` for ignore patterns, scoring weights, cleanup
  settings, and themes
- **In-TUI command editor** — press F2 to edit the selected command before executing
- **In-TUI config editor** — press F9 or run `ukrop config` to edit settings with live preview
- **Non-interactive mode** — `ukrop cd <query>` prints best match when stdout is not a TTY (for scripts)
- **Auto-cleanup** — stale missing directories are automatically pruned

```text
┌ ukrop (5/62)  filter: All ─────────────────────────────────┐
│ > car_                                                      │
├──────────────────────────────────────────────────────────────┤
│ > $ cargo build                                       1.4s │
│   $ cargo test --release                              8.2s │
│   / ~/www/gupalo/ukrop/target                              │
│   @ carbon-prod                          root@10.0.0.4     │
│   / ~/old/carcass                                        ✗ │
├──────────────────────────────────────────────────────────────┤
│  Path: ~/www/gupalo/ukrop/target  │  Last: 2h ago  │ exists   │
├──────────────────────────────────────────────────────────────┤
│ F1 help  F2 edit  F9 config  Tab filter  Enter  ^F fav  Esc   │
└──────────────────────────────────────────────────────────────┘
```

## Install

### Homebrew (macOS/Linux)

```sh
brew tap ukroporg/tap https://github.com/ukroporg/homebrew-tap
brew install ukroporg/tap/ukrop
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
u               # open the unified list (All types)
u run           # unified list, run type filter preselected
u ssh           # unified list, ssh type filter preselected
u search docker # unified list, "docker" pre-filled, no type filter
u cd projects   # unified list, cd type filter preselected, "projects" pre-filled
u --help        # show help
```

Press **Ctrl+R** in your shell to open the TUI on the unfiltered `All` list — commands, directories and SSH hosts compete together. Press Tab to narrow it to a single type.

### Export / Import / Demo

```sh
ukrop export --file backup.jsonl   # back up entire database
ukrop import --file backup.jsonl   # restore from backup (replaces current data)
ukrop demo                         # generate demo data for screencasts
```

For manual setup and detailed usage, see [doc/usage.md](doc/usage.md).

## TUI Keys

| Key          | Action                                        |
|--------------|-------------------------------------------------|
| Type         | Filter the list (fuzzy search, all types)        |
| Up / Down    | Move selection                                    |
| Page Up/Down | Jump one page                                     |
| Left / Right | Move cursor in search bar                         |
| Home / End   | Cursor to start/end of search                     |
| Enter        | Select entry                                      |
| Shift+Enter / F5 | Paste to terminal for editing                |
| Tab / Shift+Tab | Cycle type filter (All → cd → run → ssh)       |
| Ctrl+W       | Toggle cwd filter (rows tied to this directory)   |
| Ctrl+F       | Toggle favorite                                   |
| F8 / Ctrl+Del | Delete entry (with confirmation)                 |
| Ctrl+U       | Clear search input                                |
| Ctrl+Y       | Copy to clipboard                                 |
| F1           | Show help                                         |
| F2           | Edit selected command                             |
| F9           | Open config editor                                |
| Esc / Ctrl+C | Cancel                                            |

> **Note:** `Shift+Enter` and `Ctrl+Del` require a modern terminal with extended key sequence support (kitty, WezTerm, foot, Ghostty). Alternatives: `F5` for paste-to-terminal, `F8` for entry deletion, `F2` for command editing. See [doc/usage.md](doc/usage.md#terminal-compatibility) for details.

## License

MIT

## Help Ukraine

If you find this project useful, please consider supporting Ukraine:
🇺🇦 [Donate](https://commission.europa.eu/topics/eu-solidarity-ukraine/donate_en)
