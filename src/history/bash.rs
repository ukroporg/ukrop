use anyhow::Result;
use std::collections::HashSet;

use super::{parse_ssh_command, resolve_cd_target, dedup_with_cwd};
use crate::ssh::config::SshConfigHost;

/// Parse bash history and guess cwd for each command by tracking cd commands.
pub fn parse_history_with_cwd() -> Result<Vec<(String, Option<String>)>> {
    let path = dirs::home_dir()
        .map(|h| h.join(".bash_history"))
        .unwrap_or_default();
    parse_history_file_with_cwd(&path.to_string_lossy())
}

pub fn parse_history_file_with_cwd(path: &str) -> Result<Vec<(String, Option<String>)>> {
    Ok(dedup_with_cwd(
        parse_history_file_pairs_raw(path)?,
        should_skip,
    ))
}

/// Parse bash history and guess cwd for each command by tracking cd commands,
/// without deduping. Used to derive directory/ssh transitions, where a route
/// walked more than once must be counted more than once.
pub fn parse_history_with_cwd_raw() -> Result<Vec<(String, Option<String>)>> {
    let path = dirs::home_dir()
        .map(|h| h.join(".bash_history"))
        .unwrap_or_default();
    parse_history_file_pairs_raw(&path.to_string_lossy())
}

pub fn parse_history_file_pairs_raw(path: &str) -> Result<Vec<(String, Option<String>)>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut pairs: Vec<(String, Option<String>)> = Vec::new();
    let mut current_cwd: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let next_cwd = resolve_cd_target(line);
        pairs.push((line.to_string(), current_cwd.clone()));
        if let Some(dir) = next_cwd {
            current_cwd = Some(dir);
        }
    }

    Ok(pairs)
}

pub fn parse_history() -> Result<Vec<String>> {
    let path = dirs::home_dir()
        .map(|h| h.join(".bash_history"))
        .unwrap_or_default();
    parse_history_file(&path.to_string_lossy())
}

pub fn parse_history_file(path: &str) -> Result<Vec<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut commands = Vec::new();

    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if should_skip(line) {
            continue;
        }
        if seen.insert(line.to_string()) {
            commands.push(line.to_string());
        }
    }

    commands.reverse();
    Ok(commands)
}

fn should_skip(cmd: &str) -> bool {
    let trivial = ["ls", "cd", "pwd", "exit", "clear", "history", "ukrop"];
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    trivial.contains(&first_word) || cmd.len() < 3
}

pub fn extract_directories_from_history() -> Result<Vec<String>> {
    let path = dirs::home_dir()
        .map(|h| h.join(".bash_history"))
        .unwrap_or_default();
    extract_directories_from_file(&path.to_string_lossy())
}

pub fn extract_directories_from_file(path: &str) -> Result<Vec<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut dirs = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(dir) = extract_cd_target(line) {
            if seen.insert(dir.clone()) {
                dirs.push(dir);
            }
        }
    }

    Ok(dirs)
}

pub fn extract_ssh_hosts_from_history() -> Result<Vec<SshConfigHost>> {
    let path = dirs::home_dir()
        .map(|h| h.join(".bash_history"))
        .unwrap_or_default();
    extract_ssh_hosts_from_file(&path.to_string_lossy())
}

pub fn extract_ssh_hosts_from_file(path: &str) -> Result<Vec<SshConfigHost>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut hosts = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(host) = parse_ssh_command(line) {
            if seen.insert(host.host.clone()) {
                hosts.push(host);
            }
        }
    }

    Ok(hosts)
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
