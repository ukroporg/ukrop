use anyhow::Result;
use std::collections::HashSet;

use super::{parse_ssh_command, resolve_cd_target, dedup_with_cwd};
use crate::ssh::config::SshConfigHost;

/// Parse fish history and guess cwd for each command by tracking cd commands.
pub fn parse_history_with_cwd() -> Result<Vec<(String, Option<String>)>> {
    let path = fish_history_path();
    parse_history_file_with_cwd(&path)
}

pub fn parse_history_file_with_cwd(path: &str) -> Result<Vec<(String, Option<String>)>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut pairs: Vec<(String, Option<String>)> = Vec::new();
    let mut current_cwd: Option<String> = None;

    for cmd in iter_commands(&content) {
        if let Some(dir) = resolve_cd_target(&cmd) {
            current_cwd = Some(dir);
        }
        pairs.push((cmd, current_cwd.clone()));
    }

    Ok(dedup_with_cwd(pairs, should_skip))
}

pub fn parse_history() -> Result<Vec<String>> {
    let path = fish_history_path();
    parse_history_file(&path)
}

pub fn parse_history_file(path: &str) -> Result<Vec<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut commands = Vec::new();

    for cmd in iter_commands_rev(&content) {
        if should_skip(&cmd) {
            continue;
        }
        if seen.insert(cmd.clone()) {
            commands.push(cmd);
        }
    }

    commands.reverse();
    Ok(commands)
}

pub fn extract_directories_from_history() -> Result<Vec<String>> {
    let path = fish_history_path();
    extract_directories_from_file(&path)
}

pub fn extract_directories_from_file(path: &str) -> Result<Vec<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut dirs = Vec::new();

    for cmd in iter_commands(&content) {
        if let Some(dir) = extract_cd_target(&cmd) {
            if seen.insert(dir.clone()) {
                dirs.push(dir);
            }
        }
    }

    Ok(dirs)
}

pub fn extract_ssh_hosts_from_history() -> Result<Vec<SshConfigHost>> {
    let path = fish_history_path();
    extract_ssh_hosts_from_file(&path)
}

pub fn extract_ssh_hosts_from_file(path: &str) -> Result<Vec<SshConfigHost>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut hosts = Vec::new();

    for cmd in iter_commands(&content) {
        if let Some(host) = parse_ssh_command(&cmd) {
            if seen.insert(host.host.clone()) {
                hosts.push(host);
            }
        }
    }

    Ok(hosts)
}

/// Return the default fish history file path.
/// Respects `$XDG_DATA_HOME` if set, otherwise uses `~/.local/share/fish/fish_history`.
fn fish_history_path() -> String {
    let base = std::env::var("XDG_DATA_HOME").ok().map(std::path::PathBuf::from).unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".local/share"))
            .unwrap_or_default()
    });
    base.join("fish/fish_history").to_string_lossy().into_owned()
}

/// Unescape fish history escape sequences: `\\n` -> newline, `\\\\` -> backslash.
fn unescape_fish(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('\\') => {
                    chars.next();
                    out.push('\\');
                }
                _ => {
                    out.push(c);
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Collect all commands from the fish history content (forward order).
fn iter_commands(content: &str) -> Vec<String> {
    let mut commands = Vec::new();
    for line in content.lines() {
        if let Some(raw) = line.strip_prefix("- cmd: ") {
            commands.push(unescape_fish(raw.trim()));
        }
    }
    commands
}

/// Collect all commands from the fish history content in reverse order.
fn iter_commands_rev(content: &str) -> Vec<String> {
    let mut commands = iter_commands(content);
    commands.reverse();
    commands
}

fn should_skip(cmd: &str) -> bool {
    let trivial = ["ls", "cd", "pwd", "exit", "clear", "history", "ukrop"];
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    trivial.contains(&first_word) || cmd.len() < 3
}

fn extract_cd_target(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();

    let cd_part = if let Some(pos) = cmd.rfind("&& cd ") {
        &cmd[pos + 3..]
    } else if let Some(pos) = cmd.rfind("; cd ") {
        &cmd[pos + 2..]
    } else if cmd.starts_with("cd ") {
        cmd
    } else if cmd == "cd" {
        return dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
    } else {
        return None;
    };

    let args: Vec<&str> = cd_part.split_whitespace().collect();
    let target = match args.as_slice() {
        ["cd", "--", p] => *p,
        ["cd", p] if !p.starts_with('-') => *p,
        _ => return None,
    };

    let target = target.trim_matches(|c| c == '"' || c == '\'');

    if target.is_empty() || target == "-" || target == "~" {
        if target == "~" {
            return dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
        }
        return None;
    }

    let resolved = if target.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&target[2..]).to_string_lossy().into_owned()
        } else {
            return None;
        }
    } else if target.starts_with('/') {
        target.to_string()
    } else {
        return None;
    };

    if std::path::Path::new(&resolved).is_dir() {
        Some(resolved)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
- cmd: git status
  when: 1700000001
- cmd: ls
  when: 1700000002
- cmd: cd /tmp
  when: 1700000003
- cmd: echo hello\\nworld
  when: 1700000004
- cmd: ssh user@example.com
  when: 1700000005
- cmd: git status
  when: 1700000006
";

    fn write_tmp(content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fish_history");
        std::fs::write(&path, content).unwrap();
        (dir, path.to_str().unwrap().to_string())
    }

    #[test]
    fn test_parse_history_file_deduplicates_keeps_last() {
        let (_dir, path) = write_tmp(SAMPLE);
        let cmds = parse_history_file(&path).unwrap();
        let count = cmds.iter().filter(|c| c.as_str() == "git status").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_parse_history_file_skips_trivial() {
        let (_dir, path) = write_tmp(SAMPLE);
        let cmds = parse_history_file(&path).unwrap();
        assert!(!cmds.iter().any(|c| c.as_str() == "ls"));
        assert!(!cmds.iter().any(|c| c.as_str() == "cd /tmp"));
    }

    #[test]
    fn test_unescape_fish() {
        assert_eq!(unescape_fish("hello\\nworld"), "hello\nworld");
        assert_eq!(unescape_fish("back\\\\slash"), "back\\slash");
        assert_eq!(unescape_fish("no escapes"), "no escapes");
    }

    #[test]
    fn test_extract_directories_from_file() {
        let (_dir, path) = write_tmp("- cmd: cd /tmp\n  when: 1\n");
        let dirs = extract_directories_from_file(&path).unwrap();
        assert!(dirs.contains(&"/tmp".to_string()) || dirs.contains(&"/private/tmp".to_string()));
    }

    #[test]
    fn test_extract_ssh_hosts_from_file() {
        let (_dir, path) = write_tmp(SAMPLE);
        let hosts = extract_ssh_hosts_from_file(&path).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("example.com"));
        assert_eq!(hosts[0].user.as_deref(), Some("user"));
    }
}
