use crate::config::ScoringConfig;
use crate::tui::PickerMode;

const DAY_SECS: i64 = 24 * 3600;
const WEEK_SECS: i64 = 7 * DAY_SECS;

/// How the query matched a row. `None` means no query is active.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchKind {
    None,
    /// Display text starts with the query. Implies Substring — both bonuses apply.
    Prefix,
    /// Query occurs as a contiguous substring.
    Substring,
    /// Query characters occur in order but not contiguously.
    Fuzzy,
}

/// Everything the scorer needs about one row. Built by the caller from a
/// `PickerEntry` plus per-session context (cwd, transitions).
pub struct RankInput<'a> {
    pub kind: PickerMode,
    pub display: &'a str,
    /// Already-decayed frecency score from the database.
    pub frecency: f64,
    /// Unix timestamp of last use/visit.
    pub last_time: i64,
    pub is_favorite: bool,
    /// True only for Commands rows whose recorded cwd equals the current directory.
    pub cwd_match: bool,
    /// Already-decayed transition score from the current directory to this row.
    /// Always 0.0 for Commands rows.
    pub transition_score: f64,
    pub match_kind: MatchKind,
    /// Raw nucleo match quality, 0 when no query is active.
    pub fuzzy_score: u32,
}

/// Score a row on every axis except the position-dependent type bonus,
/// which `interleave` applies afterwards.
pub fn base_score(input: &RankInput, cfg: &ScoringConfig, now: i64) -> i32 {
    let mut score: i32 = 0;

    match input.match_kind {
        MatchKind::None => {}
        // A prefix match is necessarily also a substring match; both apply.
        MatchKind::Prefix => score += cfg.prefix_bonus + cfg.substring_bonus,
        MatchKind::Substring => score += cfg.substring_bonus,
        MatchKind::Fuzzy => score += cfg.fuzzy_penalty,
    }
    score += input.fuzzy_score as i32;

    score += scale_capped(input.frecency, cfg.frecency_weight, cfg.frecency_cap);

    // Recency tiers are mutually exclusive. A clock skew that puts last_time in
    // the future is clamped to "just now" rather than wrapping negative.
    let age = (now - input.last_time).max(0);
    if age < DAY_SECS {
        score += cfg.recency_24h_bonus;
    } else if age < WEEK_SECS {
        score += cfg.recency_7d_bonus;
    }

    if input.cwd_match {
        score += cfg.cwd_bonus;
    }
    score += scale_capped(input.transition_score, cfg.transition_weight, cfg.transition_cap);

    // Char count, not byte length: a path with non-ASCII components should not
    // be penalized for its UTF-8 encoding.
    let len = input.display.chars().count() as i32;
    score += (cfg.brevity_bonus_max - len.saturating_mul(15)).max(0);

    if input.is_favorite {
        score += cfg.favorite_bonus;
    }

    score
}

fn scale_capped(value: f64, weight: f64, cap: i32) -> i32 {
    if value <= 0.0 {
        return 0;
    }
    (value * weight).min(cap as f64) as i32
}

use crate::config::TypeBonusConfig;

/// A row with its computed scores. `score` is only meaningful after `interleave`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scored {
    pub row_idx: usize,
    pub kind: PickerMode,
    /// Query-and-context score, position-independent.
    pub base: i32,
    /// `base` plus the position-dependent type bonus.
    pub score: i32,
}

/// The type bonus a row of `kind` receives when `emitted` rows of that same kind
/// already sit above it. Commands never receive a bonus. The schedule's last
/// element is the floor for all higher positions — without a floor, a long run of
/// one type would drive its bonus toward negative infinity and lock it out of the
/// rest of the list entirely.
pub fn type_bonus_at(kind: PickerMode, emitted: usize, cfg: &TypeBonusConfig) -> i32 {
    if matches!(kind, PickerMode::Commands) || cfg.schedule.is_empty() {
        return 0;
    }
    let idx = emitted.min(cfg.schedule.len() - 1);
    cfg.schedule[idx]
}

/// Fixed order used to break ties between types. Also indexes the per-kind lanes.
const KIND_ORDER: [PickerMode; 3] = [
    PickerMode::Directories,
    PickerMode::Commands,
    PickerMode::SshHosts,
];

fn lane_of(kind: PickerMode) -> usize {
    match kind {
        PickerMode::Directories => 0,
        PickerMode::Commands => 1,
        PickerMode::SshHosts => 2,
    }
}

