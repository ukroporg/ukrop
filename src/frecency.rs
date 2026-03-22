/// Half-life in seconds (1 week)
const HALF_LIFE: f64 = 7.0 * 24.0 * 3600.0;

/// Threshold for aging all scores
pub const AGE_THRESHOLD: f64 = 10_000.0;

/// Decay a score from `last_time` to `now` using exponential decay with a 1-week half-life.
pub fn decay(score: f64, last_time: i64, now: i64) -> f64 {
    let elapsed = (now - last_time).max(0) as f64;
    score * (0.5_f64).powf(elapsed / HALF_LIFE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_decay_at_same_time() {
        let score = decay(100.0, 1000, 1000);
        assert!((score - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_half_after_one_week() {
        let week = 7 * 24 * 3600;
        let score = decay(100.0, 0, week);
        assert!((score - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_quarter_after_two_weeks() {
        let two_weeks = 14 * 24 * 3600;
        let score = decay(100.0, 0, two_weeks);
        assert!((score - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_negative_elapsed_no_growth() {
        let score = decay(100.0, 1000, 500);
        assert!((score - 100.0).abs() < 0.001);
    }
}
