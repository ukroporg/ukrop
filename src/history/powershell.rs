use anyhow::Result;
use std::collections::HashSet;

use super::{parse_ssh_command, resolve_cd_target, dedup_with_cwd};
use crate::ssh::config::SshConfigHost;

/// Parse PowerShell history and guess cwd for each command by tracking cd commands.
pub fn parse_history_with_cwd() -> Result<Vec<(String, Option<String>)>> {
    let path = psreadline_history_path();
    parse_history_file_with_cwd(&path)
}

pub fn parse_history_file_with_cwd(path: &str) -> Result<Vec<(String, Option<String>)>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut pairs: Vec<(String, Option<String>)> = Vec::new();
    let mut current_cwd: Option<String> = None;
    let mut current = String::new();

    // Walk forward for cwd tracking
    for line in content.lines() {
        if current.is_empty() {
            current = line.to_string();
        } else {
            current = format!("{}\n{}", current, line);
        }

        // If previous line ended with backtick, keep accumulating
        if line.ends_with('`') {
            continue;
        }

        let cmd = current.trim().to_string();
        current.clear();

        if cmd.is_empty() {
            continue;
        }

        // Track cd/Set-Location for cwd guessing, also try the shared resolver
        if let Some(dir) = extract_cd_target_no_check(&cmd)
            .or_else(|| resolve_cd_target(&cmd))
        {
            current_cwd = Some(dir);
        }
        pairs.push((cmd, current_cwd.clone()));
    }

    Ok(dedup_with_cwd(pairs, should_skip))
}

/// Resolve PowerShell-specific cd targets (Set-Location, sl) without existence check.
fn extract_cd_target_no_check(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();

    let target = if let Some(rest) = cmd.strip_prefix("Set-Location ") {
        extract_set_location_arg(rest)?
    } else if let Some(rest) = cmd.strip_prefix("sl ") {
        extract_set_location_arg(rest)?
    } else {
        return None; // let resolve_cd_target handle "cd" cases
    };

    let target = target.trim_matches(|c: char| c == '"' || c == '\'');
    if target.is_empty() || target == "-" {
        return None;
    }
    if target == "~" {
        return dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
    }
    if target.starts_with("~/") {
        dirs::home_dir().map(|h| h.join(&target[2..]).to_string_lossy().into_owned())
    } else if target.starts_with('/') {
        Some(target.to_string())
    } else {
        None
    }
}

pub fn parse_history() -> Result<Vec<String>> {
    let path = psreadline_history_path();
    parse_history_file(&path)
}

pub fn parse_history_file(path: &str) -> Result<Vec<String>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut seen = HashSet::new();
    let mut commands = Vec::new();

    // PSReadLine history is one command per line (newest at end),
    // with multiline commands joined by backtick-newline
    let mut current = String::new();
    for line in content.lines().rev() {
        if current.is_empty() {
            current = line.to_string();
        } else {
            // Multiline continuation: previous line ended with backtick
            current = format!("{}\n{}", line, current);
        }

        // If line ends with backtick (PS line continuation), keep accumulating
        if line.ends_with('`') {
            continue;
        }

        let cmd = current.trim().to_string();
        current.clear();

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

fn should_skip(cmd: &str) -> bool {
    let trivial = [
        "ls", "cd", "pwd", "exit", "clear", "cls", "history",
        "dir", "Get-Location", "Get-ChildItem", "ukrop",
    ];
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    trivial.contains(&first_word) || cmd.len() < 3
}

pub fn extract_directories_from_history() -> Result<Vec<String>> {
    let path = psreadline_history_path();
    extract_directories_from_file(&path)
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
    let path = psreadline_history_path();
    extract_ssh_hosts_from_file(&path)
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

/// Return the default PSReadLine history file path.
/// - Windows: $env:APPDATA\Microsoft\Windows\PowerShell\PSReadLine\ConsoleHost_history.txt
/// - macOS/Linux: ~/.local/share/powershell/PSReadLine/ConsoleHost_history.txt
fn psreadline_history_path() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return std::path::PathBuf::from(appdata)
                .join("Microsoft/Windows/PowerShell/PSReadLine/ConsoleHost_history.txt")
                .to_string_lossy()
                .into_owned();
        }
    }

    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/share"))
                .unwrap_or_default()
        });
    base.join("powershell/PSReadLine/ConsoleHost_history.txt")
        .to_string_lossy()
        .into_owned()
}

fn extract_cd_target(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();

    // Handle PowerShell Set-Location and cd/sl aliases
    let target = if let Some(rest) = cmd.strip_prefix("Set-Location ") {
        extract_set_location_arg(rest)?
    } else if let Some(rest) = cmd.strip_prefix("sl ") {
        extract_set_location_arg(rest)?
    } else if cmd.starts_with("cd ") {
        let rest = &cmd[3..];
        // PowerShell cd supports -LiteralPath and -Path
        extract_set_location_arg(rest)?
    } else if cmd == "cd" {
        return dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
    } else {
        return None;
    };

    let target = target.trim_matches(|c: char| c == '"' || c == '\'');

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

/// Extract the path argument from a Set-Location/cd/sl command,
/// handling -LiteralPath and -Path parameters.
fn extract_set_location_arg(args: &str) -> Option<String> {
    let args = args.trim();
    if let Some(rest) = args.strip_prefix("-LiteralPath ") {
        Some(rest.trim().trim_matches(|c: char| c == '"' || c == '\'').to_string())
    } else if let Some(rest) = args.strip_prefix("-Path ") {
        Some(rest.trim().trim_matches(|c: char| c == '"' || c == '\'').to_string())
    } else if args.starts_with('-') {
        None
    } else {
        Some(args.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("ukrop_test_ps");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_parse_history_deduplicates() {
        let path = write_tmp("dedup.txt", "git status\necho hello\ngit status\n");
        let cmds = parse_history_file(path.to_str().unwrap()).unwrap();
        let count = cmds.iter().filter(|c| c.as_str() == "git status").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_parse_history_skips_trivial() {
        let path = write_tmp("trivial.txt", "ls\ncls\ncd\nexit\ngit push\n");
        let cmds = parse_history_file(path.to_str().unwrap()).unwrap();
        assert_eq!(cmds, vec!["git push"]);
    }

    #[test]
    fn test_extract_cd_target_basic() {
        assert_eq!(extract_cd_target("cd /tmp"), Some("/tmp".to_string()).filter(|p| std::path::Path::new(p).is_dir()));
    }

    #[test]
    fn test_extract_cd_target_set_location() {
        assert_eq!(
            extract_cd_target("Set-Location /tmp"),
            Some("/tmp".to_string()).filter(|p| std::path::Path::new(p).is_dir())
        );
    }

    #[test]
    fn test_extract_cd_target_literal_path() {
        assert_eq!(
            extract_cd_target("Set-Location -LiteralPath /tmp"),
            Some("/tmp".to_string()).filter(|p| std::path::Path::new(p).is_dir())
        );
    }

    #[test]
    fn test_missing_file_returns_empty() {
        let cmds = parse_history_file("/nonexistent/path").unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_extract_ssh_hosts() {
        let path = write_tmp("ssh.txt", "ssh user@example.com\ngit push\nssh -p 2222 root@server\n");
        let hosts = extract_ssh_hosts_from_file(path.to_str().unwrap()).unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].hostname.as_deref(), Some("example.com"));
        assert_eq!(hosts[1].user.as_deref(), Some("root"));
    }
}
