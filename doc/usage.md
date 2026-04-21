# Usage

## Setup

### Quick setup (recommended)

```sh
ukrop setup
```

This will:

1. **Import shell history** — commands, directories, and SSH hosts from your shell history and `~/.ssh/config`
2. **Add shell integration** — appends `eval "$(ukrop init zsh)"` (or bash/fish equivalent) to your rc file
3. **Enable `u` shortcut** — the shell integration includes `alias u=ukrop`

Each step asks for confirmation, so you can skip anything. On first run with an empty database, ukrop will suggest
running setup automatically.

To re-run setup (e.g. after declining on first run):

```sh
ukrop setup --force
```

### Manual setup

Add to your shell config:

```sh
# ~/.zshrc
eval "$(ukrop init zsh)"

# ~/.bashrc
eval "$(ukrop init bash)"

# ~/.config/fish/config.fish
ukrop init fish | source
```

```powershell
# PowerShell profile (run `echo $PROFILE` to find it)
Invoke-Expression (& { (ukrop init powershell | Out-String) })
```

Restart your shell or `source` the config file.

The `u` shortcut is included automatically in the init script (as a shell alias). A standalone `u` binary is also
installed, supporting `u --help` and `u --version` even without shell integration:

```sh
u              # jump to directory (active panel: cd)
u run          # run a command (active panel: run)
u ssh          # connect to SSH host (active panel: ssh)
u add ~/projects/myapp
u --help       # show help
u --version    # show version
```

### What shell integration installs

- A **precmd/prompt hook** that records the current directory on every prompt
- A **preexec hook** that captures each command and start time, and the precmd hook records it along with exit code,
  working directory, and execution duration via `ukrop hook-cmd`
- A **shell wrapper function** `ukrop` that handles `cd`, `run`, and `ssh` output
- A **Ctrl+R binding** that opens the TUI with run panel active; supports all result types (cd/run/ssh)
- An **alias `u=ukrop`** for quick access

## Commands

### Jump to a directory

```sh
ukrop          # opens TUI with cd panel active
ukrop cd       # same as above
```

### Run a command from history

```sh
ukrop run      # opens TUI with run panel active
```

Or press **Ctrl+R** in your shell.

### Search with pre-filled query

```sh
ukrop search docker    # opens TUI with run panel active, "docker" in search box
u search git push      # same via shortcut
```

All subcommands (`cd`, `run`, `ssh`) also accept an optional query:

```sh
u cd projects          # opens cd panel with "projects" pre-filled
u run make             # opens run panel with "make" pre-filled
u ssh prod             # opens ssh panel with "prod" pre-filled
```

### SSH to a host

```sh
ukrop ssh      # opens TUI with ssh panel active
```

