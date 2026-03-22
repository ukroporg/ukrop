use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

pub struct FuzzyMatcher {
    matcher: Matcher,
}

impl FuzzyMatcher {
    pub fn new() -> Self {
        FuzzyMatcher {
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
        }
    }

    /// Returns indices of items that match the query, sorted by match quality.
    /// If query is empty, returns all indices in original order.
    /// The returned tuple is (index, score, is_substring_match).
    pub fn filter(&mut self, query: &str, items: &[String]) -> Vec<(usize, u32, bool)> {
        if query.is_empty() {
            return items.iter().enumerate().map(|(i, _)| (i, 0, false)).collect();
        }

        let has_space = query.contains(' ');

        let mut results: Vec<(usize, u32, bool)> = if has_space {
            // When query contains spaces, use Atom directly to preserve
            // the literal query (Pattern splits on whitespace).
            let atom = Atom::new(
                query,
                CaseMatching::Smart,
                Normalization::Smart,
                AtomKind::Substring,
                false,
            );
            items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    let mut buf = Vec::new();
                    let score = atom.score(
                        nucleo_matcher::Utf32Str::new(item, &mut buf),
                        &mut self.matcher,
                    )?;
                    Some((idx, score as u32, true))
                })
                .collect()
        } else {
            // Two-tier matching: try substring first, fall back to fuzzy.
            // Substring matches are tagged so they can receive a ranking bonus.
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
            items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    let mut buf = Vec::new();
                    let haystack = nucleo_matcher::Utf32Str::new(item, &mut buf);
                    // Try substring first
                    if let Some(score) = substring_atom.score(haystack, &mut self.matcher) {
                        return Some((idx, score as u32, true));
                    }
                    // Fall back to fuzzy
                    let score = fuzzy_pattern.score(haystack, &mut self.matcher)?;
                    Some((idx, score, false))
                })
                .collect()
        };

        results.sort_by(|a, b| b.1.cmp(&a.1));
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

        let has_space = query.contains(' ');
        if has_space {
            let atom = Atom::new(
                query,
                CaseMatching::Smart,
                Normalization::Smart,
                AtomKind::Substring,
                false,
            );
            atom.indices(haystack, &mut self.matcher, &mut indices);
        } else {
            // Try substring first, fall back to fuzzy
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
        assert_eq!(results[0].0, 0);
        assert_eq!(results[1].0, 1);
        assert_eq!(results[2].0, 2);
    }

    #[test]
    fn test_substring_match() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["foobar".to_string(), "baz".to_string()];
        let results = matcher.filter("foo", &items);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0);
    }

    #[test]
    fn test_fuzzy_match() {
        let mut matcher = FuzzyMatcher::new();
        let items = vec!["foobar".to_string(), "xyz".to_string()];
        let results = matcher.filter("fb", &items);
        assert!(results.iter().any(|(i, _, _)| *i == 0));
        assert!(!results.iter().any(|(i, _, _)| *i == 1));
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
        assert_eq!(results[0].0, 0);
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
}
