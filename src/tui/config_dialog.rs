use crate::config::{Config, LayoutConfig, ThemePreset};

pub struct ConfigDialog {
    pub fields: Vec<ConfigField>,
    pub focused: usize,
    pub dirty: bool,
    /// The `[layout]` section is no longer editable from the dialog (the
    /// unified list has no panels to size), but existing users may still
    /// have custom values in their `config.toml`. Carry them through
    /// unedited so saving the dialog doesn't silently reset them.
    layout: LayoutConfig,
}

pub struct ConfigField {
    pub label: &'static str,
    pub section: &'static str,
    pub kind: FieldKind,
}

pub enum FieldKind {
    Float { value: String },
    Uint { value: String },
    Enum { options: &'static [&'static str], selected: usize },
    Bool { value: bool },
    StringList { items: Vec<String>, selected: usize, editing_buf: Option<String> },
}

impl ConfigDialog {
    pub fn from_config(cfg: &Config) -> Self {
        let fields = vec![
            ConfigField {
                label: "frecency_weight",
                section: "Scoring",
                kind: FieldKind::Float { value: format!("{}", cfg.scoring.frecency_weight) },
            },
            ConfigField {
                label: "substring_bonus",
                section: "Scoring",
                kind: FieldKind::Uint { value: cfg.scoring.substring_bonus.to_string() },
            },
            ConfigField {
                label: "prefix_bonus",
                section: "Scoring",
                kind: FieldKind::Uint { value: cfg.scoring.prefix_bonus.to_string() },
            },
            ConfigField {
                label: "stale_days",
                section: "Cleanup",
                kind: FieldKind::Uint { value: cfg.cleanup.stale_days.to_string() },
            },
            ConfigField {
                label: "preset",
                section: "Theme",
                kind: FieldKind::Enum {
                    options: &["Default", "Light", "Nord", "Solarized", "Monochrome", "Dracula", "Gruvbox", "Catppuccin", "Tokyo Night", "Kanagawa", "Everforest", "Rose"],
                    selected: cfg.theme.preset.index(),
                },
            },
            ConfigField {
                label: "selection_bold",
                section: "Theme",
                kind: FieldKind::Bool { value: cfg.theme.selection_bold },
            },
            ConfigField {
                label: "match_underline",
                section: "Theme",
                kind: FieldKind::Bool { value: cfg.theme.match_underline },
            },
            ConfigField {
                label: "favorite_italic",
                section: "Theme",
                kind: FieldKind::Bool { value: cfg.theme.favorite_italic },
            },
            ConfigField {
                label: "confirm_delete",
                section: "Behavior",
                kind: FieldKind::Bool { value: cfg.confirm_delete },
            },
            ConfigField {
                label: "ignore_patterns",
                section: "Ignore Patterns",
                kind: FieldKind::StringList {
                    items: cfg.ignore_patterns.clone(),
                    selected: 0,
                    editing_buf: None,
                },
            },
        ];
        ConfigDialog {
            fields,
            focused: 0,
            dirty: false,
            layout: cfg.layout.clone(),
        }
    }

