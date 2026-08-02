use anyhow::Result;
use crossterm::terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::fs::File;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use super::config_dialog::ConfigDialog;
use super::edit_dialog::EditDialog;
use super::fuzzy::FuzzyMatcher;
use super::input::handle_key;
use super::theme::Theme;
use super::tty_reader::PollResult;
use super::ui::draw;
use super::{PickResult, PickerEntry};
use crate::config::Config;
use crate::db::store::Store;
use std::io::Write as IoWrite;

fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut child = if cfg!(target_os = "macos") {
        std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?
    } else {
        std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .or_else(|_| {
                std::process::Command::new("xsel")
                    .args(["--clipboard", "--input"])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
            })?
    };
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }
    drop(child.stdin.take());
    child.wait()?;
    Ok(())
}

pub enum Action {
    Continue,
    Select,
    SelectEdit,
    CopyToClipboard,
    Quit,
    ToggleFavorite,
    Delete,
    ConfirmDelete,
    CancelDelete,
    CycleFilter,
    CycleFilterBack,
    ToggleCwdFilter,
    ToggleHelp,
    ToggleConfig,
    SaveConfig,
    EditCommand,
    ExecuteEdit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickerMode {
    Directories,
    Commands,
    SshHosts,
}

impl PickerMode {
    pub fn label(self) -> &'static str {
        match self {
            PickerMode::Directories => "cd",
            PickerMode::Commands => "run",
            PickerMode::SshHosts => "ssh",
        }
    }

    /// Stable identifier used as the `kind` column in the transitions table.
    pub fn db_kind(self) -> &'static str {
        match self {
            PickerMode::Directories => "cd",
            PickerMode::Commands => "run",
            PickerMode::SshHosts => "ssh",
        }
    }

    /// Single-character type marker shown at the left of each row.
    pub fn sigil(self) -> &'static str {
        match self {
            PickerMode::Directories => "/",
            PickerMode::Commands => "$",
            PickerMode::SshHosts => "@",
        }
    }
}

use super::ranking::{base_score, interleave, MatchKind, RankInput, Scored};
use super::{Row, TypeFilter};
use crate::config::ScoringConfig;
use std::collections::HashMap;

pub struct UnifiedList {
    pub rows: Vec<Row>,
    /// Ranked, filtered view into `rows`. Rebuilt by `update_filter`.
    pub ranked: Vec<Scored>,
    pub filter: TypeFilter,
    pub selected: usize,
    pub scroll_offset: usize,
    pub visible_height: usize,
    pub cwd_filter: bool,
    pub cwd: Option<String>,
    /// (kind, target) -> decayed transition score, all originating at `cwd`.
    transitions: HashMap<(String, String), f64>,
    scoring: ScoringConfig,
    fuzzy: FuzzyMatcher,
    display_texts: Vec<String>,
    last_query: String,
}

impl UnifiedList {
    pub fn new(
        rows: Vec<Row>,
        cwd: Option<String>,
        transitions: HashMap<(String, String), f64>,
        scoring: ScoringConfig,
    ) -> Self {
        let display_texts = rows.iter().map(|r| r.entry.display.clone()).collect();
        let mut list = UnifiedList {
            rows,
            ranked: Vec::new(),
            filter: TypeFilter::All,
            selected: 0,
            scroll_offset: 0,
            visible_height: 0,
            cwd_filter: false,
            cwd,
            transitions,
            scoring,
            fuzzy: FuzzyMatcher::new(),
            display_texts,
            last_query: String::new(),
        };
        list.update_filter("");
        list
    }

    pub fn selected_row_index(&self) -> Option<usize> {
        self.ranked.get(self.selected).map(|s| s.row_idx)
    }

    pub fn selected_row(&self) -> Option<&Row> {
        self.selected_row_index().map(|i| &self.rows[i])
    }

    pub fn cycle_filter(&mut self) {
        self.filter = self.filter.next();
        self.selected = 0;
        self.scroll_offset = 0;
        let q = self.last_query.clone();
        self.update_filter(&q);
    }

