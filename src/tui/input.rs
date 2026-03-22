use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{Action, AppState};

pub fn handle_key(key: KeyEvent, state: &mut AppState) -> Action {
    // Config dialog intercepts all keys when open
    if state.show_config.is_some() {
        return handle_config_key_wrapper(key, state);
    }

    if state.show_help {
        state.show_help = false;
        return Action::Continue;
    }

    if state.confirm_delete {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmDelete,
            _ => Action::CancelDelete,
        };
    }

    match key.code {
        KeyCode::F(1) => Action::ToggleHelp,
        KeyCode::F(2) => Action::ToggleConfig,
        KeyCode::Esc => Action::Quit,
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => Action::SelectEdit,
        KeyCode::Enter => Action::Select,
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Action::SwitchPanelBack,
        KeyCode::Tab => Action::SwitchPanel,
        KeyCode::BackTab => Action::SwitchPanelBack,
        KeyCode::Up => {
            state.active_panel_mut().move_up();
            Action::Continue
        }
        KeyCode::Down => {
            state.active_panel_mut().move_down();
            Action::Continue
        }
        KeyCode::PageUp => {
            state.active_panel_mut().page_up();
            Action::Continue
        }
        KeyCode::PageDown => {
            state.active_panel_mut().page_down();
            Action::Continue
        }
        KeyCode::Backspace => {
            if state.cursor > 0 {
                let byte_pos = state.query.char_indices()
                    .nth(state.cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                state.query.remove(byte_pos);
                state.cursor -= 1;
            }
            Action::Continue
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.query.clear();
            state.cursor = 0;
            Action::Continue
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.active_panel_mut().move_up();
            Action::Continue
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.active_panel_mut().move_down();
            Action::Continue
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ToggleFavorite
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::Delete if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Delete,
        KeyCode::Delete => {
            let len = state.query.chars().count();
            if state.cursor < len {
                let byte_pos = state.query.char_indices()
                    .nth(state.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(state.query.len());
                state.query.remove(byte_pos);
            } else {
                // No char at cursor position — act as entry delete
                return Action::Delete;
            }
            Action::Continue
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::CopyToClipboard
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ToggleCwdFilter
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.cursor = 0;
            Action::Continue
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.cursor = state.query.chars().count();
            Action::Continue
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.cursor > 0 {
                state.cursor -= 1;
            }
            Action::Continue
        }
        KeyCode::Left => {
            if state.cursor > 0 {
                state.cursor -= 1;
            }
            Action::Continue
        }
        KeyCode::Right => {
            if state.cursor < state.query.chars().count() {
                state.cursor += 1;
            }
            Action::Continue
        }
        KeyCode::Home => {
            state.cursor = 0;
            Action::Continue
        }
        KeyCode::End => {
            state.cursor = state.query.chars().count();
            Action::Continue
        }
        KeyCode::Char(c) if c.is_control() => {
            // Filter out non-printable characters
            Action::Continue
        }
        KeyCode::Char(c) => {
            let byte_pos = state.query.char_indices()
                .nth(state.cursor)
                .map(|(i, _)| i)
                .unwrap_or(state.query.len());
            state.query.insert(byte_pos, c);
            state.cursor += 1;
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn handle_config_key_wrapper(key: KeyEvent, state: &mut AppState) -> Action {
    let dialog = state.show_config.as_mut().unwrap();

    let action = match key.code {
        KeyCode::Esc => {
            if dialog.handle_escape() {
                return Action::ToggleConfig;
            }
            Action::Continue
        }
        KeyCode::F(2) => Action::SaveConfig,
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::SaveConfig,
        KeyCode::Up => { dialog.move_up(); Action::Continue }
        KeyCode::Down => { dialog.move_down(); Action::Continue }
        KeyCode::Tab => { dialog.move_down(); Action::Continue }
        KeyCode::BackTab => { dialog.move_up(); Action::Continue }
        KeyCode::Left => { dialog.handle_left(); Action::Continue }
        KeyCode::Right => { dialog.handle_right(); Action::Continue }
        KeyCode::Enter | KeyCode::Char(' ') => { dialog.handle_enter(); Action::Continue }
        KeyCode::Backspace => { dialog.handle_backspace(); Action::Continue }
        KeyCode::Delete => { dialog.handle_delete(); Action::Continue }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.handle_delete(); Action::Continue }
        KeyCode::Char(c) if !c.is_control() => { dialog.handle_char(c); Action::Continue }
        _ => Action::Continue,
    };

    // Live preview
    if let Some(ref dialog) = state.show_config {
        if dialog.dirty {
            if let Ok(preview_cfg) = dialog.to_config() {
                state.config = preview_cfg;
                state.rebuild_theme();
            }
        }
    }

    action
}
