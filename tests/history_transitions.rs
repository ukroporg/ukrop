use ukrop::history;

fn pairs() -> Vec<(String, Option<String>)> {
    history::bash::parse_history_file_pairs_raw("tests/fixtures/transitions_history.txt").unwrap()
}

#[test]
fn test_cd_command_is_attributed_to_its_origin_directory() {
    let p = pairs();
    let (cmd, cwd) = p
        .iter()
        .find(|(c, _)| c == "cd /home/me/projects/api")
        .unwrap();
    assert_eq!(cmd, "cd /home/me/projects/api");
    assert_eq!(
        cwd.as_deref(),
        Some("/home/me/projects"),
        "a cd command ran in the directory it left, not the one it entered"
    );
}

#[test]
fn test_extracts_cd_transitions() {
    let t = history::extract_transitions(&pairs());
    assert!(t.contains(&(
        "/home/me/projects".to_string(),
        "cd".to_string(),
        "/home/me/projects/api".to_string()
    )));
}

#[test]
fn test_extracts_ssh_transitions_with_originating_dir() {
    let t = history::extract_transitions(&pairs());
    assert!(t.contains(&(
        "/home/me/projects/api".to_string(),
        "ssh".to_string(),
        "prod-web1".to_string()
    )));
    assert!(t.contains(&(
        "/home/me/projects".to_string(),
        "ssh".to_string(),
        "prod-db".to_string()
    )));
}

/// The fixture's very first line is `cd /home/me/projects`, run from an unknown
/// directory: the parser reports `cwd: None` for it, so no transition can be
/// attributed. Assert on the concrete transition set rather than on
/// `!from.is_empty()` — an unknown origin is a `None`, never an empty string,
/// so that predicate is vacuously true and tests nothing.
#[test]
fn test_first_cd_has_no_origin_and_is_skipped() {
    let t = history::extract_transitions(&pairs());

    assert_eq!(
        t,
        vec![
            (
                "/home/me/projects".to_string(),
                "cd".to_string(),
                "/home/me/projects/api".to_string()
            ),
            (
                "/home/me/projects/api".to_string(),
                "ssh".to_string(),
                "prod-web1".to_string()
            ),
            (
                "/home/me/projects/api".to_string(),
                "cd".to_string(),
                "/home/me/projects".to_string()
            ),
            (
                "/home/me/projects".to_string(),
                "ssh".to_string(),
                "prod-db".to_string()
            ),
            (
                "/home/me/projects".to_string(),
                "cd".to_string(),
                "/home/me/projects/api".to_string()
            ),
        ],
        "9 history lines must yield exactly these 5 transitions"
    );

    // The only `cd` into /home/me/projects is the one on line 6 (from .../api).
    // If the origin-less first line were recorded there would be a second.
    let into_projects: Vec<&String> = t
        .iter()
        .filter(|(_, kind, target)| kind == "cd" && target == "/home/me/projects")
        .map(|(from, _, _)| from)
        .collect();
    assert_eq!(
        into_projects,
        vec!["/home/me/projects/api"],
        "the origin-less first `cd /home/me/projects` must contribute no transition"
    );
}

#[test]
fn test_unresolvable_relative_cd_is_skipped() {
    let t = history::extract_transitions(&pairs());
    assert!(
        !t.iter().any(|(_, _, target)| target == "relative-dir"),
        "relative cd targets cannot be resolved and must be skipped"
    );
}

#[test]
fn test_repeated_route_appears_once_per_occurrence() {
    let t = history::extract_transitions(&pairs());
    let n = t
        .iter()
        .filter(|(f, k, tgt)| {
            f == "/home/me/projects" && k == "cd" && tgt == "/home/me/projects/api"
        })
        .count();
    assert_eq!(n, 2, "the route was walked twice, so it must be counted twice");
}
