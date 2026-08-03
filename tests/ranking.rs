use ukrop::config::ScoringConfig;
use ukrop::db::Transitions;
use ukrop::tui::{PickerEntry, PickerMode, Row, TypeFilter, UnifiedList};

const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn entry(display: &str, score: f64, age_secs: i64) -> PickerEntry {
    PickerEntry {
        display: display.to_string(),
        value: display.to_string(),
        connect_value: None,
        score,
        is_favorite: false,
        last_time: now() - age_secs,
        use_count: 1,
        exists: None,
        duration_ms: None,
        cwd: None,
    }
}

fn shown(l: &UnifiedList) -> Vec<String> {
    l.ranked
        .iter()
        .map(|r| l.rows[r.row_idx].entry.display.clone())
        .collect()
}

#[test]
fn test_todays_command_beats_a_more_frequent_stale_one() {
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Commands, entry: entry("cargo build", 1.0, 2 * HOUR) },
            Row { kind: PickerMode::Commands, entry: entry("cargo clippy", 30.0, 40 * DAY) },
        ],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.update_filter("cargo");
    assert_eq!(
        shown(&l)[0],
        "cargo build",
        "the +6000 recency tier must outweigh a large frecency lead"
    );
}

#[test]
fn test_substring_match_beats_fuzzy_match_of_equal_merit() {
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Commands, entry: entry("clear", 1.0, HOUR) },
            // Must NOT start with the query, or it classifies as MatchKind::Prefix
            // (prefix_bonus + substring_bonus) instead of the MatchKind::Substring
            // (substring_bonus alone) this test means to pin — see the fixture bug
            // found and fixed in the two fuzzy-penalty tests below for exactly this
            // failure mode.
            Row { kind: PickerMode::Commands, entry: entry("~/old/carcass", 1.0, HOUR) },
        ],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.update_filter("car");
    // "~/old/carcass" contains "car" as a substring (not a prefix, since the
    // string starts with "~"); "clear" only fuzzy-matches c-a-r. This is the
    // substring-vs-fuzzy tier at equal merit (frecency, recency, favorite,
    // cwd all tied) — the boundary the spec's worked example is about.
    assert_eq!(shown(&l)[0], "~/old/carcass");
}

// Pins the fuzzy/substring boundary from the approved spec's worked example
// (.claude/superpowers/specs/2026-08-02-unified-picker-design.md:221), using
// the spec's own literal fixture string `~/old/carcass` so there is zero room
// to wonder whether the fixture was tuned rather than corrected: a fuzzy
// match with favorite + cwd + 24h-recency stacked on top can outscore a stale,
// SUBSTRING-only match (fuzzy_penalty -4,000 vs. substring_bonus +8,000 is a
// 12,000 gap, crossable by 15,000 of stackable merit).
//
// Measured base scores (ScoringConfig::default()):
//   "clear" (fuzzy, favorite, cwd_match, 60s old):        base = 18,991
//   "~/old/carcass" (substring, 200d stale, no favorite):  base = 10,891
// "clear" wins by 8,100. Note this margin also banks a ~4,998-point frecency
// edge (50.0 vs. 0.02) on top of the 15,000 of favorite/cwd/recency merit, so
// it does not tightly pin the 12,000-vs-15,000 boundary on its own — losing
// just the favorite weight, for instance, would still likely pass here. That
// narrower regression (a favorite failing to surface) is what
// `test_favorites_survive_the_empty_query_view` below is for.
//
// The stale fixture must NOT start with the query ("car"), or it becomes a
// PREFIX match instead of a substring match, which is a different, much wider
// gap that this test is not meant to probe (see
// test_fuzzy_penalty_is_not_crossable_by_favorite_cwd_and_recency_alone below).
#[test]
fn test_fuzzy_penalty_is_crossable_by_overwhelming_merit() {
    let mut fresh = entry("clear", 50.0, 60);
    fresh.cwd = Some("/here".to_string());
    fresh.is_favorite = true;
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Commands, entry: fresh },
            Row { kind: PickerMode::Commands, entry: entry("~/old/carcass", 0.02, 200 * DAY) },
        ],
        Some("/here".to_string()),
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.update_filter("car");
    assert_eq!(
        shown(&l)[0],
        "clear",
        "a fuzzy match with favorite + cwd + recency should still be able to win"
    );
}

