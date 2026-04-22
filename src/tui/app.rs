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
use super::PickerEntry;
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
    SwitchPanel,
    SwitchPanelBack,
    ToggleCwdFilter,
    ToggleHelp,
    ToggleConfig,
    SaveConfig,
    EditCommand,
    ExecuteEdit,
}

#[derive(Clone, Copy, PartialEq)]
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
}

pub struct PanelState {
    pub mode: PickerMode,
    pub items: Vec<PickerEntry>,
    pub display_texts: Vec<String>,
    pub filtered_indices: Vec<(usize, u32, bool)>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub fuzzy: FuzzyMatcher,
    pub visible_height: usize,
    /// Current working directory, used for cwd bonus in Commands panel.
    pub cwd: Option<String>,
}

impl PanelState {
    fn new(mode: PickerMode, items: Vec<PickerEntry>) -> Self {
        let display_texts: Vec<String> = items.iter().map(|i| i.display.clone()).collect();
        let mut fuzzy = FuzzyMatcher::new();
        let filtered_indices = fuzzy.filter("", &display_texts);
        PanelState {
            mode,
            items,
            display_texts,
            filtered_indices,
            selected: 0,
            scroll_offset: 0,
            fuzzy,
            visible_height: 0,
            cwd: None,
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).map(|(i, _, _)| *i)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
        }
    }

    #[allow(dead_code)]
    pub fn move_to_first(&mut self) {
        self.selected = 0;
    }

    #[allow(dead_code)]
    pub fn move_to_last(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn page_up(&mut self) {
        let ps = if self.visible_height > 1 { self.visible_height - 1 } else { 1 };
        self.selected = self.selected.saturating_sub(ps);
        self.scroll_offset = self.scroll_offset.saturating_sub(ps);
    }

    pub fn page_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            let ps = if self.visible_height > 1 { self.visible_height - 1 } else { 1 };
            let max = self.filtered_indices.len() - 1;
            self.selected = (self.selected + ps).min(max);
            let max_offset = self.filtered_indices.len().saturating_sub(self.visible_height);
            self.scroll_offset = (self.scroll_offset + ps).min(max_offset);
        }
    }

    /// Adjust scroll_offset so that `selected` is visible, with minimal scrolling.
    pub fn ensure_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        self.visible_height = visible_height;
        // Cap scroll_offset so we don't scroll past the last screenful
        let max_offset = self.filtered_indices.len().saturating_sub(visible_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected - visible_height + 1;
        }
    }

    pub fn update_filter(&mut self, query: &str) {
        self.filtered_indices = self.fuzzy.filter(query, &self.display_texts);

        if !query.is_empty() {
            let query_lower = query.to_lowercase();
            for entry in &mut self.filtered_indices {
                let idx = entry.0;
                let fuzzy_score = entry.1;
                let is_substring = entry.2;

                // Prefix bonus: big boost if display text starts with the query
                let prefix_bonus: u32 = if self.display_texts[idx]
                    .to_lowercase()
                    .starts_with(&query_lower)
                {
                    10_000
                } else {
                    0
                };

                // Substring bonus: substring matches rank above fuzzy-only matches
                let substring_bonus: u32 = if is_substring { 8_000 } else { 0 };

                // Frecency bonus: scale DB score (typically 1-50) into ranking range
                let frecency_bonus = (self.items[idx].score * 100.0).min(5_000.0) as u32;

                // Brevity bonus: shorter commands rank higher (max 3000 for very short commands)
                let len = self.display_texts[idx].len() as u32;
                let brevity_bonus: u32 = 3_000u32.saturating_sub(len * 15);

                // CWD bonus: commands previously run in the current directory rank higher
                let cwd_bonus: u32 = match (&self.cwd, &self.items[idx].cwd) {
                    (Some(current), Some(item_cwd)) if current == item_cwd => 4_000,
                    _ => 0,
                };

                entry.1 = fuzzy_score + prefix_bonus + substring_bonus + frecency_bonus + brevity_bonus + cwd_bonus;
            }
            self.filtered_indices.sort_by(|a, b| b.1.cmp(&a.1));
        }

        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

}

pub struct AppState {
    pub query: String,
    pub cursor: usize,
    pub panels: [PanelState; 3],
    pub active: usize,
    pub confirm_delete: bool,
    pub cwd_filter: bool,
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
    pub fn active_panel(&self) -> &PanelState {
        &self.panels[self.active]
    }

    pub fn active_panel_mut(&mut self) -> &mut PanelState {
        &mut self.panels[self.active]
    }

    pub fn update_all_filters(&mut self) {
        let query = self.query.clone();
        for panel in &mut self.panels {
            panel.update_filter(&query);
        }
    }

    pub fn switch_panel(&mut self) {
        self.active = (self.active + 1) % 3;
    }

    pub fn switch_panel_back(&mut self) {
        self.active = (self.active + 2) % 3;
    }

    /// Rebuild theme from current config for live preview
    pub fn rebuild_theme(&mut self) {
        self.theme = Theme::from_config(&self.config);
    }
}

fn load_items(mode: PickerMode, store: &mut Store) -> Result<Vec<PickerEntry>> {
    Ok(match mode {
        PickerMode::Directories => PickerEntry::from_dirs(store.list_directories()?),
        PickerMode::Commands => PickerEntry::from_cmds(store.list_commands()?),
        PickerMode::SshHosts => PickerEntry::from_ssh_hosts(store.list_ssh_hosts()?),
    })
}

