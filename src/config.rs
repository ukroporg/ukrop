use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ignore_patterns: Vec<String>,
    pub scoring: ScoringConfig,
    pub cleanup: CleanupConfig,
    pub theme: ThemeConfig,
    pub layout: LayoutConfig,
    pub confirm_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringConfig {
    pub frecency_weight: f64,
    pub frecency_cap: i32,
    pub substring_bonus: i32,
    pub prefix_bonus: i32,
    /// Negative. Applied to fuzzy-only matches so they rank below substring matches.
    pub fuzzy_penalty: i32,
    /// Multiplies `fuzzy::contiguity_score` so fuzzy matches whose characters
    /// stayed together outrank scattered ones. Fuzzy tier only.
    pub contiguity_weight: f64,
    pub contiguity_cap: i32,
    pub favorite_bonus: i32,
    pub recency_24h_bonus: i32,
    pub recency_7d_bonus: i32,
    pub cwd_bonus: i32,
    pub transition_weight: f64,
    pub transition_cap: i32,
    pub brevity_bonus_max: i32,
    pub type_bonus: TypeBonusConfig,
}

/// Position-dependent bonus for `cd` and `ssh` rows. Index is the number of rows
/// of that type already emitted above; the last element is the floor for all
/// higher indices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TypeBonusConfig {
    pub schedule: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CleanupConfig {
    pub stale_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub preset: ThemePreset,
    pub selection_bold: bool,
    pub match_underline: bool,
    pub favorite_italic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    Default,
    Light,
    Nord,
    Solarized,
    Monochrome,
    Dracula,
    Gruvbox,
    Catppuccin,
    TokyoNight,
    Kanagawa,
    Everforest,
    Rose,
}

impl ThemePreset {
    pub const ALL: &'static [ThemePreset] = &[
        ThemePreset::Default,
        ThemePreset::Light,
        ThemePreset::Nord,
        ThemePreset::Solarized,
        ThemePreset::Monochrome,
        ThemePreset::Dracula,
        ThemePreset::Gruvbox,
        ThemePreset::Catppuccin,
        ThemePreset::TokyoNight,
        ThemePreset::Kanagawa,
        ThemePreset::Everforest,
        ThemePreset::Rose,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ThemePreset::Default => "Default",
            ThemePreset::Light => "Light",
            ThemePreset::Nord => "Nord",
            ThemePreset::Solarized => "Solarized",
            ThemePreset::Monochrome => "Monochrome",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::Gruvbox => "Gruvbox",
            ThemePreset::Catppuccin => "Catppuccin",
            ThemePreset::TokyoNight => "Tokyo Night",
            ThemePreset::Kanagawa => "Kanagawa",
            ThemePreset::Everforest => "Everforest",
            ThemePreset::Rose => "Rose",
        }
    }

    pub fn index(&self) -> usize {
        Self::ALL.iter().position(|p| p == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL.get(i).cloned().unwrap_or(ThemePreset::Default)
    }
}

/// Deprecated as of 0.20.0. The unified list has no panels, so these ratios are
/// parsed for backward compatibility and then ignored. Do not remove the struct:
/// existing config files still contain a `[layout]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    pub left_panel_pct: u16,
    pub cd_panel_pct: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignore_patterns: Vec::new(),
            scoring: ScoringConfig::default(),
            cleanup: CleanupConfig::default(),
            theme: ThemeConfig::default(),
            layout: LayoutConfig::default(),
            confirm_delete: true,
        }
    }
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            frecency_weight: 100.0,
            frecency_cap: 5000,
            substring_bonus: 8000,
            prefix_bonus: 10000,
            fuzzy_penalty: -4000,
            contiguity_weight: 200.0,
            contiguity_cap: 6000,
            favorite_bonus: 5000,
            recency_24h_bonus: 6000,
            recency_7d_bonus: 2500,
            cwd_bonus: 4000,
            transition_weight: 100.0,
            transition_cap: 4000,
            brevity_bonus_max: 3000,
            type_bonus: TypeBonusConfig::default(),
        }
    }
}

impl Default for TypeBonusConfig {
    fn default() -> Self {
        Self {
            schedule: vec![3000, 1500, 0, -1500, -3000],
        }
    }
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self { stale_days: 90 }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: ThemePreset::Default,
            selection_bold: true,
            match_underline: true,
            favorite_italic: false,
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            left_panel_pct: 25,
            cd_panel_pct: 75,
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("UKROP_CONFIG_PATH") {
        return Some(PathBuf::from(p));
    }
    dirs::config_dir().map(|d| d.join("ukrop").join("config.toml"))
}

impl Config {
    pub fn load() -> Config {
        let path = match config_path() {
            Some(p) => p,
            None => return Config::default(),
        };

        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return Config::default(),
        };