// Companion to the test above: pins the OTHER side of the boundary. A prefix
// match ("carcass-utility-script" starts with "car") is the strongest
// possible signal that the user typed the thing they want, and the fuzzy
// penalty must NOT be crossable against it by favorite + cwd + 24h recency
// alone (fuzzy_penalty -4,000 vs. prefix_bonus + substring_bonus +18,000 is a
// 22,000 gap, wider than the 15,000 of favorite/cwd/recency merit that closed
// the substring-only gap above).
//
// This is deliberately scoped to "by favorite + cwd + recency alone": prefix
// IS crossable in a degenerate corner not exercised here — max theoretical
// merit differential is favorite 5,000 + cwd 4,000 + recency 6,000 +
// frecency_cap 5,000 + brevity_max 3,000 = 23,000 > the 22,000 gap, reachable
// when the prefix competitor also has a >=200-char display (brevity floors at
// 0) and zero frecency. This test's fixture keeps the competitor's frecency
// and brevity intact, so only the favorite/cwd/recency stack is under test.
//
// Measured base scores for this exact fixture (ScoringConfig::default()):
//   "clear" (fuzzy, favorite, cwd_match, 60s old):            base = 18,991
//   "carcass-utility-script" (prefix+substring, 200d stale):  base = 20,756
// The stale prefix match wins by 1,765. That margin is deliberately a knife
// edge, not a comfortable one: it will flip under an intentional
// recalibration (e.g. favorite_bonus 5,000 -> 6,800 alone closes it). If this
// assertion starts failing, read it as a signal that the weights moved —
// check `src/config.rs` for an intentional change before assuming a bug —
// not as proof the tier boundary broke.
#[test]
fn test_fuzzy_penalty_is_not_crossable_by_favorite_cwd_and_recency_alone() {
    let mut fresh = entry("clear", 50.0, 60);
    fresh.cwd = Some("/here".to_string());
    fresh.is_favorite = true;
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Commands, entry: fresh },
            Row { kind: PickerMode::Commands, entry: entry("carcass-utility-script", 0.02, 200 * DAY) },
        ],
        Some("/here".to_string()),
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.update_filter("car");
    assert_eq!(
        shown(&l)[0],
        "carcass-utility-script",
        "a stale prefix match must not be beaten by a fuzzy match, even with favorite + cwd + recency stacked"
    );
}

#[test]
fn test_transition_target_outranks_an_unrelated_directory() {
    let t = Transitions::from([("cd", "/proj/api", 25.0)]);
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Directories, entry: entry("/proj/api", 1.0, 5 * DAY) },
            Row { kind: PickerMode::Directories, entry: entry("/proj/web", 4.0, 5 * DAY) },
        ],
        Some("/proj".to_string()),
        t,
        ScoringConfig::default(),
    );
    l.update_filter("");
    assert_eq!(shown(&l)[0], "/proj/api");
}

#[test]
fn test_empty_query_top_of_list_is_type_diverse() {
    // 20 recent commands and one recent directory: the directory must not be buried.
    let mut rows: Vec<Row> = (0..20)
        .map(|i| Row {
            kind: PickerMode::Commands,
            entry: entry(&format!("cmd{:02}", i), 5.0, HOUR),
        })
        .collect();
    rows.push(Row {
        kind: PickerMode::Directories,
        entry: entry("/proj", 5.0, HOUR),
    });
    let mut l = UnifiedList::new(rows, None, Transitions::new(), ScoringConfig::default());
    l.update_filter("");
    let dir_pos = l
        .ranked
        .iter()
        .position(|r| r.kind == PickerMode::Directories)
        .unwrap();
    assert!(dir_pos < 5, "the only directory should surface near the top, got {}", dir_pos);
}

#[test]
fn test_type_filter_shows_only_that_type() {
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Directories, entry: entry("/proj", 1.0, HOUR) },
            Row { kind: PickerMode::Commands, entry: entry("ls", 1.0, HOUR) },
            Row { kind: PickerMode::SshHosts, entry: entry("prod", 1.0, HOUR) },
        ],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    for (filter, expected) in [
        (TypeFilter::Cd, "/proj"),
        (TypeFilter::Run, "ls"),
        (TypeFilter::Ssh, "prod"),
    ] {
        l.filter = filter;
        l.update_filter("");
        assert_eq!(shown(&l), vec![expected.to_string()]);
    }
}