    pub fn to_config(&self) -> Result<Config, String> {
        let mut cfg = Config::default();
        cfg.layout = self.layout.clone();

        for field in &self.fields {
            match (field.label, &field.kind) {
                ("frecency_weight", FieldKind::Float { value }) => {
                    cfg.scoring.frecency_weight = value.parse::<f64>()
                        .map_err(|_| format!("Invalid frecency_weight: {}", value))?;
                }
                ("substring_bonus", FieldKind::Uint { value }) => {
                    cfg.scoring.substring_bonus = value.parse::<i32>()
                        .map_err(|_| format!("Invalid substring_bonus: {}", value))?;
                }
                ("prefix_bonus", FieldKind::Uint { value }) => {
                    cfg.scoring.prefix_bonus = value.parse::<i32>()
                        .map_err(|_| format!("Invalid prefix_bonus: {}", value))?;
                }
                ("stale_days", FieldKind::Uint { value }) => {
                    cfg.cleanup.stale_days = value.parse::<u64>()
                        .map_err(|_| format!("Invalid stale_days: {}", value))?;
                }
                ("preset", FieldKind::Enum { selected, .. }) => {
                    cfg.theme.preset = ThemePreset::from_index(*selected);
                }
                ("selection_bold", FieldKind::Bool { value }) => {
                    cfg.theme.selection_bold = *value;
                }
                ("match_underline", FieldKind::Bool { value }) => {
                    cfg.theme.match_underline = *value;
                }
                ("favorite_italic", FieldKind::Bool { value }) => {
                    cfg.theme.favorite_italic = *value;
                }
                ("confirm_delete", FieldKind::Bool { value }) => {
                    cfg.confirm_delete = *value;
                }
                ("ignore_patterns", FieldKind::StringList { items, .. }) => {
                    cfg.ignore_patterns = items.clone();
                }
                _ => {}
            }
        }
        Ok(cfg)
    }

    /// Total navigable rows (fields + string list items + "add new" row)
    pub fn total_rows(&self) -> usize {
        let mut count = 0;
        for field in &self.fields {
            match &field.kind {
                FieldKind::StringList { items, .. } => {
                    count += 1; // section label row
                    count += items.len(); // each item
                    count += 1; // "add new" row
                }
                _ => count += 1,
            }
        }
        count
    }

