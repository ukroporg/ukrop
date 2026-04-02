pub mod bash;
pub mod fish;
pub mod powershell;
pub mod zsh;

use crate::ssh::config::SshConfigHost;

/// Resolve a cd target path without checking if the directory exists.
/// Used during history import to guess cwd for subsequent commands.
/// Returns None for relative paths (can't resolve without prior cwd knowledge).
pub fn resolve_cd_target(cmd: &str) -> Option<String> {
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
        None // relative path — can't resolve
    }
}

/// Given a list of (command, cwd_at_time) pairs in chronological order,
/// dedup keeping the last occurrence of each command with its cwd.
/// Returns commands that pass `should_skip` filter removed, in original order.
pub fn dedup_with_cwd(
    pairs: Vec<(String, Option<String>)>,
    should_skip: fn(&str) -> bool,
) -> Vec<(String, Option<String>)> {
    use std::collections::HashMap;
    // Track last-seen index for each command
    let mut last_idx: HashMap<String, usize> = HashMap::new();
    for (i, (cmd, _)) in pairs.iter().enumerate() {
        last_idx.insert(cmd.clone(), i);
    }
    pairs
        .into_iter()
        .enumerate()
        .filter(|(i, (cmd, _))| {
            !should_skip(cmd) && last_idx.get(cmd.as_str()) == Some(i)
        })
        .map(|(_, pair)| pair)
        .collect()
}

/// Parse an ssh command line into an SshConfigHost.
/// Handles: ssh host, ssh user@host, ssh -p PORT host, ssh -p PORT user@host
/// Skips: ssh-keygen, ssh-copy-id, ssh-add, ssh -W (proxy)
pub fn parse_ssh_command(cmd: &str) -> Option<SshConfigHost> {
    let cmd = cmd.trim();
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() || parts[0] != "ssh" {
        return None;
    }

    // Skip ssh-* commands that happen to start with "ssh" when typed differently
    // The split_whitespace check above ensures the first word is exactly "ssh"

    let mut port: Option<i32> = None;
    let mut target: Option<&str> = None;
    let mut i = 1;

    while i < parts.len() {
        let arg = parts[i];

        // Skip flags that indicate non-interactive use
        if arg == "-W" || arg == "-G" || arg == "-T" || arg == "-N" || arg == "-f" {
            return None;
        }

        if arg == "-p" {
            i += 1;
            if i < parts.len() {
                port = parts[i].parse().ok();
            }
            i += 1;
            continue;
        }

        // Skip options that take a value
        if matches!(arg, "-o" | "-i" | "-l" | "-L" | "-R" | "-D" | "-J" | "-F" | "-E" | "-S" | "-b" | "-c" | "-m" | "-w" | "-O" | "-Q") {
            i += 2; // skip flag + value
            continue;
        }

        // Skip boolean flags
        if arg.starts_with('-') {
            i += 1;
            continue;
        }

        // This should be the target (host or user@host)
        target = Some(arg);
        break;
    }

    let target = target?;

    // Skip if target looks like a command (contains /, starts with .)
    if target.contains('/') || target.starts_with('.') {
        return None;
    }

    let (user, hostname) = if let Some(at_pos) = target.find('@') {
        let u = &target[..at_pos];
        let h = &target[at_pos + 1..];
        if h.is_empty() {
            return None;
        }
        (Some(u.to_string()), h.to_string())
    } else {
        (None, target.to_string())
    };

    // Build host key for dedup: user@host:port or host:port
    let host_key = format_ssh_args(user.as_deref(), &hostname, port);

    Some(SshConfigHost {
        host: host_key,
        hostname: Some(hostname),
        port,
        user,
    })
}

/// Format SSH connection args as they would be passed to ssh.
/// This becomes the `host` field (unique key) and the value output by TUI.
pub fn format_ssh_args(user: Option<&str>, hostname: &str, port: Option<i32>) -> String {
    let mut args = String::new();
    if let Some(p) = port {
        if p != 22 {
            args.push_str(&format!("-p {} ", p));
        }
    }
    if let Some(u) = user {
        args.push_str(&format!("{}@{}", u, hostname));
    } else {
        args.push_str(hostname);
    }
    args
}
