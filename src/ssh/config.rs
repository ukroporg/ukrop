use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SshConfigHost {
    pub host: String,
    pub hostname: Option<String>,
    pub port: Option<i32>,
    pub user: Option<String>,
}

pub fn parse_ssh_config() -> Result<Vec<SshConfigHost>> {
    let path = dirs::home_dir()
        .map(|h| h.join(".ssh/config"))
        .unwrap_or_default();
    parse_ssh_config_file(&path.to_string_lossy())
}

pub fn parse_ssh_config_file(path: &str) -> Result<Vec<SshConfigHost>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };
    Ok(parse_ssh_config_str(&content))
}

pub fn parse_ssh_config_str(content: &str) -> Vec<SshConfigHost> {
    let mut results = Vec::new();
    let mut current: Option<SshConfigHost> = None;

    // Track Host * defaults
    let mut default_user: Option<String> = None;
    let mut default_port: Option<i32> = None;
    let mut in_wildcard = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = match split_directive(line) {
            Some(kv) => kv,
            None => continue,
        };

        if key.eq_ignore_ascii_case("Host") {
            // Save previous entry
            if let Some(mut entry) = current.take() {
                apply_defaults(&mut entry, &default_user, &default_port);
                results.push(entry);
            }

            in_wildcard = value.contains('*') || value.contains('?');
            if in_wildcard {
                // This is a wildcard block; parse its directives as defaults
                current = None;
            } else {
                current = Some(SshConfigHost {
                    host: value.to_string(),
                    hostname: None,
                    port: None,
                    user: None,
                });
            }
            continue;
        }

        if key.eq_ignore_ascii_case("Match") {
            // Save previous entry, skip Match blocks
            if let Some(mut entry) = current.take() {
                apply_defaults(&mut entry, &default_user, &default_port);
                results.push(entry);
            }
            in_wildcard = false;
            current = None;
            continue;
        }

        if in_wildcard {
            // Collect defaults from wildcard block
            if key.eq_ignore_ascii_case("User") {
                default_user = Some(value.to_string());
            } else if key.eq_ignore_ascii_case("Port") {
                default_port = value.parse().ok();
            }
            continue;
        }

        if let Some(ref mut entry) = current {
            if key.eq_ignore_ascii_case("Hostname") {
                entry.hostname = Some(value.to_string());
            } else if key.eq_ignore_ascii_case("Port") {
                entry.port = value.parse().ok();
            } else if key.eq_ignore_ascii_case("User") {
                entry.user = Some(value.to_string());
            }
        }
    }

    // Don't forget the last entry
    if let Some(mut entry) = current.take() {
        apply_defaults(&mut entry, &default_user, &default_port);
        results.push(entry);
    }

    results
}

fn apply_defaults(entry: &mut SshConfigHost, default_user: &Option<String>, default_port: &Option<i32>) {
    if entry.user.is_none() {
        entry.user = default_user.clone();
    }
    if entry.port.is_none() {
        entry.port = *default_port;
    }
}

fn split_directive(line: &str) -> Option<(&str, &str)> {
    // SSH config uses either whitespace or '=' as delimiter
    let line = line.trim();
    if let Some(eq_pos) = line.find('=') {
        let key = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();
        if !key.is_empty() && !value.is_empty() {
            return Some((key, value));
        }
    }
    let mut parts = line.splitn(2, char::is_whitespace);
    let key = parts.next()?;
    let value = parts.next()?.trim();
    if value.is_empty() {
        return None;
    }
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_host() {
        let config = "Host myserver\n  Hostname 192.168.1.1\n  User admin\n  Port 2222\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "myserver");
        assert_eq!(hosts[0].hostname.as_deref(), Some("192.168.1.1"));
        assert_eq!(hosts[0].user.as_deref(), Some("admin"));
        assert_eq!(hosts[0].port, Some(2222));
    }

    #[test]
    fn test_wildcard_defaults() {
        let config = "Host *\n  User root\n  Port 22\n\nHost web\n  Hostname web.example.com\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "web");
        assert_eq!(hosts[0].user.as_deref(), Some("root"));
        assert_eq!(hosts[0].port, Some(22));
    }

    #[test]
    fn test_skip_wildcards() {
        let config = "Host *.example.com\n  User deploy\n\nHost prod\n  Hostname prod.example.com\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "prod");
    }

    #[test]
    fn test_multiple_hosts() {
        let config = "Host a\n  Hostname 1.1.1.1\n\nHost b\n  Hostname 2.2.2.2\n  User bob\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].host, "a");
        assert_eq!(hosts[1].host, "b");
        assert_eq!(hosts[1].user.as_deref(), Some("bob"));
    }

    #[test]
    fn test_comments_and_empty_lines() {
        let config = "# This is a comment\n\nHost test\n  # Another comment\n  Hostname test.local\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("test.local"));
    }

    #[test]
    fn test_equals_delimiter() {
        let config = "Host myhost\n  Hostname=10.0.0.1\n  Port=443\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.1"));
        assert_eq!(hosts[0].port, Some(443));
    }

    #[test]
    fn test_host_without_details() {
        let config = "Host bare\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "bare");
        assert_eq!(hosts[0].hostname, None);
    }

    #[test]
    fn test_override_defaults() {
        let config = "Host *\n  User default\n  Port 22\n\nHost special\n  Hostname s.com\n  User custom\n  Port 8022\n";
        let hosts = parse_ssh_config_str(config);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("custom"));
        assert_eq!(hosts[0].port, Some(8022));
    }
}