#[test]
fn test_favorites_survive_the_empty_query_view() {
    let mut fav = entry("rarely-used-but-starred", 0.02, 100 * DAY);
    fav.is_favorite = true;
    let mut rows: Vec<Row> = (0..10)
        .map(|i| Row {
            kind: PickerMode::Commands,
            entry: entry(&format!("common{:02}", i), 3.0, 3 * DAY),
        })
        .collect();
    rows.push(Row { kind: PickerMode::Commands, entry: fav });
    let mut l = UnifiedList::new(rows, None, Transitions::new(), ScoringConfig::default());
    l.update_filter("");
    let pos = shown(&l)
        .iter()
        .position(|s| s == "rarely-used-but-starred")
        .unwrap();
    assert!(pos < 3, "a favorite must stay near the top, got position {}", pos);
}

#[test]
fn test_clustered_fuzzy_match_outranks_a_scattered_one() {
    // Query "seo2". Neither row contains it literally, so both land in the
    // fuzzy tier. The scattered row is deliberately given the higher frecency
    // and the shorter length, so it wins on every other signal — only the
    // contiguity bonus can lift the clustered row above it. Zero out
    // `contiguity_weight` and this assertion flips.
    let mut l = UnifiedList::new(
        vec![
            Row {
                kind: PickerMode::Commands,
                // s..e..o..2, every matched character isolated
                entry: entry("svc deploy prod v2", 12.0, 2 * DAY),
            },
            Row {
                kind: PickerMode::Commands,
                // "seo" stays together, then a lone "2"
                entry: entry("run seo task v2", 5.0, 2 * DAY),
            },
        ],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.filter = TypeFilter::Run;
    l.update_filter("seo2");
    assert_eq!(
        shown(&l),
        vec!["run seo task v2".to_string(), "svc deploy prod v2".to_string()]
    );
}

#[test]
fn test_literal_substring_still_beats_the_best_fuzzy_match() {
    // Contiguity must not let a fuzzy row jump the substring tier, however
    // tightly its characters cluster.
    let mut l = UnifiedList::new(
        vec![
            Row {
                kind: PickerMode::Commands,
                entry: entry("gcx login seo --org-id 2", 30.0, HOUR),
            },
            Row { kind: PickerMode::Commands, entry: entry("ssh seo2", 0.1, 60 * DAY) },
        ],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.filter = TypeFilter::Run;
    l.update_filter("seo2");
    assert_eq!(shown(&l)[0], "ssh seo2");
}

#[test]
fn test_spaced_query_keeps_token_matches_below_the_literal_phrase() {
    // "run cc h" occurs literally only in the first row. The second row
    // contains every token — run, cc, h — and must still appear, ranked
    // below the literal phrase rather than being filtered out.
    let mut l = UnifiedList::new(
        vec![
            Row {
                kind: PickerMode::Commands,
                entry: entry("uv run cc homepages --help", 5.0, HOUR),
            },
            Row {
                kind: PickerMode::Commands,
                entry: entry("uv run cc fenix-homepages --out ./data/homepages.tsv", 5.0, HOUR),
            },
        ],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.filter = TypeFilter::Run;
    l.update_filter("run cc h");
    assert_eq!(
        shown(&l),
        vec![
            "uv run cc homepages --help".to_string(),
            "uv run cc fenix-homepages --out ./data/homepages.tsv".to_string(),
        ]
    );
}

#[test]
fn test_spaced_query_tokens_match_in_any_order() {
    let mut l = UnifiedList::new(
        vec![Row {
            kind: PickerMode::Commands,
            entry: entry("uv run cc homepages", 5.0, HOUR),
        }],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.filter = TypeFilter::Run;
    l.update_filter("homepages run");
    assert_eq!(shown(&l), vec!["uv run cc homepages".to_string()]);
}

#[test]
fn test_spaced_query_still_excludes_rows_missing_a_token() {
    let mut l = UnifiedList::new(
        vec![Row { kind: PickerMode::Commands, entry: entry("git commit", 5.0, HOUR) }],
        None,
        Transitions::new(),
        ScoringConfig::default(),
    );
    l.filter = TypeFilter::Run;
    l.update_filter("git status");
    assert!(shown(&l).is_empty(), "a row missing the `status` token must not match");
}
