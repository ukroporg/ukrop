use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{Action, AppState};

pub fn handle_key(key: KeyEvent, state: &mut AppState) -> Action {
    // Edit dialog intercepts all keys when open
    if let Some(ref mut dialog) = state.edit_dialog {
        return match key.code {
            KeyCode::Esc => {
                state.edit_dialog = None;
                Action::Continue
            }
            KeyCode::F(5) => Action::ExecuteEdit,
            KeyCode::Enter => { dialog.insert('\n'); Action::Continue }
            KeyCode::Up => { dialog.move_up(); Action::Continue }
            KeyCode::Down => { dialog.move_down(); Action::Continue }
            KeyCode::Left => { dialog.move_left(); Action::Continue }
            KeyCode::Right => { dialog.move_right(); Action::Continue }
            KeyCode::Home => { dialog.move_home(); Action::Continue }
            KeyCode::End => { dialog.move_end(); Action::Continue }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.move_home(); Action::Continue }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.move_end(); Action::Continue }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.move_left(); Action::Continue }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.move_right(); Action::Continue }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.move_up(); Action::Continue }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.move_down(); Action::Continue }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => { dialog.clear(); Action::Continue }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                state.edit_dialog = None;
                Action::Continue
            }
            KeyCode::Backspace => { dialog.backspace(); Action::Continue }
            KeyCode::Delete => { dialog.delete(); Action::Continue }
            KeyCode::Char(c) if !c.is_control() => { dialog.insert(c); Action::Continue }
            _ => Action::Continue,
        };
    }

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
        KeyCode::F(2) => Action::EditCommand,
        KeyCode::F(5) => Action::SelectEdit,
        KeyCode::F(9) => Action::ToggleConfig,
        KeyCode::Esc => Action::Quit,
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => Action::SelectEdit,
        KeyCode::Enter => Action::Select,
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Action::CycleFilterBack,
        KeyCode::Tab => Action::CycleFilter,
        KeyCode::BackTab => Action::CycleFilterBack,
        KeyCode::Up => {
            state.list.move_up();
            Action::Continue
        }
        KeyCode::Down => {
            state.list.move_down();
            Action::Continue
        }
        KeyCode::PageUp => {
            state.list.page_up();
            Action::Continue
        }
        KeyCode::PageDown => {
            state.list.page_down();
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
            state.list.move_up();
            Action::Continue
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.list.move_down();
            Action::Continue
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::ToggleFavorite
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
        KeyCode::F(8) => Action::Delete,
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
        KeyCode::F(9) => Action::SaveConfig,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // `app` is a private module, but `input` is its sibling inside `tui`,
    // so reach it via `super::` rather than an absolute `crate::tui::app` path.
    fn state() -> super::super::app::AppState {
        super::super::app::AppState::for_test()
    }

    #[test]
    fn test_tab_cycles_filter_forward() {
        let mut s = state();
        let a = handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &mut s);
        assert!(matches!(a, Action::CycleFilter));
    }

    #[test]
    fn test_shift_tab_cycles_filter_back() {
        let mut s = state();
        let a = handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE), &mut s);
        assert!(matches!(a, Action::CycleFilterBack));
    }

    #[test]
    fn test_enter_selects() {
        let mut s = state();
        let a = handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut s);
        assert!(matches!(a, Action::Select));
    }

    #[test]
    fn test_ctrl_w_toggles_cwd_filter() {
        let mut s = state();
        let a = handle_key(
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            &mut s,
        );
        assert!(matches!(a, Action::ToggleCwdFilter));
    }
}
