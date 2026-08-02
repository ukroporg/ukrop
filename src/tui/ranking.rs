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
}
