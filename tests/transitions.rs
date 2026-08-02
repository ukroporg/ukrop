use std::process::Command;
use ukrop::db::store::Store;

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(dir.path().join("t.db").to_str().unwrap()).unwrap()
}

fn ukrop_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ukrop"))
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

#[test]
fn test_shell_pwd_change_records_a_cd_transition() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_prompt_pwd(&mut s, "/a", Some("42")).unwrap();
    ukrop::record_prompt_pwd(&mut s, "/b", Some("42")).unwrap();
    let map = s.transitions_from("/a").unwrap();
    assert!(map.contains_key(&("cd".to_string(), "/b".to_string())));
}

#[test]
fn test_same_pwd_twice_records_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_prompt_pwd(&mut s, "/a", Some("42")).unwrap();
    ukrop::record_prompt_pwd(&mut s, "/a", Some("42")).unwrap();
    assert!(s.transitions_from("/a").unwrap().is_empty());
}

#[test]
fn test_concurrent_shells_do_not_cross_contaminate() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    // Two tabs sitting in different directories, alternating prompts.
    ukrop::record_prompt_pwd(&mut s, "/tab1", Some("1")).unwrap();
    ukrop::record_prompt_pwd(&mut s, "/tab2", Some("2")).unwrap();
    ukrop::record_prompt_pwd(&mut s, "/tab1", Some("1")).unwrap();
    ukrop::record_prompt_pwd(&mut s, "/tab2", Some("2")).unwrap();
    assert!(
        s.transitions_from("/tab1").unwrap().is_empty(),
        "no fabricated transition between unrelated shells"
    );
    assert!(s.transitions_from("/tab2").unwrap().is_empty());
}

#[test]
fn test_missing_shell_id_records_nothing_but_does_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(&dir);
    ukrop::record_prompt_pwd(&mut s, "/a", None).unwrap();
    ukrop::record_prompt_pwd(&mut s, "/b", None).unwrap();
    assert!(s.transitions_from("/a").unwrap().is_empty());
}

// Review finding 1 (Task 8): a manually-typed `ssh` must record its
// transition against the alias `record_ssh_from_command` resolved to
// (`ssh_hosts.host`, the same key picker rows key on) — not against the raw
// connect string the user typed, which the picker can never match.
#[test]
fn test_manual_ssh_transition_uses_resolved_alias_not_connect_string() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // Seed an ssh_hosts row whose alias differs from its connect string.
    {
        let mut s = Store::open(db_str).unwrap();
        s.record_ssh_host("prod", Some("203.0.113.5"), None, Some("deploy"), "config")
            .unwrap();
    }

    // Simulate a manually-typed `ssh deploy@203.0.113.5` via the real
    // hook-cmd dispatch path (cmd_hook_cmd's SSH detection).
    let output = ukrop_bin()
        .args([
            "hook-cmd",
            "--cmd",
            "ssh deploy@203.0.113.5",
            "--exit-code",
            "0",
            "--cwd",
            "/home/x",
        ])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    let s = Store::open(db_str).unwrap();
    let map = s.transitions_from("/home/x").unwrap();
    assert!(
        map.contains_key(&("ssh".to_string(), "prod".to_string())),
        "transition must be keyed on the resolved alias 'prod', got: {:?}",
        map.keys().collect::<Vec<_>>()
    );
    assert!(
        !map.contains_key(&("ssh".to_string(), "deploy@203.0.113.5".to_string())),
        "transition must not be keyed on the raw connect string"
    );
}

// Review finding 2 (Task 8): a single picker ssh pick must record exactly
// one transition, not two. `run_tui_inner` already records it synchronously
// via `record_pick_transition`; the shell's `hook-ssh` call for that same
// pick must NOT also pass --cwd (the init scripts no longer do, see
// tests/cli_test.rs::test_init_scripts_pass_new_hook_flags).
#[test]
fn test_picker_ssh_pick_is_not_double_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    // What run_tui_inner does synchronously on a picker selection.
    {
        let mut s = Store::open(db_str).unwrap();
        ukrop::record_pick_transition(&mut s, Some("/home/x"), "ssh", "prod").unwrap();
    }

    // What the (fixed) shell wrapper does for that same pick: hook-ssh with
    // no --cwd, so it must not record a second transition.
    let output = ukrop_bin()
        .args(["hook-ssh", "--host", "prod"])
        .env("UKROP_DB_PATH", db_str)
        .output()
        .unwrap();
    assert!(output.status.success());

    let s = Store::open(db_str).unwrap();
    let map = s.transitions_from("/home/x").unwrap();
    let score = *map
        .get(&("ssh".to_string(), "prod".to_string()))
        .expect("transition should exist");
    assert!(
        score < 1.5,
        "expected a single record (~1.0), got {} — transition was double-recorded",
        score
    );
}
