use ukrop::frecency;

#[test]
fn test_decay_function() {
    let week = 7 * 24 * 3600;

    assert!((frecency::decay(100.0, 0, 0) - 100.0).abs() < 0.001);
    assert!((frecency::decay(100.0, 0, week) - 50.0).abs() < 0.5);
    assert!((frecency::decay(100.0, 0, 2 * week) - 25.0).abs() < 0.5);
}

#[test]
fn test_age_threshold() {
    assert!(frecency::AGE_THRESHOLD > 0.0);
}