/// Order rows by merit while decaying the bonus for `cd` and `ssh` as more rows of
/// that type are emitted, so the top of the list stays type-diverse.
///
/// Implemented as a three-way merge rather than repeated re-scoring: at any step
/// every remaining candidate of a given type receives the *same* bonus, so the best
/// candidate of that type is always its highest-`base` element and the order within
/// a type never changes. That makes this O(n log n) for the sorts plus O(n) for the
/// merge, instead of the O(n²) a naive greedy would cost on every keystroke.
pub fn interleave(scored: Vec<Scored>, cfg: &TypeBonusConfig) -> Vec<Scored> {
    let total = scored.len();
    let mut lanes: [Vec<Scored>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for item in scored {
        lanes[lane_of(item.kind)].push(item);
    }
    // Descending by base; ties by lower row_idx so the result is input-order independent.
    for lane in lanes.iter_mut() {
        lane.sort_by(|a, b| b.base.cmp(&a.base).then(a.row_idx.cmp(&b.row_idx)));
    }

    let mut heads = [0usize; 3];
    let mut counts = [0usize; 3];
    let mut out = Vec::with_capacity(total);

    for _ in 0..total {
        let mut best: Option<(usize, i32)> = None;
        for (lane_idx, kind) in KIND_ORDER.iter().enumerate() {
            let head = heads[lane_idx];
            let Some(candidate) = lanes[lane_idx].get(head) else {
                continue;
            };
            let adjusted = candidate.base + type_bonus_at(*kind, counts[lane_idx], cfg);
            // Strictly greater: KIND_ORDER iteration order breaks ties, so cd wins
            // over run, and run over ssh, at equal adjusted score.
            let better = match best {
                None => true,
                Some((_, best_score)) => adjusted > best_score,
            };
            if better {
                best = Some((lane_idx, adjusted));
            }
        }
        let Some((lane_idx, adjusted)) = best else {
            break;
        };
        let mut item = lanes[lane_idx][heads[lane_idx]];
        item.score = adjusted;
        out.push(item);
        heads[lane_idx] += 1;
        counts[lane_idx] += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScoringConfig;
    use crate::tui::PickerMode;

    const NOW: i64 = 1_700_000_000;
    const HOUR: i64 = 3600;
    const DAY: i64 = 24 * 3600;

    fn input(display: &str) -> RankInput<'_> {
        RankInput {
            kind: PickerMode::Commands,
            display,
            frecency: 0.0,
            last_time: 0,
            is_favorite: false,
            cwd_match: false,
            transition_score: 0.0,
            match_kind: MatchKind::None,
            fuzzy_score: 0,
        }
    }

    #[test]
    fn test_empty_query_scores_only_intrinsics() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = 0; // ancient
        // brevity only: 3000 - 3*15 = 2955
        assert_eq!(base_score(&i, &cfg, NOW), 2955);
    }

    #[test]
    fn test_prefix_and_substring_stack() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.match_kind = MatchKind::Prefix;
        // 10000 + 8000 + 2955
        assert_eq!(base_score(&i, &cfg, NOW), 20955);
    }

    #[test]
    fn test_substring_alone() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.match_kind = MatchKind::Substring;
        assert_eq!(base_score(&i, &cfg, NOW), 10955);
    }

    #[test]
    fn test_fuzzy_is_penalized() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.match_kind = MatchKind::Fuzzy;
        // -4000 + 2955
        assert_eq!(base_score(&i, &cfg, NOW), -1045);
    }

    #[test]
    fn test_fuzzy_score_is_added() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.match_kind = MatchKind::Substring;
        i.fuzzy_score = 137;
        assert_eq!(base_score(&i, &cfg, NOW), 10955 + 137);
    }

    #[test]
    fn test_recency_24h_tier() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = NOW - 2 * HOUR;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 6000);
    }

    #[test]
    fn test_recency_boundary_just_under_24h() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = NOW - DAY + 1;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 6000);
    }

    #[test]
    fn test_recency_boundary_exactly_24h_falls_to_7d_tier() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = NOW - DAY;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 2500);
    }

    #[test]
    fn test_recency_boundary_exactly_7d_is_stale() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = NOW - 7 * DAY;
        assert_eq!(base_score(&i, &cfg, NOW), 2955);
    }

    #[test]
    fn test_recency_tiers_do_not_stack() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = NOW - HOUR;
        let s = base_score(&i, &cfg, NOW);
        assert_eq!(s, 2955 + 6000, "24h tier only, never 6000+2500");
    }

    #[test]
    fn test_future_timestamp_treated_as_now() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.last_time = NOW + 10_000;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 6000);
    }

    #[test]
    fn test_frecency_scales_and_caps() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.frecency = 3.0;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 300);
        i.frecency = 9999.0;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 5000, "capped at frecency_cap");
    }

    #[test]
    fn test_cwd_match_bonus() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.cwd_match = true;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 4000);
    }

    #[test]
    fn test_transition_scales_and_caps() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.transition_score = 12.0;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 1200);
        i.transition_score = 500.0;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 4000, "capped at transition_cap");
    }

    #[test]
    fn test_favorite_bonus() {
        let cfg = ScoringConfig::default();
        let mut i = input("abc");
        i.is_favorite = true;
        assert_eq!(base_score(&i, &cfg, NOW), 2955 + 5000);
    }

    #[test]
    fn test_brevity_floors_at_zero_for_long_text() {
        let cfg = ScoringConfig::default();
        let long = "x".repeat(500);
        let i = input(&long);
        assert_eq!(base_score(&i, &cfg, NOW), 0, "brevity never goes negative");
    }

    #[test]
    fn test_brevity_counts_chars_not_bytes() {
        let cfg = ScoringConfig::default();
        // 3 chars, 9 bytes in UTF-8
        let i = input("日本語");
        assert_eq!(base_score(&i, &cfg, NOW), 2955);
    }

    #[test]
    fn test_score_can_go_negative() {
        let cfg = ScoringConfig::default();
        let long = "x".repeat(500);
        let mut i = input(&long);
        i.match_kind = MatchKind::Fuzzy;
        assert_eq!(base_score(&i, &cfg, NOW), -4000);
    }

    #[test]
    fn test_db_kind_strings() {
        assert_eq!(PickerMode::Directories.db_kind(), "cd");
        assert_eq!(PickerMode::Commands.db_kind(), "run");
        assert_eq!(PickerMode::SshHosts.db_kind(), "ssh");
    }

    fn tb(schedule: &[i32]) -> crate::config::TypeBonusConfig {
        crate::config::TypeBonusConfig { schedule: schedule.to_vec() }
    }

    fn s(row_idx: usize, kind: PickerMode, base: i32) -> Scored {
        Scored { row_idx, kind, base, score: 0 }
    }

    fn kinds(out: &[Scored]) -> Vec<&'static str> {
        out.iter().map(|x| x.kind.db_kind()).collect()
    }

    #[test]
    fn test_type_bonus_schedule_positions() {
        let c = crate::config::TypeBonusConfig::default();
        assert_eq!(type_bonus_at(PickerMode::Directories, 0, &c), 3000);
        assert_eq!(type_bonus_at(PickerMode::Directories, 1, &c), 1500);
        assert_eq!(type_bonus_at(PickerMode::Directories, 2, &c), 0);
        assert_eq!(type_bonus_at(PickerMode::Directories, 3, &c), -1500);
        assert_eq!(type_bonus_at(PickerMode::Directories, 4, &c), -3000);
    }

    #[test]
    fn test_type_bonus_clamps_to_floor() {
        let c = crate::config::TypeBonusConfig::default();
        assert_eq!(type_bonus_at(PickerMode::SshHosts, 5, &c), -3000);
        assert_eq!(type_bonus_at(PickerMode::SshHosts, 999, &c), -3000);
    }

    #[test]
    fn test_run_rows_never_get_a_type_bonus() {
        let c = crate::config::TypeBonusConfig::default();
        for n in 0..10 {
            assert_eq!(type_bonus_at(PickerMode::Commands, n, &c), 0);
        }
    }

    #[test]
    fn test_empty_schedule_yields_zero() {
        let c = tb(&[]);
        assert_eq!(type_bonus_at(PickerMode::Directories, 0, &c), 0);
        assert_eq!(type_bonus_at(PickerMode::Directories, 7, &c), 0);
    }

    #[test]
    fn test_cd_and_ssh_counters_are_independent() {
        // Three cd rows must not consume the ssh budget: the first ssh row
        // still receives the full +3000 no matter how many cd rows precede it.
        let c = crate::config::TypeBonusConfig::default();
        let out = interleave(
            vec![
                s(0, PickerMode::Directories, 10_000),
                s(1, PickerMode::Directories, 9_900),
                s(2, PickerMode::Directories, 9_800),
                s(3, PickerMode::SshHosts, 9_000),
            ],
            &c,
        );
        let ssh = out.iter().find(|x| x.kind == PickerMode::SshHosts).unwrap();
        assert_eq!(ssh.score, 9_000 + 3000, "first ssh row gets the full bonus");
    }

    #[test]
    fn test_interleave_breaks_up_a_run_of_one_type() {
        // Five cd rows tightly clustered in merit, one run row well below them.
        // The decaying cd bonus must let the run row surface before the tail.
        let c = crate::config::TypeBonusConfig::default();
        let out = interleave(
            vec![
                s(0, PickerMode::Directories, 10_000),
                s(1, PickerMode::Directories, 9_990),
                s(2, PickerMode::Directories, 9_980),
                s(3, PickerMode::Directories, 9_970),
                s(4, PickerMode::Directories, 9_960),
                s(5, PickerMode::Commands, 9_500),
            ],
            &c,
        );
        let run_pos = out.iter().position(|x| x.kind == PickerMode::Commands).unwrap();
        assert!(run_pos < 5, "run row must not be pinned to last, got position {}", run_pos);
    }

    #[test]
    fn test_floor_prevents_type_lockout() {
        // A high-merit cd row must still appear even after a long wall of run rows.
        let c = crate::config::TypeBonusConfig::default();
        let mut rows: Vec<Scored> = (0..50)
            .map(|i| s(i, PickerMode::Commands, 20_000 - i as i32))
            .collect();
        rows.push(s(100, PickerMode::Directories, 15_000));
        let out = interleave(rows, &c);
        let cd_pos = out.iter().position(|x| x.kind == PickerMode::Directories).unwrap();
        assert!(cd_pos < 51, "cd row must appear somewhere, got {}", cd_pos);
        assert_eq!(out.len(), 51);
    }

    #[test]
    fn test_relative_order_within_a_type_is_preserved() {
        let c = crate::config::TypeBonusConfig::default();
        let out = interleave(
            vec![
                s(0, PickerMode::Directories, 100),
                s(1, PickerMode::Directories, 300),
                s(2, PickerMode::Directories, 200),
            ],
            &c,
        );
        let bases: Vec<i32> = out.iter().map(|x| x.base).collect();
        assert_eq!(bases, vec![300, 200, 100], "within a type, higher base always first");
    }

    #[test]
    fn test_score_field_is_base_plus_bonus() {
        let c = crate::config::TypeBonusConfig::default();
        let out = interleave(vec![s(0, PickerMode::Directories, 500)], &c);
        assert_eq!(out[0].base, 500);
        assert_eq!(out[0].score, 3500);
    }

    #[test]
    fn test_ties_are_deterministic() {
        let c = crate::config::TypeBonusConfig::default();
        let rows = vec![
            s(7, PickerMode::Commands, 1000),
            s(3, PickerMode::Commands, 1000),
        ];
        let a = interleave(rows, &c);
        let b = interleave(
            vec![
                s(3, PickerMode::Commands, 1000),
                s(7, PickerMode::Commands, 1000),
            ],
            &c,
        );
        assert_eq!(
            a.iter().map(|x| x.row_idx).collect::<Vec<_>>(),
            b.iter().map(|x| x.row_idx).collect::<Vec<_>>(),
            "input order must not affect output"
        );
        assert_eq!(a[0].row_idx, 3, "lower row_idx wins a tie");
    }

    #[test]
    fn test_kind_order_breaks_cross_type_ties() {
        let c = tb(&[0]); // no type bonus, so bases tie exactly
        let out = interleave(
            vec![
                s(0, PickerMode::SshHosts, 1000),
                s(1, PickerMode::Commands, 1000),
                s(2, PickerMode::Directories, 1000),
            ],
            &c,
        );
        assert_eq!(kinds(&out), vec!["cd", "run", "ssh"]);
    }

    #[test]
    fn test_empty_input() {
        let c = crate::config::TypeBonusConfig::default();
        assert!(interleave(vec![], &c).is_empty());
    }

    #[test]
    fn test_all_rows_are_emitted_exactly_once() {
        let c = crate::config::TypeBonusConfig::default();
        let rows: Vec<Scored> = (0..30)
            .map(|i| {
                let kind = match i % 3 {
                    0 => PickerMode::Directories,
                    1 => PickerMode::Commands,
                    _ => PickerMode::SshHosts,
                };
                s(i, kind, (i as i32) * 7 % 101)
            })
            .collect();
        let out = interleave(rows, &c);
        assert_eq!(out.len(), 30);
        let mut idxs: Vec<usize> = out.iter().map(|x| x.row_idx).collect();
        idxs.sort_unstable();
        assert_eq!(idxs, (0..30).collect::<Vec<_>>());
    }

    #[test]
    fn test_single_type_input_is_plain_sort() {
        let c = crate::config::TypeBonusConfig::default();
        let out = interleave(
            vec![
                s(0, PickerMode::Commands, 10),
                s(1, PickerMode::Commands, 30),
                s(2, PickerMode::Commands, 20),
            ],
            &c,
        );
        assert_eq!(out.iter().map(|x| x.base).collect::<Vec<_>>(), vec![30, 20, 10]);
    }
}
