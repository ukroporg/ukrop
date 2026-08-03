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
u              # unified list, no type filter (All)
u cd           # unified list, cd type filter preselected
u run          # unified list, run type filter preselected
u ssh          # unified list, ssh type filter preselected
u add ~/projects/myapp
u --help       # show help
u --version    # show version
```

### What shell integration installs

- A **precmd/prompt hook** that records the current directory on every prompt, tagged with a per-shell id
  (`ukrop hook --shell-id "$$" -- "$PWD"`) so it can also detect manual `cd`s and record a directory-to-directory
  transition
- A **preexec hook** that captures each command and start time, and the precmd hook records it along with exit code,
  working directory, and execution duration via `ukrop hook-cmd`. Commands starting with `ssh ` are additionally
  matched against known hosts and recorded as a transition from the command's cwd
- A **shell wrapper function** `ukrop` that handles `cd`, `run`, and `ssh` output
- A **Ctrl+R binding** that opens the unified list unfiltered (the `All` type filter), so commands, directories and
  SSH hosts all compete for the top of the list; press Tab to narrow it to a single type
- An **alias `u=ukrop`** for quick access

See [Shell integration flags](#shell-integration-flags) below for what `--shell-id` and `--cwd` are for and what
happens if your init script predates them.

## Commands

### Jump to a directory

```sh
ukrop          # opens the unified list with no type filter (All)
ukrop cd       # opens the unified list with the cd type filter preselected
```

### Run a command from history

```sh
ukrop run      # opens the unified list with the run type filter preselected
```

Or press **Ctrl+R** in your shell.

### Search with pre-filled query

```sh
ukrop search docker    # opens the unified list (All types), "docker" in search box
u search git push      # same via shortcut
```

All subcommands (`cd`, `run`, `ssh`) also accept an optional query:

```sh
u cd projects          # cd filter preselected, "projects" pre-filled
u run make             # run filter preselected, "make" pre-filled
u ssh prod             # ssh filter preselected, "prod" pre-filled
```

Muscle memory from the old per-panel commands is preserved — `u cd`, `u run`, and `u ssh` still narrow to one type — but
now you can press **Tab** to widen to `All` without leaving the picker.

### SSH to a host

```sh
ukrop ssh      # opens the unified list with the ssh type filter preselected
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

