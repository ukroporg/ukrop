# Comparison with Alternatives

ukrop combines directory jumping, command history search, and SSH host picking in a single tool with a unified TUI. Most
alternatives focus on only one of these areas.

## Quick Comparison Table

| Feature                       | ukrop                 | zoxide         | autojump      | fzf             | Atuin             | McFly           | HSTR            | hiSHtory           | Television        |
|-------------------------------|-----------------------|----------------|---------------|-----------------|-------------------|-----------------|-----------------|--------------------|-------------------|
| **GitHub stars**              | —                     | 34.8k          | 16.9k         | 78.9k           | 28.8k             | 7.6k            | 4.5k            | 3.1k               | ~7k               |
| **Last updated**              | Mar 2026              | Mar 2026       | Feb 2025      | Mar 2026        | Mar 2026          | Feb 2026        | Feb 2026        | Mar 2026           | Mar 2026          |
| **Language**                  | Rust                  | Rust           | Python        | Go              | Rust              | Rust            | C/C++           | Go                 | Rust              |
| **Directory jumping**         | Yes                   | Yes            | Yes           | Alt+C           | -                | -              | -              | -                 | -                |
| **Command history search**    | Yes                   | -             | -            | Ctrl+R          | Yes               | Yes             | Yes             | Yes                | -                |
| **SSH host picker**           | Yes                   | -             | -            | -              | -                | -              | -              | -                 | -                |
| **Unified TUI**               | Single list           | No (CLI)       | No (CLI)      | Single list     | Single list       | Single list     | Single list     | Single list        | Single list       |
| **Frecency scoring**          | Yes                   | Yes            | Yes           | -              | -                | Neural net      | Ranking algo    | -                 | -                |
| **Fuzzy search**              | nucleo                | fzf (ext)      | -            | Built-in        | Built-in          | Optional        | Substring       | Substring          | Built-in (nucleo) |
| **Two-tier search**           | Yes (sub+fuzzy)       | -             | -            | -              | -                | -              | -              | -                 | -                |
| **Match highlighting**        | Yes (cyan+underline)  | -             | -            | Yes             | Yes               | -              | -              | -                 | Yes               |
| **Pre-filled query**          | Yes (`u cd foo`)      | Yes (`z foo`)  | Yes (`j foo`) | -              | -                | -              | -              | -                 | -                |
| **CWD-scoped commands**       | Ctrl+W toggle         | -            | -           | -              | Yes (filter)      | Yes (context)   | -              | Yes (column)       | -                |
| **Command duration**          | -                    | -             | -            | -              | Yes               | Yes             | -              | Yes                | -                |
| **Cross-terminal history**    | Yes (SQLite)          | -            | -           | -              | Yes               | Yes             | -              | Yes                | -                |
| **Stale entry cleanup**       | Auto (configurable)   | Auto (90 days) | -            | -              | -                | -              | -              | -                 | -                |
| **Privacy controls**          | Yes (ignore patterns) | -             | -            | -              | Yes (HIST_IGNORE) | -              | Yes (blacklist) | Yes (pause/resume) | -                |
| **AI integration**            | -                    | -             | -            | -              | Yes               | -              | -              | Yes (ChatGPT)      | -                |
| **Configuration file**        | Yes (config.toml)     | -             | -            | Env vars        | Yes (config.toml) | Env vars        | -              | Yes (config)       | Yes (config.toml) |
| **Favorites / bookmarks**     | Yes (Ctrl+F)          | -             | -            | -              | -                | -              | Yes             | -                 | -                |
| **Edit before execute**       | Yes (Shift+Enter)     | -             | -            | -              | -                | -              | -              | -                 | -                |
| **Copy to clipboard**         | Yes (Ctrl+Y)          | -             | -            | -              | -                | -              | -              | -                 | -                |
| **Setup wizard**              | Yes (`ukrop setup`)   | -             | -            | -              | Yes               | -              | -              | Yes                | -                |
| **Short alias**               | `u`                   | `z`            | `j`           | —               | —                 | —               | `hh`            | —                  | `tv`              |
| **Cloud sync**                | -                    | -             | -            | -              | Yes (E2E)         | -              | -              | Yes (E2E)          | -                |
| **Shell support**             | zsh, bash, fish       | 8+ shells      | bash, zsh     | bash, zsh, fish | bash, zsh, fish   | bash, zsh, fish | bash, zsh       | bash, zsh, fish    | bash, zsh, fish   |
| **Ctrl+R replacement**        | Yes                   | -             | -            | Yes             | Yes               | Yes             | Yes             | Yes                | -                |
| **Exit code tracking**        | Yes                   | -             | -            | -              | Yes               | Yes             | -              | Yes                | -                |
| **SQLite database**           | Yes (WAL)             | No (custom)    | No (text)     | -              | Yes               | Yes             | -              | Yes                | -                |
| **Import from shell history** | Yes                   | Yes (migrate)  | -            | -              | Yes               | -              | -              | Yes                | -                |
| **Import SSH config**         | Yes                   | -             | -            | -              | -                | -              | -              | -                 | -                |
| **In-TUI config editor**      | Yes (F9)              | -             | -            | -              | -                | -              | -              | -                 | -                |
| **Non-interactive mode**      | Yes (`u cd foo`)      | Yes (`z foo`)  | Yes (`j foo`) | -              | -                | -              | -              | -                 | -                |
| **Help overlay (F1)**         | Yes                   | -             | -            | -              | -                | -              | -              | -                 | -                |
| **License**                   | MIT                   | MIT            | GPL-3.0       | MIT             | MIT               | MIT             | Apache-2.0      | MIT                | MIT               |

