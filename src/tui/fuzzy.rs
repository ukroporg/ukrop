use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

/// Rewards matches whose characters land in few, long, contiguous runs.
///
/// `sum(run_len^2) - total_matched_len` over the runs of consecutive positions.
/// Fully scattered matches score 0, so this only ever lifts a row; a run of
/// length `n` contributes `n^2 - n`, i.e. quadratic in how much of the query
/// stayed together. `positions` must be sorted ascending.
///
/// For query `seo2`: `seo|2` scores `3^2 + 1^2 - 4 = 6`, while `s|e|o|2`
/// scores `1+1+1+1 - 4 = 0`.
pub fn contiguity_score(positions: &[u32]) -> u32 {
    let mut total = 0u32;
    let mut run = 0u32;
    let mut prev: Option<u32> = None;
    for &p in positions {
        run = match prev {
            Some(q) if p == q + 1 => run + 1,
            _ => 1,
        };
        // Extending a run from n-1 to n adds (n^2 - (n-1)^2) = 2n - 1.
        total += 2 * run - 1;
        prev = Some(p);
    }
    total.saturating_sub(positions.len() as u32)
}

/// One row that matched the query.
pub struct FuzzyMatch {
    /// Index into the `items` slice passed to `filter`.
    pub idx: usize,
    /// Raw nucleo match quality.
    pub score: u32,
    /// True when the query occurs as a literal substring.
    pub is_substring: bool,
    /// `contiguity_score` of the matched positions. Always 0 for substring
    /// matches — those are a single run by definition, so the value would be
    /// the same constant for every one of them and could not order them.
    pub contiguity: u32,
}

pub struct FuzzyMatcher {
    matcher: Matcher,
}

impl FuzzyMatcher {
    pub fn new() -> Self {
        FuzzyMatcher {
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
        }
    }

