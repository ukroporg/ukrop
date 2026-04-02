# Search & Ranking

## Search modes

Ukrop uses a two-tier search strategy. Substring (exact) matches always rank
above fuzzy-only matches thanks to a dedicated bonus.

### No spaces in query — substring + fuzzy

When the query has no spaces, each item is first tested for a **substring** match.
If the query appears as a contiguous substring, the item receives a substring bonus
and ranks higher. Items that don't contain the substring are still matched with
**fuzzy** matching (characters in order but not necessarily adjacent), so they
still appear — just below the substring matches.

| Query  | Substring matches        | Fuzzy-only matches                                             |
|--------|--------------------------|----------------------------------------------------------------|
| `gp`   | —                        | `git push`, `grep`                                             |
| `curl` | `curl -H ...`, `curling` | —                                                              |
| `auth` | `authentication_ctrl`    | `authconrb` (not a real example, but `a..u..t..h` would match) |

### Spaces in query — substring only

When the query contains a space, it switches to literal substring matching.
The entire query including spaces must appear as-is in the item.

| Query     | Matches                   | Doesn't match      |
|-----------|---------------------------|--------------------|
| `c `      | `c test`, `c d:m:m`       | `cat`, `curl`      |
| `git p`   | `git push`, `git pull`    | `gitpod`           |
| `curl -H` | `curl -H 'User-Agent...'` | `curl https://...` |

## Match highlighting

Matched characters are highlighted in the TUI with **cyan + underline** styling,
so you can see exactly which characters in each entry matched the query. This
works for both substring and fuzzy matches.

## Ranking formula

When a query is active, matched items are ranked by a combined score:

```
combined = fuzzy_score + prefix_bonus + substring_bonus + frecency_bonus + brevity_bonus + cwd_bonus
```

### Components

| Component         | Range         | Description                                                                                                                                                       |
|-------------------|---------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `fuzzy_score`     | 0–300 typical | Match quality score from nucleo. Higher when characters are closer together, at word boundaries, etc.                                                             |
| `prefix_bonus`    | 0 or 10,000   | Applied when the item text starts with the query (case-insensitive). Ensures prefix matches always appear above non-prefix matches.                               |
| `substring_bonus` | 0 or 8,000    | Applied when the query appears as a contiguous substring. Ensures substring matches rank above fuzzy-only matches.                                                |
| `frecency_bonus`  | 0–5,000       | `min(frecency_score * frecency_weight, 5000)`. Items used more frequently and more recently get a higher bonus.                                                   |
| `brevity_bonus`   | 0–3,000       | `max(0, 3000 - length * 15)`. Shorter commands get a higher bonus, so concise commands rank above long ones.                                                      |
| `cwd_bonus`       | 0 or 4,000    | Applied when the command was previously run in the current working directory. CWD is recorded by shell hooks or guessed from `cd` commands during `ukrop import`. |

All bonus values are configurable in `~/.config/ukrop/config.toml`:

```toml
[scoring]
frecency_weight = 100.0   # default
substring_bonus = 8000    # default
prefix_bonus = 10000      # default
```

### Frecency score

The frecency score comes from the database. Each use adds 1.0 to the score, and
the score decays with a 1-week half-life (exponential decay). This means:

- An item used once today has a score of ~1.0
- After 1 week without use, the score halves to ~0.5
- After 2 weeks, it drops to ~0.25
- An item used 10 times today has a score of ~10.0

The frecency bonus is capped at 5,000 (reached at frecency_score = 50.0) to
prevent very frequently used items from dominating over better fuzzy matches.

### Effect on ranking

The bonuses create a clear ranking hierarchy:

1. **Prefix matches** (prefix_bonus 10,000) always appear first.
2. **Substring matches** (substring_bonus 8,000) appear next — items that
   contain the query as a contiguous substring but don't start with it.
3. **Fuzzy-only matches** (no bonus) appear last — items where the query
   characters appear in order but not adjacent.

Within each tier, items are further sorted by fuzzy quality + frecency + brevity + cwd.
Shorter commands get a significant boost (up to 3,000) so concise commands
rank above long compound commands with the same query match. Commands
previously run in the current directory get an additional 4,000 bonus.

**Example for query `gp` (cwd = /project):**

| Item                       | fuzzy | prefix | substring | frecency | brevity | cwd   | combined |
|----------------------------|-------|--------|-----------|----------|---------|-------|----------|
| `gp` (score=5.0, cwd=here) | 100   | 10,000 | 8,000     | 500      | 2,970   | 4,000 | 25,570   |
| `gp` (score=5.0)           | 100   | 10,000 | 8,000     | 500      | 2,970   | 0     | 21,570   |
| `grep foo` (score=3.0)     | 80    | 0      | 8,000     | 300      | 2,865   | 0     | 11,245   |
| `git push` (score=8.0)     | 60    | 0      | 0         | 800      | 2,865   | 0     | 3,725    |

### CWD tracking

The working directory for each command is tracked in two ways:

1. **Shell hooks** (live): When shell integration is active (`eval "$(ukrop init zsh)"`),
   each command is recorded with its actual `$PWD`.

2. **History import** (`ukrop import`): When importing from shell history, the cwd is
   *guessed* by tracking `cd` commands. The parser walks history in chronological order
   and maintains a `current_dir` state — when it sees `cd /some/path`, it updates
   `current_dir` and assigns it to subsequent commands. Only absolute paths and `~/...`
   paths are tracked; relative `cd foo` is skipped since it can't be resolved without
   knowing the prior directory.

## Empty query

When the search bar is empty, items are shown in database order:
favorites first, then sorted by frecency score descending. No fuzzy
scoring is applied.