In addition to directories, commands, and SSH hosts, `ukrop import` backfills the `transitions` table (see
[Schema](#schema)) by replaying history chronologically and maintaining a `current_dir` state, the same way it already
guesses each command's `cwd`: each resolvable `cd` (absolute and `~/...` paths only — relative `cd foo` is skipped
since it can't be resolved without knowing the prior directory) becomes a `cd` transition from `current_dir` at that
point, and each `ssh` invocation becomes an `ssh` transition from `current_dir`. This means locality-aware ranking has
real data on day one instead of warming up over a week of live usage.

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

## Unified list

Directories (`cd`), commands (`run`), and SSH hosts (`ssh`) all live in a single ranked list behind one search bar —
there are no separate panels to switch between. Each row is tagged with a one-character sigil so you can tell what it
is at a glance:

| Type  | Sigil | Example row              |
|-------|-------|----------------------------|
| `cd`  | `/`   | `/ ~/www/gupalo/ukrop`     |
| `run` | `$`   | `$ cargo build`            |
| `ssh` | `@`   | `@ prod  root@10.0.0.4`    |

`>` marks the selection cursor and is never used as a type sigil. The dim right-hand column stays type-specific —
duration for `run`, `user@host:port` for `ssh`, a `✗` missing-directory marker for `cd` — carrying over what each old
panel showed. Search filters and highlights (cyan + underline) across all visible rows regardless of type. See
[doc/search.md](search.md) for how rows are ranked, including how type diversity is preserved near the top of the
`All` view.

### Type filter defaults by entry point

| Invocation         | Type filter preselected |
|--------------------|--------------------------|
| `u` / `ukrop`      | All                      |
| `u cd`             | cd                       |
| `u run`            | run                      |
| `u ssh`            | ssh                      |
| `u search <query>` | All                      |
| Ctrl+R             | All                      |

Press **Tab** at any point to widen or narrow the filter without restarting the picker.

### Keyboard Shortcuts

Press **F1** inside the TUI to show the help overlay with all shortcuts.

| Key          | Action                                                  |
|--------------|-----------------------------------------------------------|
| Enter        | Select and run immediately                                 |
| Shift+Enter / F5 | Paste to terminal for editing                          |
| Ctrl+Y       | Copy to clipboard                                           |
| Esc          | Quit                                                        |
| Tab          | Cycle type filter forward: All → cd → run → ssh → All      |
| Shift+Tab    | Cycle type filter backward                                  |
| Up / Down    | Navigate list                                               |
| PgUp / PgDn  | Scroll page                                                 |
| Left / Right | Move cursor in search bar                                   |
| Home / End   | Cursor to start/end of search                               |
| Ctrl+A / E   | Cursor to start/end of search                               |
| Ctrl+B       | Move cursor left                                             |
| Ctrl+W       | Toggle cwd filter — narrow to rows tied to the current directory (see below) |
| Ctrl+P / N   | Navigate up / down                                           |
| Ctrl+F       | Toggle favorite                                              |
| F8 / Ctrl+Del | Delete entry                                                |
| Delete       | Delete char at cursor (or delete the selected entry, if the cursor is at the end of the search text — trivially true when the search bar is empty, but also true any time the cursor has been moved to the end of a non-empty query) |
| Ctrl+U       | Clear search                                                 |
| Ctrl+C / D   | Quit                                                         |
| F1           | Show help overlay                                            |
| F2           | Edit selected command                                        |
| F9           | Open config editor                                           |

### Cwd filter (Ctrl+W)

`Ctrl+W` toggles a filter that narrows the list to rows tied to the current directory — generalized from the old
"commands run in this directory" filter to all three types: `run` rows whose recorded `cwd` matches, plus `cd` and
`ssh` rows reached from here at least once (per the `transitions` table — see [Schema](#schema)). The active filter is
shown in the list's title bar (`filter: <type> [cwd]`) alongside the type filter.

### Terminal Compatibility

Some shortcuts depend on the terminal emulator's ability to send extended key sequences:

- **Shift+Enter** requires the [kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/). Terminals that support it include kitty, WezTerm, foot, and Ghostty. Standard terminals (macOS Terminal.app, older GNOME Terminal) send the same byte for `Enter` and `Shift+Enter`, so the shortcut has no effect there. Use **F5** as a universal alternative for paste-to-terminal, or **F2** to open the in-TUI command editor.
- **Ctrl+Delete** requires the terminal to send a CSI modifier sequence (`ESC[3;5~`). Some terminals (notably macOS Terminal.app) send a plain `Delete` even with Ctrl held. Use **F8** as a universal alternative, or press `Delete` when the search bar is empty.

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
# ... plus fuzzy_penalty, contiguity_weight, contiguity_cap, favorite_bonus, recency_24h_bonus,
# recency_7d_bonus, cwd_bonus, transition_weight, transition_cap, brevity_bonus_max,
# and [scoring.type_bonus].schedule —
# see doc/search.md#configuration for the complete list with every default.

[cleanup]
stale_days = 90            # auto-remove missing directories older than this (default: 90)

# Ask for confirmation before deleting an entry (default: true)
confirm_delete = true

[theme]
preset = "default"         # color theme: default, light, nord, solarized, monochrome
selection_bold = true      # bold styling for selected items (default: true)
match_underline = true     # underline matched characters (default: true)
favorite_italic = false    # italic styling for favorites (default: false)

# Deprecated: the unified list has no panels, so these ratios are parsed for
# backward compatibility with old config files and then ignored. Not
# shown in the in-TUI config editor.
[layout]
left_panel_pct = 25
cd_panel_pct = 75
```

### In-TUI config editor

Press **F9** inside the TUI or run `ukrop config` to open the config editor as a modal overlay. Changes are previewed
live (theme changes update the result list colors in real-time). Press **F9** or **Ctrl+S** to save, **Esc**
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

When opening the picker, directories that no longer exist on disk and haven't been visited in `stale_days` days (
default: 90) with a low score are automatically removed. The same pass prunes `transitions` rows and per-shell
`last_pwd:<id>` bookkeeping keys older than the same window.

### Non-interactive mode

When stdout is not a TTY (e.g., in scripts), `ukrop cd <query>` prints the best match directly without opening the TUI:

```sh
cd "$(ukrop cd projects)"   # jump to best-matching directory
```

## How It Works

1. **Directory tracking**: The shell hook calls `ukrop hook --shell-id "$$" -- "$PWD"` on every prompt, recording the
   visit with a frecency score
2. **Command tracking**: The preexec hook captures each command, and the precmd hook records it along with exit code,
   working directory, and execution duration via `ukrop hook-cmd`
3. **Transition tracking**: A directory-to-directory or directory-to-host jump is recorded in the `transitions` table
   whenever it can be attributed to an origin directory — synchronously for picker-initiated jumps, from the prompt
   hook's per-shell PWD tracking for manually typed `cd`, and from `hook-cmd` detecting a manually typed `ssh`. See
   [Shell integration flags](#shell-integration-flags) and [doc/search.md](search.md#locality-cwd_bonus-vs-transition_bonus).
4. **Frecency scoring**: Scores (and transition scores) decay exponentially with a 1-week half-life — a directory
   visited 100 times a month ago scores lower than one visited 10 times today
5. **Aging**: When total scores exceed 10,000, all scores are scaled down and near-zero entries are pruned
6. **TUI rendering**: The picker renders to `/dev/tty` (not stdout), so the shell wrapper can capture stdout to get the
   selected path/command
7. **Atomic writes**: Database operations use SQLite transactions to prevent corruption from concurrent access

## Database

SQLite database (WAL mode for concurrent reads) at:

- **macOS**: `~/Library/Application Support/ukrop/ukrop.db`
- **Linux**: `~/.local/share/ukrop/ukrop.db`

Override location with the `UKROP_DB_PATH` environment variable:

```sh
UKROP_DB_PATH=/tmp/test.db ukrop list
```

### Schema

Migrations are version-gated and run automatically on open (`src/db/migrate.rs`); the database is currently at
schema version 5.

**directories** — tracked directory visits:

- `path`, `score`, `visit_count`, `last_visit`, `is_favorite`

**commands** — tracked command executions:

- `command`, `score`, `use_count`, `last_used`, `is_favorite`, `source`, `exit_code`, `cwd`, `duration_ms`

**ssh_hosts** — tracked SSH connections:

- `host`, `hostname`, `port`, `user`, `score`, `use_count`, `last_used`, `is_favorite`, `source`

**transitions** — directory-to-directory and directory-to-host jumps, added in migration v5. Drives the
locality-aware ranking described in [doc/search.md](search.md#locality-cwd_bonus-vs-transition_bonus):

- `from_cwd`, `kind` (`cd` or `ssh`), `target`, `score`, `count`, `last_time` — primary key `(from_cwd, kind, target)`

`score` decays with the same exponential, 1-week-half-life formula as the other three tables. Read once at TUI
startup into an in-memory map (one row per `(kind, target)` pair originating at the current directory) — no
per-keystroke database work.

**meta** — key/value store used for schema version bookkeeping and, since this feature, per-shell PWD tracking under
`last_pwd:<shell-id>` keys (see [Shell integration flags](#shell-integration-flags)).

### Shell integration flags

Two shell-hook flags support transition capture for manually typed `cd` and `ssh`, and both are optional with a
graceful fallback:

- **`ukrop hook --shell-id "$$" -- "$PWD"`** — `--shell-id` (the shell's PID) lets ukrop tell concurrent shells apart
  when tracking the last-seen directory per shell, so it can detect a manual `cd` and record a transition. Without
  it, directory visits are still recorded as before, but no transition is derived from this path (a single global
  "last pwd" would fabricate transitions between unrelated terminal tabs sitting in different directories).
- **`ukrop hook-ssh --host ... [--cwd "$PWD"]`** — `--cwd`, when present, records a transition from that directory to
  the resolved SSH host. The `ukrop` shell wrapper's own picker-selection branch deliberately does **not** pass
  `--cwd` here: when you pick an `ssh` row from the TUI, ukrop already records that exact transition synchronously
  before printing the result, so also passing `--cwd` from the shell would double-count it. Manually typed `ssh`
  commands are instead captured via the `hook-cmd` path (which already carries `--cwd`) recognizing the `ssh ` prefix.

Both flags were added in 0.20.0. A shell init script generated by an older version omits them and keeps working
exactly as before — directory and command tracking, and picker-initiated transitions (recorded internally by ukrop,
independent of the shell script) are unaffected — it simply won't derive transitions from manually typed `cd`/`ssh`.
Run `ukrop setup --force`, or re-source your rc file after `ukrop init <shell>`, to pick up the new flags.
