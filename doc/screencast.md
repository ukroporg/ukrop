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

## Scene 3: Unified List Overview (0:50 - 1:30)

```sh
u
```

The TUI opens on one ranked list mixing directories, commands, and SSH hosts — no panels to choose between. Each row
carries a sigil: `/` for a directory, `$` for a command, `@` for an SSH host.

Pause to let the viewer see the layout. Point out:

- **Sigils**: `/` directories sorted by locality/frecency, `$` commands with full history, `@` SSH hosts with
  connection details, all interleaved in one list so the top stays type-diverse
- **Title bar**: shows the match count, the active type filter (`All` by default), and the cwd-filter tag when on
- **Details bar** at the bottom: path, visit count, last visit, exists/missing status
- **Shortcut bar**: F1 help, F9 config, Tab (type filter), Enter, etc.

Voice: "Everything in one place, one ranked list. Directories, commands, SSH hosts. The most likely thing you want — right now, from here — floats to the top automatically."

---

## Scene 4: Fuzzy Search (1:30 - 2:15)

Start typing in the search bar:

1. Type `web` -- the list filters across all three types at once. webapp paths, web-related commands, and web servers all show up, ranked together.
2. Clear with Ctrl+U.
3. Type `docker` -- docker commands surface, along with any matching directory or host.
4. Clear. Type `prod` -- prod servers and deploy commands surface together.

Voice: "One search box filters everything. Substring matches first, fuzzy fallback. The matched characters are highlighted so you always know why something matched."

---

## Scene 5: Type Filter with Tab (2:15 - 2:50)

1. Press **Tab** to cycle the type filter: All → cd → run → ssh → All (shown in the title bar).
2. Press **Shift+Tab** to cycle backward.
3. Use **Up/Down** to navigate the list. Show the details bar updating as you move.
4. Press **Ctrl+W** to show the cwd filter narrowing the list to rows tied to the current directory, then press it again to clear it.

Voice: "Tab cycles a type filter instead of switching panels — the ranking underneath doesn't change, it just narrows what's shown. Ctrl+W narrows further to only what's tied to where you are right now."

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

Press **F9** to open the config editor. A modal overlay appears on top of the TUI showing all settings grouped into sections:

- **Scoring** -- frecency_weight, substring_bonus, prefix_bonus (tune how results are ranked)
- **Cleanup** -- stale_days (how long to keep missing directories)
- **Theme** -- preset selector with `< Left/Right >` arrows to cycle through themes, plus toggles for selection_bold, match_underline, favorite_italic
- **Behavior** -- confirm_delete
- **Ignore Patterns** -- add patterns to exclude commands from tracking

(The old **Layout** section — panel width ratios — is gone from the dialog now that there's one list instead of three
panels. `[layout]` is still parsed from an existing `config.toml` for backward compatibility, but it has no effect
and isn't shown here.)

Steps to show:

1. Navigate to Theme > preset. Use **Left/Right** arrows to cycle: Default, Gruvbox, Nord, Dracula, Catppuccin. The list behind updates in real-time with each theme change -- the viewer sees colors shift live behind the config dialog.
2. Toggle **favorite_italic** on -- show how starred items become italic.
3. Adjust a scoring weight, e.g. bump **prefix_bonus** -- no live re-rank needed for the demo, just show the field editing.
4. Press **Esc** to cancel and revert all changes. Open again with **F9**, pick Gruvbox, press **F9** to save.

Voice: "F9 opens the config editor. 12 built-in themes with live preview -- the list behind updates as you browse. Tweak scoring weights, ignore patterns. F9 saves, Esc cancels and reverts everything."

---

## Scene 11: Closing (4:55 - 5:00)

```sh
u --help
```

Voice: "ukrop. Install it, set it up, forget about cd and Ctrl+R. Link in the description."

Show the Ukraine donation link from the help output.
