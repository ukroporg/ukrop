use anyhow::Result;
use std::collections::HashSet;

use super::{parse_ssh_command, resolve_cd_target, dedup_with_cwd};
use crate::ssh::config::SshConfigHost;

/// Parse zsh history and guess cwd for each command by tracking cd commands.
pub fn parse_history_with_cwd() -> Result<Vec<(String, Option<String>)>> {
    let path = std::env::var("HISTFILE").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".zsh_history").to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    parse_history_file_with_cwd(&path)
}

pub fn parse_history_file_with_cwd(path: &str) -> Result<Vec<(String, Option<String>)>> {
    Ok(dedup_with_cwd(
        parse_history_file_pairs_raw(path)?,
        should_skip,
    ))
}

/// Parse zsh history and guess cwd for each command by tracking cd commands,
/// without deduping. Used to derive directory/ssh transitions, where a route
/// walked more than once must be counted more than once.
pub fn parse_history_with_cwd_raw() -> Result<Vec<(String, Option<String>)>> {
    let path = std::env::var("HISTFILE").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".zsh_history").to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    parse_history_file_pairs_raw(&path)
}

pub fn parse_history_file_pairs_raw(path: &str) -> Result<Vec<(String, Option<String>)>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(vec![]),
    };

    let content = String::from_utf8_lossy(&bytes);
    let mut pairs: Vec<(String, Option<String>)> = Vec::new();
    let mut current_cwd: Option<String> = None;
    let mut continuation = String::new();

    for line in content.lines() {
        if !continuation.is_empty() {
            continuation = format!("{}\n{}", continuation, line);
            if !line.ends_with('\\') {
                let cmd = extract_zsh_command(&continuation);
                continuation.clear();
                if !cmd.is_empty() {
                    let next_cwd = resolve_cd_target(&cmd);
                    pairs.push((cmd, current_cwd.clone()));
                    if let Some(dir) = next_cwd {
                        current_cwd = Some(dir);
                    }
                }
            }
            continue;
        }

        if line.ends_with('\\') {
            continuation = line.to_string();
            continue;
        }

        let cmd = extract_zsh_command(line);
        if cmd.is_empty() {
            continue;
        }
        let next_cwd = resolve_cd_target(&cmd);
        pairs.push((cmd, current_cwd.clone()));
        if let Some(dir) = next_cwd {
            current_cwd = Some(dir);
        }
    }

    Ok(pairs)
}

pub fn parse_history() -> Result<Vec<String>> {
    let path = std::env::var("HISTFILE").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".zsh_history").to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    parse_history_file(&path)
}

pub fn parse_history_file(path: &str) -> Result<Vec<String>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(vec![]),
    };

    // zsh history can contain invalid UTF-8, be lenient
    let content = String::from_utf8_lossy(&bytes);

    let mut seen = HashSet::new();
    let mut commands = Vec::new();
    let mut continuation = String::new();

    for line in content.lines().rev() {
        // Handle line continuations (ending with \)
        if !continuation.is_empty() {
            continuation = format!("{}\n{}", line, continuation);
            if !line.ends_with('\\') {
                let cmd = extract_zsh_command(&continuation);
                continuation.clear();
                if !cmd.is_empty() && !should_skip(&cmd) && seen.insert(cmd.clone()) {
                    commands.push(cmd);
                }
            }
            continue;
        }

        if line.ends_with('\\') {
            continuation = line.to_string();
            continue;
        }

        let cmd = extract_zsh_command(line);
        if cmd.is_empty() || should_skip(&cmd) {
            continue;
        }
        if seen.insert(cmd.clone()) {
            commands.push(cmd);
        }
    }

    commands.reverse();
    Ok(commands)
}

/// Extract the command from a zsh history line.
/// zsh extended history format: `: timestamp:duration;command`
fn extract_zsh_command(line: &str) -> String {
    let line = line.trim();
    if line.starts_with(": ") {
        // Extended format: `: 1234567890:0;actual command`
        if let Some(pos) = line.find(';') {
            return line[pos + 1..].trim().to_string();
        }
    }
    line.to_string()
}

fn should_skip(cmd: &str) -> bool {
    let trivial = ["ls", "cd", "pwd", "exit", "clear", "history", "ukrop"];
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    trivial.contains(&first_word) || cmd.len() < 3
}

/// Extract directories from zsh history by parsing `cd` commands.
pub fn extract_directories_from_history() -> Result<Vec<String>> {
    let path = std::env::var("HISTFILE").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".zsh_history").to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    extract_directories_from_file(&path)
}

pub fn extract_directories_from_file(path: &str) -> Result<Vec<String>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(vec![]),
    };

    let content = String::from_utf8_lossy(&bytes);
    let mut seen = HashSet::new();
    let mut dirs = Vec::new();

    for line in content.lines() {
        let cmd = extract_zsh_command(line);
        if let Some(dir) = extract_cd_target(&cmd) {
            if seen.insert(dir.clone()) {
                dirs.push(dir);
            }
        }
    }

    Ok(dirs)
}

pub fn extract_ssh_hosts_from_history() -> Result<Vec<SshConfigHost>> {
    let path = std::env::var("HISTFILE").unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".zsh_history").to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    extract_ssh_hosts_from_file(&path)
}

pub fn extract_ssh_hosts_from_file(path: &str) -> Result<Vec<SshConfigHost>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(vec![]),
    };

    let content = String::from_utf8_lossy(&bytes);
    let mut seen = HashSet::new();
    let mut hosts = Vec::new();

    for line in content.lines() {
        let cmd = extract_zsh_command(line);
        if let Some(host) = parse_ssh_command(&cmd) {
            if seen.insert(host.host.clone()) {
                hosts.push(host);
            }
        }
    }

    Ok(hosts)
}

fn extract_cd_target(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();

    // Match: cd <path>, cd -- <path>
    // Also handle chained commands: ... && cd <path>, ... ; cd <path>
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
    // cd <path> or cd -- <path>
    let target = match args.as_slice() {
        ["cd", "--", p] => *p,
        ["cd", p] if !p.starts_with('-') => *p,
        _ => return None,
    };

    // Strip quotes
    let target = target.trim_matches(|c| c == '"' || c == '\'');

    if target.is_empty() || target == "-" || target == "~" {
        if target == "~" {
            return dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
        }
        return None;
    }

    // Resolve ~ prefix
    let resolved = if target.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(&target[2..]).to_string_lossy().into_owned()
        } else {
            return None;
        }
    } else if target.starts_with('/') {
        target.to_string()
    } else {
        // Relative paths can't be resolved without knowing the working dir at that time
        return None;
    };

    // Only include if the directory still exists
    if std::path::Path::new(&resolved).is_dir() {
        Some(resolved)
    } else {
        None
    }
}