pub fn run(
    initial_mode: PickerMode,
    store: &mut Store,
    initial_query: Option<String>,
    open_config: bool,
) -> Result<Option<(PickerMode, String, bool)>> {
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

    let dir_items = load_items(PickerMode::Directories, store)?;
    let cmd_items = load_items(PickerMode::Commands, store)?;
    let ssh_items = load_items(PickerMode::SshHosts, store)?;

    let active = match initial_mode {
        PickerMode::Directories => 0,
        PickerMode::Commands => 1,
        PickerMode::SshHosts => 2,
    };

    let cfg = Config::load();
    let theme = Theme::from_config(&cfg);

    let initial_q = initial_query.unwrap_or_default();
    let initial_cursor = initial_q.chars().count();
    let mut state = AppState {
        query: initial_q,
        cursor: initial_cursor,
        panels: {
            let mut cmd_panel = PanelState::new(PickerMode::Commands, cmd_items);
            cmd_panel.cwd = current_dir.clone();
            [
                PanelState::new(PickerMode::Directories, dir_items),
                cmd_panel,
                PanelState::new(PickerMode::SshHosts, ssh_items),
            ]
        },
        active,
        confirm_delete: false,
        cwd_filter: false,
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
        state.update_all_filters();
    }

    // Register SIGWINCH handler for terminal resize
    let resize_flag = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGWINCH, Arc::clone(&resize_flag))?;

    let result = (|| -> Result<Option<(PickerMode, String, bool)>> {
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
                        let panel = state.active_panel_mut();
                        if let Some(idx) = panel.selected_index() {
                            let value = &panel.items[idx].value;
                            let _ = store.forget(value);
                            panel.items.remove(idx);
                            panel.display_texts =
                                panel.items.iter().map(|i| i.display.clone()).collect();
                            let query = state.query.clone();
                            state.active_panel_mut().update_filter(&query);
                        }
                    }
                    _ => {
                        state.confirm_delete = false;
                    }
                }
            } else {
                match handle_key(key, &mut state) {
                    Action::Continue => {
                        state.update_all_filters();
                    }
                    action @ (Action::Select | Action::SelectEdit) => {
                        let edit = matches!(action, Action::SelectEdit);
                        let panel = state.active_panel();
                        if let Some(idx) = panel.selected_index() {
                            let entry = &panel.items[idx];
                            let output = entry.connect_value.as_ref()
                                .unwrap_or(&entry.value)
                                .clone();
                            return Ok(Some((panel.mode, output, edit)));
                        }
                        return Ok(None);
                    }
                    Action::Quit => {
                        if state.open_config_mode && state.show_config.is_none() {
                            return Ok(None);
                        }
                        return Ok(None);
                    }
                    Action::SwitchPanel => {
                        state.switch_panel();
                    }
                    Action::SwitchPanelBack => {
                        state.switch_panel_back();
                    }
                    Action::ToggleCwdFilter => {
                        state.cwd_filter = !state.cwd_filter;
                        let new_cmds = if state.cwd_filter {
                            if let Some(ref cwd) = current_dir {
                                PickerEntry::from_cmds(store.list_commands_by_cwd(cwd)?)
                            } else {
                                PickerEntry::from_cmds(store.list_commands()?)
                            }
                        } else {
                            PickerEntry::from_cmds(store.list_commands()?)
                        };
                        let panel = &mut state.panels[1];
                        panel.items = new_cmds;
                        panel.display_texts = panel.items.iter().map(|i| i.display.clone()).collect();
                        panel.selected = 0;
                        panel.scroll_offset = 0;
                        let query = state.query.clone();
                        panel.update_filter(&query);
                    }
                    Action::CopyToClipboard => {
                        let panel = state.active_panel();
                        if let Some(idx) = panel.selected_index() {
                            let entry = &panel.items[idx];
                            let text = entry.connect_value.as_ref()
                                .unwrap_or(&entry.value);
                            if copy_to_clipboard(text).is_ok() {
                                state.flash_message = Some(("Copied!".to_string(), Instant::now()));
                            }
                        }
                    }
                    Action::ToggleFavorite => {
                        let panel = state.active_panel_mut();
                        if let Some(idx) = panel.selected_index() {
                            let value = &panel.items[idx].value;
                            let new_fav = match panel.mode {
                                PickerMode::Directories => store.toggle_favorite_dir(value),
                                PickerMode::Commands => store.toggle_favorite_cmd(value),
                                PickerMode::SshHosts => store.toggle_favorite_ssh(value),
                            };
                            if let Ok(fav) = new_fav {
                                panel.items[idx].is_favorite = fav;
                            }
                        }
                    }
                    Action::Delete => {
                        if state.active_panel().selected_index().is_some() {
                            if state.config.confirm_delete {
                                state.confirm_delete = true;
                            } else {
                                // Delete immediately without confirmation
                                let panel = state.active_panel_mut();
                                if let Some(idx) = panel.selected_index() {
                                    let value = &panel.items[idx].value;
                                    let _ = store.forget(value);
                                    panel.items.remove(idx);
                                    panel.display_texts =
                                        panel.items.iter().map(|i| i.display.clone()).collect();
                                    let query = state.query.clone();
                                    state.active_panel_mut().update_filter(&query);
                                }
                            }
                        }
                    }
                    Action::EditCommand => {
                        let panel = state.active_panel();
                        if let Some(idx) = panel.selected_index() {
                            let entry = &panel.items[idx];
                            let text = entry.connect_value.as_ref()
                                .unwrap_or(&entry.value)
                                .clone();
                            state.edit_dialog = Some(EditDialog::new(text));
                        }
                    }
                    Action::ExecuteEdit => {
                        if let Some(dialog) = state.edit_dialog.take() {
                            let panel = state.active_panel();
                            return Ok(Some((panel.mode, dialog.text, false)));
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
