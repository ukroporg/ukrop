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

| Query | Substring matches        | Fuzzy-only matches         |
| ----- | ------------------------ | -------------------------- |
| `gp`  | —                        | `git push`, `grep`         |
| `curl`| `curl -H ...`, `curling` | —                          |
| `auth`| `authentication_ctrl`    | `authconrb` (not a real example, but `a..u..t..h` would match) |

### Spaces in query — substring only

When the query contains a space, it switches to literal substring matching.
The entire query including spaces must appear as-is in the item.

| Query     | Matches                   | Doesn't match      |
| --------- | ------------------------- | ------------------ |
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
combined = fuzzy_score + prefix_bonus + substring_bonus + frecency_bonus + brevity_bonus
```

### Components

| Component          | Range         | Description                                                                                                                         |
| ------------------ | ------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `fuzzy_score`      | 0–300 typical | Match quality score from nucleo. Higher when characters are closer together, at word boundaries, etc.                               |
| `prefix_bonus`     | 0 or 10,000   | Applied when the item text starts with the query (case-insensitive). Ensures prefix matches always appear above non-prefix matches. |
| `substring_bonus`  | 0 or 8,000    | Applied when the query appears as a contiguous substring. Ensures substring matches rank above fuzzy-only matches.                  |
| `frecency_bonus`   | 0–5,000       | `min(frecency_score * frecency_weight, 5000)`. Items used more frequently and more recently get a higher bonus.                     |
| `brevity_bonus`    | 0–3,000       | `max(0, 3000 - length * 15)`. Shorter commands get a higher bonus, so concise commands rank above long ones.                        |

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

Within each tier, items are further sorted by fuzzy quality + frecency + brevity.
Shorter commands get a significant boost (up to 3,000) so concise commands
rank above long compound commands with the same query match.

**Example for query `gp`:**

| Item                      | fuzzy | prefix | substring | frecency (score) | brevity | combined |
| ------------------------- | ----- | ------ | --------- | ---------------- | ------- | -------- |
| `gp` (score=5.0)          | 100   | 10,000 | 8,000     | 500              | 2,970   | 21,570   |
| `grep foo` (score=3.0)    | 80    | 0      | 8,000     | 300              | 2,865   | 11,245   |
| `git push` (score=8.0)    | 60    | 0      | 0         | 800              | 2,865   | 3,725    |
| `git pull` (score=2.0)    | 60    | 0      | 0         | 200              | 2,865   | 3,125    |

## Empty query

When the search bar is empty, items are shown in database order:
favorites first, then sorted by frecency score descending. No fuzzy
scoring is applied.
