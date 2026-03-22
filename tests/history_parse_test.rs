use std::path::PathBuf;

fn fixture_path(name: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p.to_string_lossy().into_owned()
}

#[test]
fn test_parse_bash_history() {
    let path = fixture_path("bash_history_sample");
    let commands = ukrop::history::bash::parse_history_file(&path).unwrap();
    assert!(!commands.is_empty());
    assert!(!commands.iter().any(|c| c == "ls"));
    assert!(!commands.iter().any(|c| c == "cd"));
    assert!(!commands.iter().any(|c| c == "clear"));
    assert!(!commands.iter().any(|c| c == "exit"));
    assert!(commands.iter().any(|c| c == "git status"));
    assert!(commands.iter().any(|c| c == "cargo build --release"));
}

#[test]
fn test_parse_zsh_history() {
    let path = fixture_path("zsh_history_sample");
    let commands = ukrop::history::zsh::parse_history_file(&path).unwrap();
    assert!(!commands.is_empty());
    assert!(!commands.iter().any(|c| c == "ls"));
    assert!(!commands.iter().any(|c| c == "cd"));
    assert!(commands.iter().any(|c| c == "git status"));
    assert!(commands.iter().any(|c| c == "cargo build --release"));
    assert!(!commands.iter().any(|c| c.starts_with(": ")));
}

#[test]
fn test_ssh_config_parse() {
    let path = fixture_path("ssh_config_sample");
    let hosts = ukrop::ssh::config::parse_ssh_config_file(&path).unwrap();
    assert_eq!(hosts.len(), 4); // webserver, db-prod, staging, bastion (*.internal is wildcard)

    let web = hosts.iter().find(|h| h.host == "webserver").unwrap();
    assert_eq!(web.hostname.as_deref(), Some("192.168.1.100"));
    assert_eq!(web.user.as_deref(), Some("admin"));
    assert_eq!(web.port, Some(2222));

    // db-prod should inherit User=root and Port=22 from Host *
    let db = hosts.iter().find(|h| h.host == "db-prod").unwrap();
    assert_eq!(db.hostname.as_deref(), Some("db.example.com"));
    assert_eq!(db.user.as_deref(), Some("root"));
    assert_eq!(db.port, Some(22));

    let bastion = hosts.iter().find(|h| h.host == "bastion").unwrap();
    assert_eq!(bastion.port, Some(8022));
}

#[test]
fn test_parse_ssh_command_from_history() {
    let host = ukrop::history::parse_ssh_command("ssh myhost").unwrap();
    assert_eq!(host.hostname.as_deref(), Some("myhost"));
    assert_eq!(host.user, None);
    assert_eq!(host.port, None);

    let host = ukrop::history::parse_ssh_command("ssh root@10.0.0.1").unwrap();
    assert_eq!(host.hostname.as_deref(), Some("10.0.0.1"));
    assert_eq!(host.user.as_deref(), Some("root"));

    let host = ukrop::history::parse_ssh_command("ssh -p 2222 admin@server.com").unwrap();
    assert_eq!(host.hostname.as_deref(), Some("server.com"));
    assert_eq!(host.user.as_deref(), Some("admin"));
    assert_eq!(host.port, Some(2222));

    // Should skip non-ssh commands
    assert!(ukrop::history::parse_ssh_command("ls -la").is_none());
    assert!(ukrop::history::parse_ssh_command("ssh -W proxy:22 host").is_none());
}

#[test]
fn test_bash_dedup() {
    let path = fixture_path("bash_history_sample");
    let commands = ukrop::history::bash::parse_history_file(&path).unwrap();
    let unique_count = commands.len();
    let mut deduped = commands.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(unique_count, deduped.len(), "should have no duplicates");
}