        match toml::from_str::<Config>(&text) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "ukrop: failed to parse config {}: {}; using defaults",
                    path.display(),
                    e
                );
                Config::default()
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config path"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&path, text)?;
        Ok(())
    }
}

/// Returns true if `cmd` should not be recorded, based on the config's ignore patterns.
///
/// Pattern matching rules:
/// - `" "` (a single space) — matches any command that starts with a space.
/// - `"foo *"` (prefix with trailing ` *`) — matches any command that starts with `"foo "`.
/// - `"foo"` (exact, no wildcard) — matches only the command `"foo"` verbatim.
pub fn should_ignore(config: &Config, cmd: &str) -> bool {
    for pattern in &config.ignore_patterns {
        if pattern == " " {
            if cmd.starts_with(' ') {
                return true;
            }
        } else if let Some(prefix) = pattern.strip_suffix(" *") {
            // e.g. "cd *" → matches "cd foo", "cd bar", but requires the prefix + space
            if cmd.starts_with(&format!("{} ", prefix)) {
                return true;
            }
        } else {
            // exact match
            if cmd == pattern.as_str() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(patterns: &[&str]) -> Config {
        Config {
            ignore_patterns: patterns.iter().map(|s| s.to_string()).collect(),
            ..Config::default()
        }
    }

    #[test]
    fn test_exact_match() {
        let c = cfg(&["ls", "pwd"]);
        assert!(should_ignore(&c, "ls"));
        assert!(should_ignore(&c, "pwd"));
        assert!(!should_ignore(&c, "lsof"));
        assert!(!should_ignore(&c, "ls -la"));
    }

    #[test]
    fn test_prefix_wildcard() {
        let c = cfg(&["cd *"]);
        assert!(should_ignore(&c, "cd /tmp"));
        assert!(should_ignore(&c, "cd ~"));
        assert!(!should_ignore(&c, "cd")); // no trailing space+arg
        assert!(!should_ignore(&c, "cdd /tmp"));
    }

    #[test]
    fn test_leading_space() {
        let c = cfg(&[" "]);
        assert!(should_ignore(&c, " secret-command"));
        assert!(!should_ignore(&c, "visible-command"));
    }

    #[test]
    fn test_no_patterns() {
        let c = Config::default();
        assert!(!should_ignore(&c, "ls"));
        assert!(!should_ignore(&c, " hidden"));
    }

    #[test]
    fn test_defaults() {
        let c = Config::default();
        assert_eq!(c.scoring.frecency_weight, 100.0);
        assert_eq!(c.scoring.frecency_cap, 5000);
        assert_eq!(c.scoring.substring_bonus, 8000);
        assert_eq!(c.scoring.prefix_bonus, 10000);
        assert_eq!(c.scoring.fuzzy_penalty, -4000);
        assert_eq!(c.scoring.favorite_bonus, 5000);
        assert_eq!(c.scoring.recency_24h_bonus, 6000);
        assert_eq!(c.scoring.recency_7d_bonus, 2500);
        assert_eq!(c.scoring.cwd_bonus, 4000);
        assert_eq!(c.scoring.transition_weight, 100.0);
        assert_eq!(c.scoring.transition_cap, 4000);
        assert_eq!(c.scoring.brevity_bonus_max, 3000);
        assert_eq!(c.scoring.type_bonus.schedule, vec![3000, 1500, 0, -1500, -3000]);
        assert_eq!(c.cleanup.stale_days, 90);
        assert!(c.ignore_patterns.is_empty());
        assert_eq!(c.theme.preset, ThemePreset::Default);
        assert!(c.theme.selection_bold);
        assert!(c.theme.match_underline);
        assert!(!c.theme.favorite_italic);
    }

    #[test]
    fn test_partial_config_keeps_defaults() {
        // An existing user config predating this feature must still load.
        let text = r#"
ignore_patterns = ["ls"]

[scoring]
frecency_weight = 250.0

[layout]
left_panel_pct = 30
cd_panel_pct = 60
"#;
        let c: Config = toml::from_str(text).expect("old config must still parse");
        assert_eq!(c.scoring.frecency_weight, 250.0);
        assert_eq!(c.scoring.recency_24h_bonus, 6000, "unset key falls back to default");
        assert_eq!(c.scoring.type_bonus.schedule, vec![3000, 1500, 0, -1500, -3000]);
        assert_eq!(c.ignore_patterns, vec!["ls".to_string()]);
    }

    #[test]
    fn test_type_bonus_schedule_override() {
        let text = r#"
[scoring.type_bonus]
schedule = [100, 0, -100]
"#;
        let c: Config = toml::from_str(text).unwrap();
        assert_eq!(c.scoring.type_bonus.schedule, vec![100, 0, -100]);
    }
}