    pub fn cycle_filter_back(&mut self) {
        self.filter = self.filter.prev();
        self.selected = 0;
        self.scroll_offset = 0;
        let q = self.last_query.clone();
        self.update_filter(&q);
    }

    pub fn set_cwd_filter(&mut self, on: bool) {
        self.cwd_filter = on;
        self.selected = 0;
        self.scroll_offset = 0;
        let q = self.last_query.clone();
        self.update_filter(&q);
    }

    /// True when this row is tied to the current directory: a command recorded
    /// here, or a directory/host we have jumped to from here.
    fn is_local(&self, row: &Row) -> bool {
        match row.kind {
            PickerMode::Commands => match (&self.cwd, &row.entry.cwd) {
                (Some(cur), Some(rc)) => cur == rc,
                _ => false,
            },
            _ => self.transition_score(row) > 0.0,
        }
    }

    fn transition_score(&self, row: &Row) -> f64 {
        if matches!(row.kind, PickerMode::Commands) {
            return 0.0;
        }
        self.transitions
            .get(&(row.kind.db_kind().to_string(), row.entry.value.clone()))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn update_filter(&mut self, query: &str) {
        self.last_query = query.to_string();
        let now = chrono::Utc::now().timestamp();
        let query_lower = query.to_lowercase();
        let matches = self.fuzzy.filter(query, &self.display_texts);

        let mut scored: Vec<Scored> = Vec::with_capacity(matches.len());
        for (idx, fuzzy_score, is_substring) in matches {
            let row = &self.rows[idx];
            if !self.filter.accepts(row.kind) {
                continue;
            }
            if self.cwd_filter && !self.is_local(row) {
                continue;
            }

            // An empty query matches everything with score 0 and is_substring
            // false. Mapping that to Fuzzy would penalize every row in the
            // opening view, so it must map to None.
            let match_kind = if query.is_empty() {
                MatchKind::None
            } else if self.display_texts[idx].to_lowercase().starts_with(&query_lower) {
                MatchKind::Prefix
            } else if is_substring {
                MatchKind::Substring
            } else {
                MatchKind::Fuzzy
            };

            let input = RankInput {
                kind: row.kind,
                display: &self.display_texts[idx],
                frecency: row.entry.score,
                last_time: row.entry.last_time,
                is_favorite: row.entry.is_favorite,
                cwd_match: matches!(row.kind, PickerMode::Commands) && self.is_local(row),
                transition_score: self.transition_score(row),
                match_kind,
                fuzzy_score,
            };
            let base = base_score(&input, &self.scoring, now);
            scored.push(Scored { row_idx: idx, kind: row.kind, base, score: base });
        }

        self.ranked = if self.filter == TypeFilter::All {
            interleave(scored, &self.scoring.type_bonus)
        } else {
            scored.sort_by(|a, b| b.base.cmp(&a.base).then(a.row_idx.cmp(&b.row_idx)));
            scored
        };

        if self.ranked.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.ranked.len() {
            self.selected = self.ranked.len() - 1;
        }
    }

    /// Drop a row from the backing store and rebuild the ranked view.
    pub fn remove_row(&mut self, row_idx: usize) {
        if row_idx >= self.rows.len() {
            return;
        }
        self.rows.remove(row_idx);
        self.display_texts.remove(row_idx);
        let q = self.last_query.clone();
        self.update_filter(&q);
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.ranked.is_empty() && self.selected < self.ranked.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn page_up(&mut self) {
        let ps = if self.visible_height > 1 { self.visible_height - 1 } else { 1 };
        self.selected = self.selected.saturating_sub(ps);
        self.scroll_offset = self.scroll_offset.saturating_sub(ps);
    }

    pub fn page_down(&mut self) {
        if self.ranked.is_empty() {
            return;
        }
        let ps = if self.visible_height > 1 { self.visible_height - 1 } else { 1 };
        self.selected = (self.selected + ps).min(self.ranked.len() - 1);
        let max_offset = self.ranked.len().saturating_sub(self.visible_height);
        self.scroll_offset = (self.scroll_offset + ps).min(max_offset);
    }

    pub fn ensure_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        self.visible_height = visible_height;
        let max_offset = self.ranked.len().saturating_sub(visible_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }

    /// Matched character positions for highlighting. Takes `&self` so it can be
    /// called while `rows` is borrowed for rendering; the matcher is built on
    /// the spot rather than reusing the `&mut self` scratch buffer.
    pub fn fuzzy_positions(&self, query: &str, text: &str) -> Vec<u32> {
        let mut m = FuzzyMatcher::new();
        m.match_positions(query, text)
    }
}

pub struct AppState {
    pub query: String,
    pub cursor: usize,
    pub list: UnifiedList,
    pub confirm_delete: bool,
    pub show_help: bool,
    pub show_config: Option<ConfigDialog>,
    pub flash_message: Option<(String, Instant)>,
    pub theme: Theme,
    pub config: Config,
    pub backup_config: Option<Config>,
    pub open_config_mode: bool,
    pub edit_dialog: Option<EditDialog>,
}

impl AppState {
    pub fn update_filter(&mut self) {
        let q = self.query.clone();
        self.list.update_filter(&q);
    }

    /// Rebuild theme from current config for live preview
    pub fn rebuild_theme(&mut self) {
        self.theme = Theme::from_config(&self.config);
    }

    #[cfg(test)]
    pub fn for_test() -> Self {
        let cfg = Config::default();
        AppState {
            query: String::new(),
            cursor: 0,
            list: UnifiedList::new(Vec::new(), None, HashMap::new(), cfg.scoring.clone()),
            confirm_delete: false,
            show_help: false,
            show_config: None,
            flash_message: None,
            theme: Theme::from_config(&cfg),
            config: cfg,
            backup_config: None,
            open_config_mode: false,
            edit_dialog: None,
        }
    }
}

pub fn run(
    initial_mode: Option<PickerMode>,
    store: &mut Store,
    initial_query: Option<String>,
    open_config: bool,
) -> Result<Option<PickResult>> {
    let tty_write = File::options()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| {
            anyhow::anyhow!(
                "Cannot open /dev/tty: {}. If using SSH, connect with `ssh -t` to allocate a pseudo-terminal.",
                e
            )
        })?;
    let mut tty_read = File::options()
        .read(true)
        .open("/dev/tty")?;

    let backend = CrosstermBackend::new(tty_write);
    let mut terminal = Terminal::new(backend)?;

    terminal::enable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        terminal::EnterAlternateScreen
    )?;
    // Enable kitty keyboard protocol (progressive enhancement, flags=1: disambiguate)
    // This makes Shift+Enter send CSI 13;2u instead of plain \r
    let _ = std::io::Write::write_all(terminal.backend_mut(), b"\x1b[>1u");