## Detailed Comparisons

### vs zoxide

[zoxide](https://github.com/ajeetdsouza/zoxide) is the most popular directory jumper today. It is fast, well-maintained,
and supports 8+ shells.

**Where ukrop wins:**

- Built-in command history search and SSH host picker -- zoxide only handles directories
- Unified, locality-ranked list shows cd, run, and ssh results together
- Two-tier search: substring matches rank above fuzzy-only matches, with highlight
- Favorites system to pin frequently used entries
- CWD-scoped command filtering
- Edit before execute (Shift+Enter to edit a command before running)
- Copy to clipboard (Ctrl+Y)
- Exit code tracking for commands
- SSH config import alongside shell history import
- Interactive setup wizard (`ukrop setup`)
- Configuration file with ignore patterns, scoring weights, theme presets, and in-TUI editor (F9)
- Auto-cleanup of stale directories (configurable, default 90 days)
- Non-interactive mode (`u cd foo` prints best match without TUI)
- No external dependency for fuzzy search (zoxide's interactive mode requires fzf)

**Where zoxide wins:**

- Larger community and ecosystem (integrations with vim, emacs, ranger, tmux)
- Supports more shells (PowerShell, Nushell, Elvish, Xonsh, Tcsh, ksh)
- Can be used as a drop-in `cd` replacement (`z foo bar` without opening a TUI)
- Import from autojump, fasd, z, z.lua, zsh-z databases
- More mature -- battle-tested in production for years

**Bottom line:** If you only need directory jumping, zoxide is a solid choice. If you want directory jumping + command
history + SSH in one tool, ukrop covers all three.

### vs autojump

[autojump](https://github.com/wting/autojump) is one of the original directory jumpers.

**Where ukrop wins:**

- Written in Rust (fast) vs Python (slower startup, ~50ms vs ~5ms)
- Command history search and SSH host picker included
- Fuzzy search built-in
- Fish shell support
- Actively maintained

**Where autojump wins:**

- Long track record and wide adoption
- Simple and well-understood behavior

**Bottom line:** autojump is largely superseded by newer tools. ukrop and zoxide both offer better performance and more
features.

### vs fzf

[fzf](https://github.com/junegunn/fzf) is a general-purpose fuzzy finder that can be configured for directory jumping (
Alt+C) and history search (Ctrl+R).

**Where ukrop wins:**

- Purpose-built for the shell workflow (cd + run + ssh)
- Frecency scoring -- fzf has no built-in ranking by usage patterns
- Unified list ranks cd, run, and ssh results together
- Two-tier search with substring priority
- Match highlighting distinguishes substring vs fuzzy matches (cyan+underline for matched characters)
- Tracks directory visits, command metadata (exit codes, CWD), and SSH hosts automatically
- Favorites, CWD filtering, edit-before-execute, and clipboard copy
- Zero configuration after `ukrop setup`

**Where fzf wins:**

- General-purpose -- works with any list (files, processes, git branches, etc.)
- Massive ecosystem of integrations and plugins
- Extremely mature and widely adopted
- Can be composed with other tools via pipes

**Bottom line:** fzf is a Swiss army knife; ukrop is a purpose-built tool. fzf requires manual setup for each use case,
while ukrop provides an integrated experience out of the box.

### vs Atuin

[Atuin](https://atuin.sh/) replaces shell history with a SQLite database and offers optional encrypted cloud sync.

**Where ukrop wins:**

- Directory jumping and SSH host picker included (Atuin is command-history only)
- Unified list searches across all entry types at once
- Two-tier search: substring matches prioritized over fuzzy, with match highlighting
- Frecency scoring with exponential decay
- Favorites system, edit-before-execute, clipboard copy
- In-TUI config editor with live preview (F9)
- Auto-cleanup of stale directories
- Lighter footprint -- no sync daemon, no account needed

**Where Atuin wins:**

- End-to-end encrypted cloud sync across devices
- Command duration tracking
- AI-powered search
- Stats and analytics
- Session tracking
- More advanced filtering and query capabilities
- Multiline command support
- Nushell and Xonsh support
- Larger community

**Bottom line:** Atuin is the best choice if you need cross-device history sync. ukrop is better if you want a single
tool for directories, commands, and SSH without cloud dependencies.

### vs McFly

[McFly](https://github.com/cantino/mcfly) replaces Ctrl+R with a neural-network-powered search.

**Where ukrop wins:**

- Directory jumping and SSH host picker included
- Unified, locality-ranked list
- Frecency scoring is transparent and predictable
- Two-tier search with substring priority and match highlighting
- Favorites, CWD filtering, edit-before-execute, clipboard copy
- Import from shell history and SSH config

**Where McFly wins:**

- Neural network ranking considers context (current directory, recent commands, exit status)
- Command duration tracking
- Maintains the normal shell history file alongside its own database

**Bottom line:** McFly is clever at context-aware command ranking. ukrop offers broader scope (cd + run + ssh) with a
more predictable scoring model.

### vs HSTR

[HSTR](https://github.com/dvorka/hstr) is a lightweight C/C++ tool for shell history search with bookmarking.

**Where ukrop wins:**

- Directory jumping and SSH host picker
- Fuzzy search (HSTR uses substring/regex)
- Fish shell support
- Frecency scoring
- Unified, locality-ranked list

**Where HSTR wins:**

- Very lightweight and fast (C/C++)
- Regex search support
- History blacklisting
- Wide platform support (Linux, macOS, BSD, Cygwin)

**Bottom line:** HSTR is a minimal, fast history browser. ukrop offers a broader feature set at the cost of a slightly
larger binary.

### vs hiSHtory

[hiSHtory](https://github.com/ddworken/hishtory) focuses on queryable, context-rich shell history with encrypted sync.

**Where ukrop wins:**

- Directory jumping and SSH host picker
- Frecency scoring
- Favorites
- No account or backend required
- Unified, locality-ranked list

**Where hiSHtory wins:**

- End-to-end encrypted cross-device sync (self-hostable)
- Customizable columns (hostname, CWD, runtime, exit code, user)
- Command duration tracking
- Privacy controls: pause/resume history recording on the fly
- AI-powered command suggestions via ChatGPT

**Bottom line:** hiSHtory shines for teams and multi-device setups. ukrop is a simpler, local-first tool that covers
more use cases in one interface.

### vs Television

[Television](https://github.com/alexpasmantier/television) is a Rust-based general-purpose fuzzy finder launched in
2025, quickly gaining popularity (~7k stars).

**Where ukrop wins:**

- Purpose-built for shell workflow: directory jumping, command history, and SSH host picking in one tool
- Frecency scoring with exponential decay ranks results by actual usage patterns
- Automatic history tracking via shell hooks -- no manual piping needed
- SSH config import and integrated SSH host picker
- Two-tier search with substring priority over fuzzy matches
- Favorites, CWD filtering, edit-before-execute, clipboard copy
- Exit code tracking for commands
- Setup wizard for quick shell integration

**Where Television wins:**

- General-purpose channel system: files, git repos, text search, environment variables, and more
- File preview with syntax highlighting
- Extensible via custom channels (TOML definitions)
- Can replace fzf in most piping workflows
- Growing plugin ecosystem

**Bottom line:** Television is a powerful general-purpose fuzzy finder. ukrop is a specialized tool that understands
shell workflows natively -- it tracks what you use, scores by frecency, and presents directories, commands, and SSH
hosts in a unified TUI without requiring manual configuration.

## Summary

Most shell productivity tools specialize in either directory jumping or command history. ukrop is designed to handle
both -- plus SSH -- in a single frecency-scored, fuzzy-searchable TUI with features like two-tier search (substring
priority + fuzzy fallback), match highlighting, edit-before-execute, clipboard copy, CWD filtering, favorites,
configurable ignore patterns, auto-cleanup of stale directories, and an interactive setup wizard.

General-purpose fuzzy finders like fzf and Television offer broader applicability (files, git, processes, etc.) but
require manual setup for shell-specific workflows. Adjacent tools like [navi](https://github.com/denisidoro/navi) (
interactive cheatsheets) and [pet](https://github.com/knqyf263/pet) (snippet manager) complement shell history tools but
serve different purposes.

ukrop includes a configuration file (`~/.config/ukrop/config.toml`) for customizing scoring weights, ignore patterns,
theme presets, and ranking weights, with an in-TUI config editor (F9) that provides live preview of changes.

The trade-off is that ukrop does not offer cloud sync or the massive plugin ecosystems of tools like fzf and zoxide.

Choose ukrop if you want **one tool** for navigating directories, recalling commands, and connecting to SSH hosts, with
no external dependencies and minimal setup.