    /// Returns items that match the query, sorted by match quality.
    /// If query is empty, returns all indices in original order.
    pub fn filter(&mut self, query: &str, items: &[String]) -> Vec<FuzzyMatch> {
        if query.is_empty() {
            return items
                .iter()
                .enumerate()
                .map(|(i, _)| FuzzyMatch { idx: i, score: 0, is_substring: false, contiguity: 0 })
                .collect();
        }

        // Two-tier matching: try substring first, fall back to fuzzy.
        // Substring matches are tagged so they can receive a ranking bonus.
        //
        // The two tiers treat whitespace differently, and that difference is
        // the point. `Atom` keeps the query literal, so tier 1 matches the
        // whole phrase including its spaces; `Pattern` splits on whitespace,
        // so tier 2 requires every token but lets them land anywhere, in any
        // order. A query with no spaces is simply the case where `Pattern`
        // yields a single atom, which is why one code path serves both.
        let substring_atom = Atom::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Substring,
            false,
        );
        let fuzzy_pattern = Pattern::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        // Scratch buffer reused across rows: `indices` runs for every
        // fuzzy-tier row on every keystroke.
        let mut indices: Vec<u32> = Vec::new();
        let mut buf: Vec<char> = Vec::new();
        let mut results: Vec<FuzzyMatch> = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let haystack = nucleo_matcher::Utf32Str::new(item, &mut buf);
                // Try substring first
                if let Some(score) = substring_atom.score(haystack, &mut self.matcher) {
                    return Some(FuzzyMatch {
                        idx,
                        score: score as u32,
                        is_substring: true,
                        contiguity: 0,
                    });
                }
                // Fall back to fuzzy. `indices` yields the same score as
                // `score` while also reporting where the characters landed,
                // so this stays a single pass over the haystack.
                indices.clear();
                let score = fuzzy_pattern.indices(haystack, &mut self.matcher, &mut indices)?;
                indices.sort_unstable();
                indices.dedup();
                Some(FuzzyMatch {
                    idx,
                    score,
                    is_substring: false,
                    contiguity: contiguity_score(&indices),
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// Returns the matched character positions for a single item against the query.
    pub fn match_positions(&mut self, query: &str, item: &str) -> Vec<u32> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut buf = Vec::new();
        let haystack = nucleo_matcher::Utf32Str::new(item, &mut buf);
        let mut indices = Vec::new();

        // Mirrors the two tiers in `filter` exactly — highlighting a row by a
        // different rule than the one that matched it would underline the
        // wrong characters.
        let substring_atom = Atom::new(
            query,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Substring,
            false,
        );
        if substring_atom.indices(haystack, &mut self.matcher, &mut indices).is_none() {
            let pattern = Pattern::new(
                query,
                CaseMatching::Smart,
                Normalization::Smart,
                AtomKind::Fuzzy,
            );
            pattern.indices(haystack, &mut self.matcher, &mut indices);
        }

        indices.sort_unstable();
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query_returns_all() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        let results = matcher.filter("", &items);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].idx, 0);
        assert_eq!(results[1].idx, 1);
        assert_eq!(results[2].idx, 2);
    }

    #[test]
    fn test_substring_match() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["foobar".to_string(), "baz".to_string()];
        let results = matcher.filter("foo", &items);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].idx, 0);
    }

    #[test]
    fn test_fuzzy_match() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["foobar".to_string(), "xyz".to_string()];
        let results = matcher.filter("fb", &items);
        assert!(results.iter().any(|m| m.idx == 0));
        assert!(!results.iter().any(|m| m.idx == 1));
    }

    #[test]
    fn test_no_match() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["foobar".to_string()];
        let results = matcher.filter("xyz", &items);
        assert!(results.is_empty());
    }

    #[test]
    fn test_space_in_query() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["git status check".to_string(), "git commit".to_string()];
        let results = matcher.filter("git status", &items);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].idx, 0);
    }

    #[test]
    fn test_match_positions_substring() {
        let mut matcher = FuzzyMatcher::new();
        let positions = matcher.match_positions("foo", "foobar");
        assert!(!positions.is_empty());
        // "foo" matches at positions 0, 1, 2
        assert!(positions.contains(&0));
        assert!(positions.contains(&1));
        assert!(positions.contains(&2));
    }

    #[test]
    fn test_match_positions_empty() {
        let mut matcher = FuzzyMatcher::new();
        let positions = matcher.match_positions("", "foobar");
        assert!(positions.is_empty());
    }

    #[test]
    fn test_non_ascii_rows_do_not_contaminate_each_other() {
        // `filter` reuses one `Vec<char>` scratch buffer across rows, and that
        // buffer is only touched for non-ASCII haystacks. A long row followed
        // by a shorter one is the case that would expose a stale tail if the
        // buffer were not reset between rows.
        let mut matcher = FuzzyMatcher::new();
        let items = vec![
            "cd ~/Документы/проекты/архив".to_string(),
            "cd ~/café".to_string(),
            "cd ~/plain-ascii".to_string(),
        ];
        let hits: Vec<usize> = matcher.filter("café", &items).iter().map(|m| m.idx).collect();
        assert_eq!(hits, vec![1], "only the café row may match");

        // Same matcher, second query: the long Cyrillic row must still match
        // on its own terms after the shorter row reused the buffer.
        let hits: Vec<usize> = matcher.filter("архив", &items).iter().map(|m| m.idx).collect();
        assert_eq!(hits, vec![0], "only the Cyrillic row may match");
    }

    #[test]
    fn test_spaced_query_falls_back_to_token_matching() {
        let mut matcher = FuzzyMatcher::new();
        // "run cc h" appears literally in row 1 only. Row 0 still contains
        // every token — run, cc, h — so it must survive in the fuzzy tier
        // rather than being filtered out entirely.
        let items = vec![
            "uv run cc fenix-homepages --out ./data/fenix/homepages.tsv".to_string(),
            "uv run cc homepages --help".to_string(),
        ];
        let results = matcher.filter("run cc h", &items);
        assert_eq!(results.len(), 2, "both rows should match");

        let literal = results.iter().find(|m| m.idx == 1).expect("literal phrase row");
        let tokens = results.iter().find(|m| m.idx == 0).expect("token row");
        assert!(literal.is_substring, "exact phrase stays in the substring tier");
        assert!(!tokens.is_substring, "token-only match lands in the fuzzy tier");
    }

    #[test]
    fn test_spaced_token_match_is_order_independent() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["uv run cc homepages".to_string()];
        assert_eq!(matcher.filter("homepages run", &items).len(), 1);
    }

    #[test]
    fn test_spaced_query_still_requires_every_token() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["git commit".to_string()];
        // "status" appears nowhere, so no tier may match.
        assert!(matcher.filter("git status", &items).is_empty());
    }

    #[test]
    fn test_match_positions_for_a_spaced_token_match() {
        let mut matcher = FuzzyMatcher::new();
        // No literal "run cc h" here, so highlighting must come from the
        // token tier rather than returning nothing.
        let positions = matcher.match_positions("run cc h", "uv run cc fenix-homepages");
        assert!(!positions.is_empty(), "token matches must still highlight");
    }

    #[test]
    fn test_filter_reports_higher_contiguity_for_the_clustered_match() {
        let mut matcher = FuzzyMatcher::new();
        // Neither row contains "seo2" literally, so both land in the fuzzy tier.
        let items = vec![
            // s..e..o..2 scattered across the whole line
            "uv run cc majestic --output-file domains2.tsv".to_string(),
            // "seo" stays together, then a lone "2"
            "gcx login seo --org-id 2".to_string(),
        ];
        let results = matcher.filter("seo2", &items);

        let scattered = results.iter().find(|m| m.idx == 0).expect("row 0 should match");
        let clustered = results.iter().find(|m| m.idx == 1).expect("row 1 should match");
        assert!(!scattered.is_substring && !clustered.is_substring);
        assert!(
            clustered.contiguity > scattered.contiguity,
            "clustered {} should beat scattered {}",
            clustered.contiguity,
            scattered.contiguity
        );
    }

    #[test]
    fn test_filter_reports_zero_contiguity_for_substring_matches() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["ssh seo2".to_string()];
        let results = matcher.filter("seo2", &items);
        assert!(results[0].is_substring);
        assert_eq!(results[0].contiguity, 0);
    }

    #[test]
    fn test_contiguity_of_no_match_is_zero() {
        assert_eq!(contiguity_score(&[]), 0);
    }

    #[test]
    fn test_contiguity_of_fully_scattered_match_is_zero() {
        // s|e|o|2 — four isolated characters: 1+1+1+1 - 4 = 0
        assert_eq!(contiguity_score(&[1, 5, 9, 14]), 0);
    }

    #[test]
    fn test_contiguity_rewards_one_long_run_over_scattered() {
        // seo|2 — a run of 3 plus a lone char: 3^2 + 1^2 - 4 = 6
        assert_eq!(contiguity_score(&[3, 4, 5, 9]), 6);
    }

    #[test]
    fn test_contiguity_of_fully_contiguous_match() {
        // seo2 — a single run of 4: 4^2 - 4 = 12
        assert_eq!(contiguity_score(&[0, 1, 2, 3]), 12);
    }

    #[test]
    fn test_contiguity_grows_quadratically_with_run_length() {
        // Two runs of 2 (2^2+2^2-4 = 4) rank below one run of 4 (12),
        // even though both matched the same number of characters.
        assert!(contiguity_score(&[0, 1, 7, 8]) < contiguity_score(&[0, 1, 2, 3]));
        assert_eq!(contiguity_score(&[0, 1, 7, 8]), 4);
    }

    #[test]
    fn test_contiguity_of_single_character_match_is_zero() {
        assert_eq!(contiguity_score(&[6]), 0);
    }
}
