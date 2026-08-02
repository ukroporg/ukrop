use std::collections::HashMap;
use ukrop::config::ScoringConfig;
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
        HashMap::new(),
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
            Row { kind: PickerMode::Commands, entry: entry("carcass", 1.0, HOUR) },
        ],
        None,
        HashMap::new(),
        ScoringConfig::default(),
    );
    l.update_filter("car");
    // "carcass" contains "car"; "clear" only fuzzy-matches c-a-r.
    assert_eq!(shown(&l)[0], "carcass");
}

// Pins the fuzzy/substring boundary from the approved spec's worked example
// (.claude/superpowers/specs/2026-08-02-unified-picker-design.md:221): a fuzzy
// match with favorite + cwd + 24h-recency stacked on top can outscore a stale,
// SUBSTRING-only match (fuzzy_penalty -4,000 vs. substring_bonus +8,000 is a
// 12,000 gap, crossable by 15,000 of stackable merit).
//
// The stale fixture must NOT start with the query ("car"), or it becomes a
// PREFIX match instead of a substring match, which is a different, much wider
// gap that this test is not meant to probe (see
// test_fuzzy_penalty_is_not_crossable_against_a_prefix_match below).
#[test]
fn test_fuzzy_penalty_is_crossable_by_overwhelming_merit() {
    let mut fresh = entry("clear", 50.0, 60);
    fresh.cwd = Some("/here".to_string());
    fresh.is_favorite = true;
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Commands, entry: fresh },
            Row { kind: PickerMode::Commands, entry: entry("~/old/carcass-utility", 0.02, 200 * DAY) },
        ],
        Some("/here".to_string()),
        HashMap::new(),
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
// penalty must NOT be crossable against it even by favorite + cwd + 24h
// recency stacked together (fuzzy_penalty -4,000 vs. prefix_bonus +
// substring_bonus +18,000 is a 22,000 gap, wider than the 15,000 of
// stackable merit that closed the substring-only gap above).
//
// Measured base scores for this exact fixture (ScoringConfig::default()):
//   "clear" (fuzzy, favorite, cwd_match, 60s old):            base = 18,991
//   "carcass-utility-script" (prefix+substring, 200d stale):  base = 20,756
// The stale prefix match wins by 1,765.
#[test]
fn test_fuzzy_penalty_is_not_crossable_against_a_prefix_match() {
    let mut fresh = entry("clear", 50.0, 60);
    fresh.cwd = Some("/here".to_string());
    fresh.is_favorite = true;
    let mut l = UnifiedList::new(
        vec![
            Row { kind: PickerMode::Commands, entry: fresh },
            Row { kind: PickerMode::Commands, entry: entry("carcass-utility-script", 0.02, 200 * DAY) },
        ],
        Some("/here".to_string()),
        HashMap::new(),
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
    let mut t = HashMap::new();
    t.insert(("cd".to_string(), "/proj/api".to_string()), 25.0);
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
    let mut l = UnifiedList::new(rows, None, HashMap::new(), ScoringConfig::default());
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
        HashMap::new(),
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
    let mut l = UnifiedList::new(rows, None, HashMap::new(), ScoringConfig::default());
    l.update_filter("");
    let pos = shown(&l)
        .iter()
        .position(|s| s == "rarely-used-but-starred")
        .unwrap();
    assert!(pos < 3, "a favorite must stay near the top, got position {}", pos);
}
