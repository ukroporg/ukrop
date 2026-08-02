use ukrop::db::store::Store;

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(dir.path().join("t.db").to_str().unwrap()).unwrap()
}

#[test]
fn test_selecting_a_directory_records_a_transition() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_pick_transition(&mut s, Some("/from/here"), "cd", "/to/there").unwrap();
    let map = s.transitions_from("/from/here").unwrap();
    assert!(map.contains_key(&("cd".to_string(), "/to/there".to_string())));
}

#[test]
fn test_selecting_an_ssh_host_records_a_transition() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_pick_transition(&mut s, Some("/from/here"), "ssh", "prod-web1").unwrap();
    let map = s.transitions_from("/from/here").unwrap();
    assert!(map.contains_key(&("ssh".to_string(), "prod-web1".to_string())));
}

#[test]
fn test_run_selections_are_not_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_pick_transition(&mut s, Some("/from/here"), "run", "cargo build").unwrap();
    assert!(s.transitions_from("/from/here").unwrap().is_empty());
}

#[test]
fn test_unknown_cwd_is_a_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_pick_transition(&mut s, None, "cd", "/to/there").unwrap();
    assert!(s.transitions_from("/to/there").unwrap().is_empty());
}

#[test]
fn test_self_transition_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_pick_transition(&mut s, Some("/same"), "cd", "/same").unwrap();
    assert!(s.transitions_from("/same").unwrap().is_empty());
}