Imports hosts from `~/.ssh/config` and shell history (ssh commands you've run).

### Import shell history

Populate the database from your existing shell history (also done by `ukrop setup`):

```sh
ukrop import        # auto-detect shell type
ukrop import zsh    # explicit
ukrop import bash
ukrop import fish
ukrop import powershell
```

### Reset database and reimport

To start fresh, delete the database and reimport:

```sh
rm "$HOME/Library/Application Support/ukrop/ukrop.db"   # macOS
rm ~/.local/share/ukrop/ukrop.db                        # Linux

ukrop import
```

### Manage entries

```sh
ukrop add ~/projects/myapp    # add favorite directory (validates path exists)
ukrop forget ~/old/path       # remove from database
ukrop list                    # show tracked directories with scores
ukrop list --commands         # show tracked commands with scores
ukrop list --ssh              # show tracked SSH hosts with scores
ukrop list --json             # JSON output for scripting
```

### Export / import database

Back up and restore the entire database (all directories, commands, SSH hosts with exact scores and timestamps):

```sh
ukrop export --file backup.jsonl     # export to JSONL file
ukrop export                         # export to stdout (pipe-friendly)
ukrop import --file backup.jsonl     # restore from JSONL (replaces current data)
```

JSONL format — one JSON object per line with a `type` field (`directory`, `command`, `ssh_host`).

Typical workflow for recording a demo video:

```sh
ukrop export --file ~/my-data.jsonl  # save personal data
ukrop demo                           # generate demo data
# ... record video ...
ukrop import --file ~/my-data.jsonl  # restore personal data
```

### Generate demo data

```sh
ukrop demo      # replaces database with realistic sample data
```

Generates ~25 directories, ~40 commands, and ~10 SSH hosts with varied scores, timestamps, and favorites — useful for screencasts and testing.

### Shell integration output

```sh
ukrop init zsh          # print zsh integration script
ukrop init bash         # print bash integration script
ukrop init fish         # print fish integration script
ukrop init powershell   # print PowerShell integration script
```

## TUI Layout

All three panels are shown simultaneously with a shared search bar at the top:

- **Left column (25% width)**: cd panel (top 75%) and ssh panel (bottom 25%)
- **Right column (75% width)**: run panel (full height)
- **Active panel** is highlighted with a green border; inactive panels are dimmed
- **Search** filters all three panels simultaneously with matched characters highlighted (cyan + underline)

### Active panel defaults

| Invocation         | Active panel |
|--------------------|--------------|
| `u` / `u cd`       | cd           |
| `u run`            | run          |
| `u ssh`            | ssh          |
| `u search <query>` | run          |
| Ctrl+R             | run          |

### Keyboard Shortcuts

Press **F1** inside the TUI to show the help overlay with all shortcuts.

| Key          | Action                        |
|--------------|-------------------------------|
| Enter        | Select and run immediately    |
| Shift+Enter  | Paste to terminal for editing |
| Ctrl+Y       | Copy to clipboard             |
| Esc          | Quit                          |
| Tab          | Next panel                    |
| Shift+Tab    | Previous panel                |
| Up / Down    | Navigate list                 |
| PgUp / PgDn  | Scroll page                   |
| Left / Right | Move cursor in search bar     |
| Home / End   | Cursor to start/end of search |
| Ctrl+A / E   | Cursor to start/end of search |
| Ctrl+B       | Move cursor left              |
| Ctrl+W       | Delete word backward          |
| Ctrl+P / N   | Navigate up / down            |
| Ctrl+F       | Toggle favorite               |
| Ctrl+Del     | Delete entry                  |
| Delete       | Delete char at cursor         |
| Ctrl+U       | Clear search                  |
| Ctrl+C / D   | Quit                          |
| F1           | Show help overlay             |
| F2           | Edit selected command         |
| F9           | Open config editor            |

## Configuration

Ukrop supports an optional TOML configuration file at `~/.config/ukrop/config.toml`. Override the path with the
`UKROP_CONFIG_PATH` environment variable.

If the file doesn't exist, defaults are used. All settings are optional.

```toml
# Commands matching these patterns are never recorded
ignore_patterns = [
    " ", # commands starting with a space
    "ls", # exact match
    "cd *", # prefix + wildcard (any cd command)
    "exit",
]

[scoring]
frecency_weight = 100.0   # scale factor for frecency bonus (default: 100.0)
substring_bonus = 8000    # bonus for substring matches (default: 8000)
prefix_bonus = 10000      # bonus for prefix matches (default: 10000)

[cleanup]
stale_days = 90            # auto-remove missing directories older than this (default: 90)

[theme]
preset = "default"         # color theme: default, light, nord, solarized, monochrome
selection_bold = true      # bold styling for selected items (default: true)
match_underline = true     # underline matched characters (default: true)
favorite_italic = false    # italic styling for favorites (default: false)

[layout]
left_panel_pct = 25        # left column width percentage (5-50, default: 25)
cd_panel_pct = 75          # cd panel height percentage within left column (10-90, default: 75)
```

### In-TUI config editor

Press **F9** inside the TUI or run `ukrop config` to open the config editor as a modal overlay. Changes are previewed
live (theme and layout changes update the background panels in real-time). Press **F9** or **Ctrl+S** to save, **Esc**
to cancel and revert.

### Theme presets

| Preset        | Description                                                  |
|---------------|--------------------------------------------------------------|
| `default`     | Yellow borders, cyan highlights, green/white/gray age colors |
| `light`       | Blue borders, magenta highlights — for light terminals       |
| `nord`        | Nord palette with blue/cyan tones                            |
| `solarized`   | Solarized dark color scheme                                  |
| `monochrome`  | White/gray only, no colors                                   |
| `dracula`     | Dracula — purple borders, green highlights, pink headers     |
| `gruvbox`     | Gruvbox dark — warm yellow/aqua/green tones                  |
| `catppuccin`  | Catppuccin Mocha — mauve/green pastel palette                |
| `tokyo_night` | Tokyo Night — cool blue borders, yellow highlights           |
| `kanagawa`    | Kanagawa — crystal blue, autumn yellow, sakura pink          |
| `everforest`  | Everforest — soft green/yellow natural tones                 |
| `rose`        | Rose Pine — love pink borders, iris purple highlights        |

### Ignore patterns

- `"ls"` — exact match only
- `"cd *"` — matches any command starting with `cd ` (prefix wildcard)
- `" "` — matches commands starting with a space (like bash HISTCONTROL=ignorespace)

### Auto-cleanup

When opening the cd panel, directories that no longer exist on disk and haven't been visited in `stale_days` days (
default: 90) with a low score are automatically removed.

### Non-interactive mode

When stdout is not a TTY (e.g., in scripts), `ukrop cd <query>` prints the best match directly without opening the TUI:

```sh
cd "$(ukrop cd projects)"   # jump to best-matching directory
```

## How It Works

1. **Directory tracking**: The shell hook calls `ukrop hook -- "$PWD"` on every prompt, recording the visit with a
   frecency score
2. **Command tracking**: The preexec hook captures each command, and the precmd hook records it along with exit code,
   working directory, and execution duration via `ukrop hook-cmd`
3. **Frecency scoring**: Scores decay exponentially with a 1-week half-life — a directory visited 100 times a month ago
   scores lower than one visited 10 times today
4. **Aging**: When total scores exceed 10,000, all scores are scaled down and near-zero entries are pruned
5. **TUI rendering**: The picker renders to `/dev/tty` (not stdout), so the shell wrapper can capture stdout to get the
   selected path/command
6. **Atomic writes**: Database operations use SQLite transactions to prevent corruption from concurrent access

## Database

SQLite database (WAL mode for concurrent reads) at:

- **macOS**: `~/Library/Application Support/ukrop/ukrop.db`
- **Linux**: `~/.local/share/ukrop/ukrop.db`

Override location with the `UKROP_DB_PATH` environment variable:

```sh
UKROP_DB_PATH=/tmp/test.db ukrop list
```

### Schema

**directories** — tracked directory visits:

- `path`, `score`, `visit_count`, `last_visit`, `is_favorite`

**commands** — tracked command executions:

- `command`, `score`, `use_count`, `last_used`, `is_favorite`, `source`, `exit_code`, `cwd`, `duration_ms`

**ssh_hosts** — tracked SSH connections:

- `host`, `hostname`, `port`, `user`, `score`, `use_count`, `last_used`, `is_favorite`, `source`