    let current_dir = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(String::from));

    let mut rows: Vec<Row> = Vec::new();
    for e in PickerEntry::from_dirs(store.list_directories()?) {
        rows.push(Row { kind: PickerMode::Directories, entry: e });
    }
    for e in PickerEntry::from_cmds(store.list_commands()?) {
        rows.push(Row { kind: PickerMode::Commands, entry: e });
    }
    for e in PickerEntry::from_ssh_hosts(store.list_ssh_hosts()?) {
        rows.push(Row { kind: PickerMode::SshHosts, entry: e });
    }

    let transitions = match &current_dir {
        Some(cwd) => store.transitions_from(cwd).unwrap_or_default(),
        None => HashMap::new(),
    };

    let cfg = Config::load();
    let theme = Theme::from_config(&cfg);

    let mut list = UnifiedList::new(rows, current_dir.clone(), transitions, cfg.scoring.clone());
    if let Some(mode) = initial_mode {
        list.filter = match mode {
            PickerMode::Directories => TypeFilter::Cd,
            PickerMode::Commands => TypeFilter::Run,
            PickerMode::SshHosts => TypeFilter::Ssh,
        };
        list.update_filter("");
    }

    let initial_q = initial_query.unwrap_or_default();
    let initial_cursor = initial_q.chars().count();
    let mut state = AppState {
        query: initial_q,
        cursor: initial_cursor,
        list,
        confirm_delete: false,
        show_help: false,
        show_config: if open_config {
            Some(ConfigDialog::from_config(&cfg))
        } else {
            None
        },
        flash_message: None,
        theme,
        config: cfg.clone(),
        backup_config: if open_config { Some(cfg) } else { None },
        open_config_mode: open_config,
        edit_dialog: None,
    };

    if !state.query.is_empty() {
        state.update_filter();
    }

    // Register SIGWINCH handler for terminal resize
    let resize_flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGWINCH, Arc::clone(&resize_flag))?;

    let result = (|| -> Result<Option<PickResult>> {
        // Initial clear to start with a clean screen
        terminal.clear()?;
        loop {
            terminal.draw(|f| draw(f, &mut state))?;

            let key = match super::tty_reader::poll_key(&mut tty_read, &resize_flag)? {
                PollResult::Key(k) => k,
                PollResult::Resize => {
                    // Auto-resize updates buffer to new terminal dimensions, then clear
                    terminal.autoresize()?;
                    terminal.clear()?;
                    continue;
                }
            };
            if state.confirm_delete {
                match handle_key(key, &mut state) {
                    Action::ConfirmDelete => {
                        state.confirm_delete = false;
                        if let Some(row_idx) = state.list.selected_row_index() {
                            let value = state.list.rows[row_idx].entry.value.clone();
                            let _ = store.forget(&value);
                            state.list.remove_row(row_idx);
                        }
                    }
                    _ => {
                        state.confirm_delete = false;
                    }
                }
            } else {
                match handle_key(key, &mut state) {
                    Action::Continue => {
                        state.update_filter();
                    }
                    action @ (Action::Select | Action::SelectEdit) => {
                        let edit = matches!(action, Action::SelectEdit);
                        if let Some(row) = state.list.selected_row() {
                            let entry = &row.entry;
                            let output = entry
                                .connect_value
                                .as_ref()
                                .unwrap_or(&entry.value)
                                .clone();
                            return Ok(Some(PickResult {
                                kind: row.kind,
                                output,
                                key: entry.value.clone(),
                                edit,
                            }));
                        }
                        return Ok(None);
                    }
                    Action::Quit => {
                        if state.open_config_mode && state.show_config.is_none() {
                            return Ok(None);
                        }
                        return Ok(None);
                    }
                    Action::CycleFilter => {
                        state.list.cycle_filter();
                    }
                    Action::CycleFilterBack => {
                        state.list.cycle_filter_back();
                    }
                    Action::ToggleCwdFilter => {
                        let on = !state.list.cwd_filter;
                        state.list.set_cwd_filter(on);
                    }
                    Action::CopyToClipboard => {
                        let text = state.list.selected_row().map(|row| {
                            row.entry
                                .connect_value
                                .as_ref()
                                .unwrap_or(&row.entry.value)
                                .clone()
                        });
                        if let Some(text) = text {
                            if copy_to_clipboard(&text).is_ok() {
                                state.flash_message = Some(("Copied!".to_string(), Instant::now()));
                            }
                        }
                    }
                    Action::ToggleFavorite => {
                        if let Some(row_idx) = state.list.selected_row_index() {
                            let row = &state.list.rows[row_idx];
                            let value = row.entry.value.clone();
                            let new_fav = match row.kind {
                                PickerMode::Directories => store.toggle_favorite_dir(&value),
                                PickerMode::Commands => store.toggle_favorite_cmd(&value),
                                PickerMode::SshHosts => store.toggle_favorite_ssh(&value),
                            };
                            if let Ok(fav) = new_fav {
                                state.list.rows[row_idx].entry.is_favorite = fav;
                            }
                        }
                    }
                    Action::Delete => {
                        if let Some(row_idx) = state.list.selected_row_index() {
                            if state.config.confirm_delete {
                                state.confirm_delete = true;
                            } else {
                                // Delete immediately without confirmation
                                let value = state.list.rows[row_idx].entry.value.clone();
                                let _ = store.forget(&value);
                                state.list.remove_row(row_idx);
                            }
                        }
                    }
                    Action::EditCommand => {
                        let text = state.list.selected_row().map(|row| {
                            row.entry
                                .connect_value
                                .as_ref()
                                .unwrap_or(&row.entry.value)
                                .clone()
                        });
                        if let Some(text) = text {
                            state.edit_dialog = Some(EditDialog::new(text));
                        }
                    }
                    Action::ExecuteEdit => {
                        if let Some(dialog) = state.edit_dialog.take() {
                            if let Some(row) = state.list.selected_row() {
                                return Ok(Some(PickResult {
                                    kind: row.kind,
                                    output: dialog.text.clone(),
                                    key: dialog.text,
                                    edit: false,
                                }));
                            }
                        }
                    }
                    Action::ToggleHelp => {
                        state.show_help = true;
                    }
                    Action::ToggleConfig => {
                        if state.show_config.is_some() {
                            // Cancel — restore backup config
                            if let Some(backup) = state.backup_config.take() {
                                state.config = backup;
                                state.rebuild_theme();
                            }
                            state.show_config = None;
                            if state.open_config_mode {
                                return Ok(None);
                            }
                        } else {
                            state.backup_config = Some(state.config.clone());
                            state.show_config = Some(ConfigDialog::from_config(&state.config));
                        }
                    }
                    Action::SaveConfig => {
                        if let Some(ref dialog) = state.show_config {
                            match dialog.to_config() {
                                Ok(new_cfg) => {
                                    match new_cfg.save() {
                                        Ok(()) => {
                                            state.config = new_cfg;
                                            state.rebuild_theme();
                                            state.backup_config = None;
                                            state.show_config = None;
                                            state.flash_message = Some(("Config saved!".to_string(), Instant::now()));
                                            if state.open_config_mode {
                                                return Ok(None);
                                            }
                                        }
                                        Err(e) => {
                                            state.flash_message = Some((format!("Save error: {}", e), Instant::now()));
                                        }
                                    }
                                }
                                Err(e) => {
                                    state.flash_message = Some((format!("Invalid: {}", e), Instant::now()));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    })();

    // Disable kitty keyboard protocol
    let _ = std::io::Write::write_all(terminal.backend_mut(), b"\x1b[<u");
    let _ = crossterm::execute!(
        terminal.backend_mut(),
        terminal::LeaveAlternateScreen
    );
    let _ = terminal::disable_raw_mode();

    result
}

#[cfg(test)]
mod unified_tests {
    use super::*;
    use crate::config::ScoringConfig;
    use crate::tui::{PickerEntry, Row, TypeFilter};
    use std::collections::HashMap;

    fn entry(display: &str, score: f64, last_time: i64) -> PickerEntry {
        PickerEntry {
            display: display.to_string(),
            value: display.to_string(),
            connect_value: None,
            score,
            is_favorite: false,
            last_time,
            use_count: 1,
            exists: None,
            duration_ms: None,
            cwd: None,
        }
    }

    fn row(kind: PickerMode, display: &str) -> Row {
        Row { kind, entry: entry(display, 1.0, 0) }
    }

    fn list(rows: Vec<Row>) -> UnifiedList {
        UnifiedList::new(rows, None, HashMap::new(), ScoringConfig::default())
    }

    #[test]
    fn test_empty_query_shows_all_rows() {
        let mut l = list(vec![
            row(PickerMode::Directories, "/a"),
            row(PickerMode::Commands, "ls"),
            row(PickerMode::SshHosts, "prod"),
        ]);
        l.update_filter("");
        assert_eq!(l.ranked.len(), 3);
    }

    #[test]
    fn test_empty_query_does_not_apply_fuzzy_penalty() {
        // Regression guard: an empty query must map to MatchKind::None, not Fuzzy.
        // "ls" alone would score 3000 - 2*15 = 2970 brevity + 100 frecency + 6000 recency.
        let mut l = UnifiedList::new(
            vec![Row { kind: PickerMode::Commands, entry: entry("ls", 1.0, i64::MAX / 2) }],
            None,
            HashMap::new(),
            ScoringConfig::default(),
        );
        l.update_filter("");
        assert!(
            l.ranked[0].base > 0,
            "empty-query rows must not be penalized as fuzzy matches, got {}",
            l.ranked[0].base
        );
    }

    #[test]
    fn test_query_filters_non_matching_rows() {
        let mut l = list(vec![
            row(PickerMode::Commands, "cargo build"),
            row(PickerMode::Commands, "npm install"),
        ]);
        l.update_filter("cargo");
        assert_eq!(l.ranked.len(), 1);
        assert_eq!(l.rows[l.ranked[0].row_idx].entry.display, "cargo build");
    }

    #[test]
    fn test_prefix_match_outranks_substring_match() {
        let mut l = list(vec![
            row(PickerMode::Commands, "xx git"),
            row(PickerMode::Commands, "git push"),
        ]);
        l.update_filter("git");
        assert_eq!(l.rows[l.ranked[0].row_idx].entry.display, "git push");
    }

    #[test]
    fn test_type_filter_restricts_rows() {
        let mut l = list(vec![
            row(PickerMode::Directories, "/a"),
            row(PickerMode::Commands, "ls"),
            row(PickerMode::SshHosts, "prod"),
        ]);
        l.filter = TypeFilter::Cd;
        l.update_filter("");
        assert_eq!(l.ranked.len(), 1);
        assert_eq!(l.rows[l.ranked[0].row_idx].kind, PickerMode::Directories);
    }

    #[test]
    fn test_type_filter_cycles_forward_and_back() {
        let mut l = list(vec![]);
        assert_eq!(l.filter, TypeFilter::All);
        l.cycle_filter();
        assert_eq!(l.filter, TypeFilter::Cd);
        l.cycle_filter();
        assert_eq!(l.filter, TypeFilter::Run);
        l.cycle_filter();
        assert_eq!(l.filter, TypeFilter::Ssh);
        l.cycle_filter();
        assert_eq!(l.filter, TypeFilter::All);
        l.cycle_filter_back();
        assert_eq!(l.filter, TypeFilter::Ssh);
    }

    #[test]
    fn test_filtered_view_skips_interleave() {
        // With a single type present the output is pure base-score order.
        let mut l = list(vec![
            row(PickerMode::Commands, "aaa"),
            row(PickerMode::Commands, "bb"),
        ]);
        l.filter = TypeFilter::Run;
        l.update_filter("");
        assert_eq!(
            l.ranked[0].score, l.ranked[0].base,
            "no type bonus should be applied in a filtered view"
        );
    }

    #[test]
    fn test_cwd_bonus_applies_only_to_commands() {
        let mut cmd = entry("ls", 1.0, 0);
        cmd.cwd = Some("/here".to_string());
        let mut l = UnifiedList::new(
            vec![
                Row { kind: PickerMode::Commands, entry: cmd },
                Row { kind: PickerMode::Commands, entry: entry("ls -l", 1.0, 0) },
            ],
            Some("/here".to_string()),
            HashMap::new(),
            ScoringConfig::default(),
        );
        l.update_filter("");
        let top = &l.rows[l.ranked[0].row_idx];
        assert_eq!(top.entry.display, "ls", "row recorded in this cwd ranks first");
    }

    #[test]
    fn test_transition_bonus_applies_to_cd_rows() {
        let mut t = HashMap::new();
        t.insert(("cd".to_string(), "/target".to_string()), 20.0);
        let mut l = UnifiedList::new(
            vec![
                Row { kind: PickerMode::Directories, entry: entry("/target", 1.0, 0) },
                Row { kind: PickerMode::Directories, entry: entry("/other", 1.0, 0) },
            ],
            Some("/here".to_string()),
            t,
            ScoringConfig::default(),
        );
        l.update_filter("");
        assert_eq!(l.rows[l.ranked[0].row_idx].entry.display, "/target");
    }

    #[test]
    fn test_transition_lookup_uses_value_not_display() {
        // SSH rows display "alias -> user@host" but the transition key is the DB value.
        let mut e = entry("prod -> root@1.2.3.4", 1.0, 0);
        e.value = "prod".to_string();
        let mut t = HashMap::new();
        t.insert(("ssh".to_string(), "prod".to_string()), 30.0);
        let mut l = UnifiedList::new(
            vec![
                Row { kind: PickerMode::SshHosts, entry: e },
                Row { kind: PickerMode::SshHosts, entry: entry("staging", 1.0, 0) },
            ],
            Some("/here".to_string()),
            t,
            ScoringConfig::default(),
        );
        l.update_filter("");
        assert_eq!(l.rows[l.ranked[0].row_idx].entry.value, "prod");
    }

    #[test]
    fn test_selection_clamps_when_results_shrink() {
        let mut l = list(vec![
            row(PickerMode::Commands, "aaa"),
            row(PickerMode::Commands, "aab"),
            row(PickerMode::Commands, "aac"),
        ]);
        l.update_filter("aa");
        l.selected = 2;
        l.update_filter("aab");
        assert_eq!(l.ranked.len(), 1);
        assert_eq!(l.selected, 0, "selection must clamp into range");
    }

    #[test]
    fn test_selection_resets_to_zero_when_nothing_matches() {
        let mut l = list(vec![row(PickerMode::Commands, "aaa")]);
        l.update_filter("zzzzz");
        assert!(l.ranked.is_empty());
        assert_eq!(l.selected, 0);
        assert!(l.selected_row().is_none());
    }

    #[test]
    fn test_selected_row_returns_kind_and_entry() {
        let mut l = list(vec![row(PickerMode::SshHosts, "prod")]);
        l.update_filter("");
        let r = l.selected_row().unwrap();
        assert_eq!(r.kind, PickerMode::SshHosts);
        assert_eq!(r.entry.value, "prod");
    }

    #[test]
    fn test_remove_row_reindexes_ranked() {
        let mut l = list(vec![
            row(PickerMode::Commands, "aaa"),
            row(PickerMode::Commands, "bbb"),
        ]);
        l.update_filter("");
        let victim = l.ranked[0].row_idx;
        let survivor = l.rows[l.ranked[1].row_idx].entry.display.clone();
        l.remove_row(victim);
        assert_eq!(l.rows.len(), 1);
        assert_eq!(l.ranked.len(), 1);
        assert_eq!(l.rows[l.ranked[0].row_idx].entry.display, survivor);
    }

    #[test]
    fn test_cwd_filter_narrows_all_three_types() {
        let mut cmd_here = entry("ls", 1.0, 0);
        cmd_here.cwd = Some("/here".to_string());
        let mut t = HashMap::new();
        t.insert(("cd".to_string(), "/target".to_string()), 5.0);
        let mut l = UnifiedList::new(
            vec![
                Row { kind: PickerMode::Commands, entry: cmd_here },
                Row { kind: PickerMode::Commands, entry: entry("pwd", 1.0, 0) },
                Row { kind: PickerMode::Directories, entry: entry("/target", 1.0, 0) },
                Row { kind: PickerMode::Directories, entry: entry("/elsewhere", 1.0, 0) },
            ],
            Some("/here".to_string()),
            t,
            ScoringConfig::default(),
        );
        l.set_cwd_filter(true);
        l.update_filter("");
        assert_eq!(l.ranked.len(), 2, "only the cwd-linked command and directory remain");
        let shown: Vec<String> = l
            .ranked
            .iter()
            .map(|r| l.rows[r.row_idx].entry.display.clone())
            .collect();
        assert!(shown.contains(&"ls".to_string()));
        assert!(shown.contains(&"/target".to_string()));
    }

    #[test]
    fn test_sigils_are_distinct_and_not_the_cursor() {
        let s = [
            PickerMode::Directories.sigil(),
            PickerMode::Commands.sigil(),
            PickerMode::SshHosts.sigil(),
        ];
        assert_eq!(s, ["/", "$", "@"]);
        assert!(!s.contains(&">"), "sigil must not collide with the selection cursor");
    }
}
