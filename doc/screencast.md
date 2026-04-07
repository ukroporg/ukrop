# Screencast Scenario (~5 minutes)

Target: developers who use the terminal daily and want faster navigation.

## Preparation

```sh
ukrop export --file ~/my-data.jsonl
ukrop demo
```

After recording, restore with `ukrop import --file ~/my-data.jsonl`.

---

## Scene 1: The Problem (0:00 - 0:20)

Terminal. Show the pain points quickly:

```sh
cd ~/projects/api-server/src/handlers    # too much typing
history | grep docker                    # noisy, hard to find the right one
cat ~/.ssh/config                        # scroll through hosts to remember the alias
```

Voice: "We all do this dozens of times a day. There's a faster way."

---

## Scene 2: Install & Setup (0:20 - 0:50)

```sh
brew install ukroporg/tap/ukrop
ukrop setup
```

Show setup prompting to import history and add shell integration. Accept both.

Voice: "One command installs, one command sets up. It imports your shell history, SSH config, and adds shell integration. That's it."

Restart the shell (or `source ~/.zshrc`).

---

## Scene 3: Three-Panel TUI Overview (0:50 - 1:30)

```sh
u
```

The TUI opens with all three panels visible: cd (directories), run (commands), ssh (hosts).

Pause to let the viewer see the layout. Point out:

- **cd panel** (top-left): directories sorted by frecency, favorites starred at top
- **run panel** (right): commands with full history
- **ssh panel** (bottom-left): SSH hosts with connection details
- **Details bar** at the bottom: path, visit count, last visit, exists/missing status
- **Shortcut bar**: F1 help, F2 config, Tab, Enter, etc.

Voice: "Everything in one place. Directories, commands, SSH hosts. The most-used items float to the top automatically."

---

## Scene 4: Fuzzy Search (1:30 - 2:15)

Start typing in the search bar:

1. Type `web` -- all three panels filter simultaneously. cd shows webapp paths, run shows web-related commands, ssh shows web servers.
2. Clear with Ctrl+U.
3. Type `docker` -- run panel highlights docker commands, cd shows infra directory.
4. Clear. Type `prod` -- ssh panel shows prod servers, run shows deploy commands.

Voice: "One search box filters everything. Substring matches first, fuzzy fallback. The matched characters are highlighted so you always know why something matched."

---

## Scene 5: Switching Panels with Tab (2:15 - 2:50)

1. Press **Tab** to switch active panel from cd to run (green border moves).
2. Press **Tab** again to switch to ssh.
3. Press **Shift+Tab** to go back.
4. Use **Up/Down** to navigate within the active panel. Show the details bar updating as you move.

Voice: "Tab switches panels. Arrow keys navigate. The active panel has the green border."

---

## Scene 6: Jump to Directory (2:50 - 3:15)

```sh
u cd
```

1. Type `api` to filter.
2. Press **Enter** on `api-server`. Terminal cd's into the directory.
3. Run `pwd` to confirm.

Voice: "Type a few characters, hit Enter, you're there. No more typing long paths."

---

## Scene 7: Re-Run a Command (3:15 - 3:50)

Press **Ctrl+R** (replaces built-in reverse search).

1. Type `compose` -- docker compose commands appear.
2. Press **Enter** on `docker compose up -d`. Command runs immediately.

Then open again:

3. Type `cargo`. Select `cargo test` but press **Shift+Enter** instead -- command is pasted into the terminal for editing before running.

Voice: "Ctrl+R replaces your shell's reverse search. Enter runs immediately. Shift+Enter pastes the command so you can edit it first."

---

## Scene 8: SSH Connect (3:50 - 4:10)

```sh
u ssh
```

1. Type `bast` -- bastion host highlighted.
2. Press **Enter** -- SSH connection starts (Ctrl+C to cancel for demo).

Voice: "SSH hosts from your config and history, all searchable. Pick a host and connect."

---

## Scene 9: Favorites & Management (4:10 - 4:35)

```sh
u
```

1. Navigate to a directory, press **Ctrl+F** -- star appears, item moves to top.
2. Navigate to a command, press **Ctrl+F** -- starred.
3. Press **Ctrl+Del** on an entry -- confirmation prompt, delete it.
4. Press **Ctrl+Y** on a command -- copied to clipboard.

Voice: "Ctrl+F to favorite, Ctrl+Del to delete, Ctrl+Y to copy. Favorites always stay at the top."

---

## Scene 10: Themes & Config (4:35 - 4:55)

Press **F2** to open the config editor. A modal overlay appears on top of the TUI showing all settings grouped into sections:

- **Scoring** -- frecency_weight, substring_bonus, prefix_bonus (tune how results are ranked)
- **Cleanup** -- stale_days (how long to keep missing directories)
- **Theme** -- preset selector with `< Left/Right >` arrows to cycle through themes, plus toggles for selection_bold, match_underline, favorite_italic
- **Layout** -- left_panel_pct and cd_panel_pct (adjust panel proportions)
- **Ignore Patterns** -- add patterns to exclude commands from tracking

Steps to show:

1. Navigate to Theme > preset. Use **Left/Right** arrows to cycle: Default, Gruvbox, Nord, Dracula, Catppuccin. The background panels update in real-time with each theme change -- the viewer sees colors shift live behind the config dialog.
2. Toggle **favorite_italic** on -- show how starred items become italic.
3. Change **left_panel_pct** from 25 to 35 -- panels resize live.
4. Press **Esc** to cancel and revert all changes. Open again with **F2**, pick Gruvbox, press **F2** to save.

Voice: "F2 opens the config editor. 12 built-in themes with live preview -- the panels behind update as you browse. Tweak layout ratios, scoring weights, ignore patterns. F2 saves, Esc cancels and reverts everything."

---

## Scene 11: Closing (4:55 - 5:00)

```sh
u --help
```

Voice: "ukrop. Install it, set it up, forget about cd and Ctrl+R. Link in the description."

Show the Ukraine donation link from the help output.