    pub fn move_up(&mut self) {
        if self.focused > 0 {
            self.focused -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.total_rows().saturating_sub(1);
        if self.focused < max {
            self.focused += 1;
        }
    }

    /// Map focused row to (field_index, sub_index).
    /// sub_index: None = the field itself, Some(i) = list item i, Some(len) = "add new" row
    pub fn focused_field(&self) -> (usize, Option<usize>) {
        let mut row = 0;
        for (fi, field) in self.fields.iter().enumerate() {
            match &field.kind {
                FieldKind::StringList { items, .. } => {
                    if self.focused == row {
                        return (fi, None); // label row (not directly editable)
                    }
                    row += 1;
                    for si in 0..items.len() {
                        if self.focused == row {
                            return (fi, Some(si));
                        }
                        row += 1;
                    }
                    // "add new" row
                    if self.focused == row {
                        return (fi, Some(items.len()));
                    }
                    row += 1;
                }
                _ => {
                    if self.focused == row {
                        return (fi, None);
                    }
                    row += 1;
                }
            }
        }
        (0, None)
    }

    pub fn handle_char(&mut self, c: char) {
        let (fi, sub) = self.focused_field();
        let field = &mut self.fields[fi];
        match &mut field.kind {
            FieldKind::Float { value } => {
                if c.is_ascii_digit() || c == '.' {
                    value.push(c);
                    self.dirty = true;
                }
            }
            FieldKind::Uint { value } => {
                if c.is_ascii_digit() {
                    value.push(c);
                    self.dirty = true;
                }
            }
            FieldKind::StringList { editing_buf: Some(buf), .. } => {
                buf.push(c);
                self.dirty = true;
            }
            _ => {}
        }
        // Ignore chars for Bool and Enum (handled by toggle/cycle)
        let _ = sub;
    }

    pub fn handle_backspace(&mut self) {
        let (fi, _sub) = self.focused_field();
        let field = &mut self.fields[fi];
        match &mut field.kind {
            FieldKind::Float { value } | FieldKind::Uint { value } => {
                value.pop();
                self.dirty = true;
            }
            FieldKind::StringList { editing_buf: Some(buf), .. } => {
                buf.pop();
                self.dirty = true;
            }
            _ => {}
        }
    }

    pub fn handle_enter(&mut self) {
        let (fi, sub) = self.focused_field();
        let field = &mut self.fields[fi];
        match &mut field.kind {
            FieldKind::Bool { value } => {
                *value = !*value;
                self.dirty = true;
            }
            FieldKind::StringList { items, editing_buf, selected: _ } => {
                if let Some(buf) = editing_buf.take() {
                    // Confirm edit
                    let trimmed = buf.trim().to_string();
                    if !trimmed.is_empty() {
                        if let Some(si) = sub {
                            if si < items.len() {
                                items[si] = trimmed;
                            } else {
                                items.push(trimmed);
                            }
                        }
                    }
                    self.dirty = true;
                } else if let Some(si) = sub {
                    // Start editing
                    if si < items.len() {
                        *editing_buf = Some(items[si].clone());
                    } else {
                        // "add new" row
                        *editing_buf = Some(String::new());
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_left(&mut self) {
        let (fi, _) = self.focused_field();
        let field = &mut self.fields[fi];
        if let FieldKind::Enum { options: _, selected } = &mut field.kind {
            if *selected > 0 {
                *selected -= 1;
                self.dirty = true;
            }
        }
    }

    pub fn handle_right(&mut self) {
        let (fi, _) = self.focused_field();
        let field = &mut self.fields[fi];
        if let FieldKind::Enum { options, selected } = &mut field.kind {
            if *selected + 1 < options.len() {
                *selected += 1;
                self.dirty = true;
            }
        }
    }

    pub fn handle_delete(&mut self) {
        let (fi, sub) = self.focused_field();
        let field = &mut self.fields[fi];
        if let FieldKind::StringList { items, editing_buf, .. } = &mut field.kind {
            if editing_buf.is_some() {
                return; // don't delete while editing
            }
            if let Some(si) = sub {
                if si < items.len() {
                    items.remove(si);
                    self.dirty = true;
                }
            }
        }
    }

    pub fn handle_escape(&mut self) -> bool {
        // If editing a string list item, cancel the edit
        let (fi, _) = self.focused_field();
        let field = &mut self.fields[fi];
        if let FieldKind::StringList { editing_buf, .. } = &mut field.kind {
            if editing_buf.is_some() {
                *editing_buf = None;
                return false; // consumed, don't close dialog
            }
        }
        true // close dialog
    }

    /// Returns true if a theme/layout field is focused (for live preview)
    pub fn is_theme_or_layout_field(&self) -> bool {
        let (fi, _) = self.focused_field();
        let section = self.fields[fi].section;
        section == "Theme" || section == "Layout"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `substring_bonus`/`prefix_bonus` are `i32` in `ScoringConfig`, but the
    /// `Uint` widget's `handle_char` imposes no length cap on digit entry
    /// (only rejects non-digits). A value that overflows `i32` but fits in
    /// `u32` (e.g. 3_000_000_000) must be rejected by `to_config`, not
    /// silently bit-reinterpreted into a negative number via `as i32`.
    #[test]
    fn test_out_of_range_scoring_value_is_rejected_not_wrapped() {
        let mut dialog = ConfigDialog::from_config(&Config::default());

        // Focus the substring_bonus field (index 1) and replace its value
        // with an out-of-i32-range, in-u32-range string, exactly as a user
        // typing digits via handle_char would produce.
        dialog.focused = 1;
        assert_eq!(dialog.fields[1].label, "substring_bonus");
        if let FieldKind::Uint { value } = &mut dialog.fields[1].kind {
            value.clear();
        }
        for c in "3000000000".chars() {
            dialog.handle_char(c);
        }

        let result = dialog.to_config();
        assert!(
            result.is_err(),
            "out-of-range substring_bonus must be rejected, got {:?}",
            result.map(|c| c.scoring.substring_bonus)
        );
    }

    #[test]
    fn test_dialog_has_no_layout_fields() {
        let cfg = crate::config::Config::default();
        let d = ConfigDialog::from_config(&cfg);
        assert!(
            !d.fields.iter().any(|f| f.label.contains("panel_pct")),
            "panel ratio fields are obsolete in the unified list"
        );
    }
}
