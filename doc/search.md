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

Ukrop shows one ranked list mixing directories (`cd`), commands (`run`), and
SSH hosts (`ssh`). Every row — regardless of type — is scored by the same
formula, so the three types genuinely compete for the top of the list instead
of being ranked separately per panel:

```
score = match_score + frecency + recency + locality + brevity + favorite + type_bonus
```

`match_score` is `prefix_bonus`/`substring_bonus`/`fuzzy_penalty` (mutually
exclusive tiers, see below) plus the raw `fuzzy_score`. `locality` is
`cwd_bonus` for `run` rows or the transition-based bonus for `cd`/`ssh` rows.
`type_bonus` is the position-dependent diversity adjustment described in
[Diversity re-rank](#diversity-re-rank); it is the only component that isn't a
pure function of the row itself.

### Components

| Component            | Value                              | Applies to                                    | Notes                                                              |
|-----------------------|-------------------------------------|------------------------------------------------|---------------------------------------------------------------------|
| `prefix_bonus`        | +10,000                             | display text starts with query (case-insensitive) | Stacks with `substring_bonus` — see below                          |
| `substring_bonus`     | +8,000                              | query appears as a contiguous substring         | Stacks with `prefix_bonus`                                          |
| `fuzzy_penalty`       | −4,000                              | fuzzy-only match (no substring)                 | Never combined with prefix/substring bonuses                        |
| `fuzzy_score`         | 0–300                               | any active query                                | Raw nucleo match-quality score                                     |
| `favorite_bonus`      | +5,000                              | `is_favorite`                                   |                                                                       |
| `recency_24h_bonus`   | +6,000                              | `last_time` within 24h                          | Mutually exclusive with the 7d tier                                 |
| `recency_7d_bonus`    | +2,500                              | `last_time` between 24h and 7 days              | Mutually exclusive with the 24h tier                                |
| `frecency_weight`     | 0–5,000 (`frecency_score × 100.0`, capped) | all rows                                | Capped by `frecency_cap` (default 5,000)                            |
| `cwd_bonus`           | +4,000                              | `run` row whose recorded `cwd` equals the current directory | Boolean — see [Locality](#locality-cwd_bonus-vs-transition_bonus)   |
| `transition_weight`   | 0–4,000 (`transition_score × 100.0`, capped) | `cd`/`ssh` row with a recorded transition from the current directory | Capped by `transition_cap` (default 4,000)                          |
| `brevity_bonus`       | 0–3,000 (`3000 − display_len × 15`, floored at 0) | all rows                          | Computed on the full display string, char count not byte length     |
| `type_bonus`          | +3,000 → −3,000                     | `cd` and `ssh` rows only                        | Position-dependent, applied by the diversity pass — see below       |

`last_time` is normalized across types — `last_visit` for directories,
`last_used` for commands and SSH hosts.

### Prefix and substring bonuses stack

A prefix match is necessarily also a substring match, so both bonuses apply
together: **+18,000 combined**, not a choice between the two. `fuzzy_penalty`
is the only match-quality term that is never combined with the other two — it
applies solely when the query matches by character order but not as a
contiguous substring.

### Recency: mutually exclusive tiers, not a curve

Recency is a step function, not a smooth curve, and it stacks on top of
frecency. Frecency's 1-week half-life is far too gentle to reflect "I ran this
ten minutes ago" — a command used once today has a database score of ~1.0,
worth only +100 via `frecency_weight`. The recency tiers exist specifically to
make today's work dominate:

- `last_time` within 24h → **+6,000**
- `last_time` between 24h and 7 days → **+2,500**
- older than 7 days → **+0**

These tiers are mutually exclusive — a row 2 hours old receives +6,000, never
+8,500. A row 23h old ranks well above one 25h old; that discontinuity at the
24h boundary is intentional and predictable, not a rounding artifact. A smooth
curve would just be a second, redundant frecency term and wouldn't produce the
"today dominates" effect the step function gives.

### Locality: `cwd_bonus` vs `transition_bonus`

Locality — "is this thing tied to where I am right now" — is one axis
expressed in two shapes, both capped at **+4,000** so they're comparable
across types:

- **`run` rows** — locality is boolean. `commands.cwd` either equals the
  current directory or it doesn't; a match is worth the full `cwd_bonus`
  (+4,000).
- **`cd` and `ssh` rows** — locality is graded, driven by the `transitions`
  table (see [doc/usage.md](usage.md) for the schema). A host reached from
  here twenty times a day outranks one reached once. The decayed transition
  score is scaled by `transition_weight` and capped at `transition_cap`
  (+4,000), the same ceiling as `cwd_bonus`.

### Fuzzy penalty: strong but crossable

`fuzzy_penalty` (−4,000) is deliberately large enough to keep fuzzy-only
matches below substring matches in the common case, but not so large that no
combination of merit can overcome it — "strong but crossable."

Worked example, `cwd = ~/www/gupalo/ukrop`, query `car`. Note that `clear` is
a **fuzzy-only** match (`c`…`a`…`r` occurs in order in `clear`) despite not
containing `car` as a substring, while `carcass` matches `car` as a literal
substring:

| Row | match | frec | recency | local | brev | fav | type | **total** |
|---|---|---|---|---|---|---|---|---|
| `/ ~/www/gupalo/ukrop/target` (2h, from here) | 8,050 | 300 | 6,000 | 3,200 | 2,625 | 0 | +3,000 (cd#1) | **23,175** |
| `$ cargo build` (2h ago, here) | 8,150 | 800 | 6,000 | 4,000 | 2,835 | 0 | 0 | **21,785** |
| `@ carbon-prod -> root@10.0.0.4` (3d, from here) | 8,100 | 250 | 2,500 | 2,400 | 2,580 | 0 | +3,000 (ssh#1) | **18,830** |
| `$ cargo test --release` (5d, here) | 8,120 | 400 | 2,500 | 4,000 | 2,700 | 0 | 0 | **17,720** |
| `/ ~/old/carcass` (4mo, missing) | 8,000 | 20 | 0 | 0 | 2,805 | 0 | +1,500 (cd#2) | **12,325** |
| `$ clear` (10min, here) | −3,900 | 900 | 6,000 | 4,000 | 2,925 | 0 | 0 | **9,925** |

`clear` is ten minutes old and used in this exact directory, yet the −4,000
fuzzy penalty keeps it below the stale, missing, four-month-old
`~/old/carcass`, which matches `car` literally. That's the intended behavior:
crossable in principle, but it takes an overwhelming advantage on every other
axis to do it. The `type` column shows the emission position the diversity
pass assigned (see below), which is why the second `cd` row receives +1,500
rather than the full +3,000.

### Diversity re-rank

`cd` and `ssh` rows receive a bonus that decays with how many rows of that
same type have already been placed above them in the final list. `run` rows
never receive a type bonus.

```
schedule[n] for cd and ssh:
    n = 0  → +3,000
    n = 1  → +1,500
    n = 2  →       0
    n = 3  → −1,500
    n ≥ 4  → −3,000   (floor)
```

**The two types have independent counters.** Three `cd` rows near the top do
not consume any of the `ssh` budget — the first `ssh` row still gets the full
+3,000 regardless of how many `cd` rows precede it.

**The floor at −3,000 is load-bearing.** Without it, a long run of one type
would drive its bonus toward negative infinity and permanently lock that type
out of the rest of the list. With the floor, a `cd` row that is 10,000 points
better on merit than everything around it still surfaces a bit further down
rather than never.

Because the bonus depends on final position, it can't be folded into the base
score and sorted in one pass — it's computed by a three-way merge: sort each
type's rows by base score, then greedily emit whichever type's next row has
the highest `base_score + schedule[count[type]]` at each step, advancing that
type's counter. Within a type, the relative order never changes, since every
remaining row of that type gets the same bonus at any given step.

**Filtered views skip the pass entirely.** When the type filter is anything
other than `All`, only one type is present in the list, so the re-rank would
be a no-op — those views are pure base-score order.

## Empty query

With an empty search bar the full formula still applies, minus the
query-dependent terms (`prefix_bonus`, `substring_bonus`, `fuzzy_penalty`,
`fuzzy_score`), and the diversity re-rank runs normally. This makes the
opening screen a genuine "what do I most likely want, here, now" list, ranked
by recency, locality, frecency, favorites, and brevity — rather than raw
database order. Typing the first character adds the match-quality terms on
top of the same ranking rather than switching to a different scoring regime,
so there's no discontinuity between the empty and non-empty views.

## Configuration

Every value above is a key under `[scoring]` in `~/.config/ukrop/config.toml`,
shown here with its default:

```toml
[scoring]
frecency_weight     = 100.0   # scale factor applied to the frecency score
frecency_cap        = 5000    # ceiling for the frecency term
substring_bonus     = 8000
prefix_bonus        = 10000
fuzzy_penalty       = -4000
favorite_bonus      = 5000
recency_24h_bonus   = 6000
recency_7d_bonus    = 2500
cwd_bonus           = 4000
transition_weight   = 100.0   # scale factor applied to the transition score
transition_cap      = 4000
brevity_bonus_max   = 3000

[scoring.type_bonus]
schedule = [3000, 1500, 0, -1500, -3000]   # index = count of that type already emitted; last value is the floor
```

`schedule` applies only to `cd` and `ssh` rows; `run` rows always receive a
type bonus of 0. Missing keys fall back to defaults, so existing config files
keep working unchanged.
