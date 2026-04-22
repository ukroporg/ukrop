use std::time::Duration;
use chrono::{TimeZone, Utc};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::app::{AppState, PanelState, PickerMode};
use super::config_dialog::{ConfigDialog, FieldKind};
use super::edit_dialog::EditDialog;
use super::theme::Theme;

fn format_relative_time(ts: i64) -> String {
    let now = Utc::now().timestamp();
    let delta = now - ts;
    if delta < 60 {
        "just now".to_string()
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86400 {
        format!("{}h ago", delta / 3600)
    } else if delta < 86400 * 30 {
        format!("{}d ago", delta / 86400)
    } else if delta < 86400 * 365 {
        let months = delta / (86400 * 30);
        format!("{}mo ago", months)
    } else {
        Utc.timestamp_opt(ts, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    }
}

fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

fn truncate_path(text: &str, max_width: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_width || max_width < 4 {
        return text.to_string();
    }
    let skip = char_count - (max_width - 3);
    let suffix: String = text.chars().skip(skip).collect();
    format!("...{}", suffix)
}

fn truncate_command(text: &str, max_width: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_width || max_width < 14 {
        return text.to_string();
    }
    // show first 10 chars + "..." + end
    let tail_len = max_width - 10 - 3;
    let head: String = text.chars().take(10).collect();
    let tail: String = text.chars().skip(char_count - tail_len).collect();
    format!("{}...{}", head, tail)
}

fn wrap_command(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    if max_width == 0 || max_lines == 0 {
        return vec![text.to_string()];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lines = Vec::new();
    let mut pos = 0;
    while pos < chars.len() && lines.len() < max_lines {
        let end = (pos + max_width).min(chars.len());
        lines.push(chars[pos..end].iter().collect());
        pos = end;
    }
    // If there are remaining chars after max_lines, truncate last line
    if pos < chars.len() && !lines.is_empty() {
        let last = lines.len() - 1;
        let remaining: String = chars[pos..].iter().collect();
        let current = &lines[last];
        let combined = format!("{}{}", current, remaining);
        lines[last] = truncate_command(&combined, max_width);
    }
    lines
}

/// Split text into spans, highlighting characters at the given positions.
fn highlighted_spans(text: &str, positions: &[u32], base_style: Style, hl_style: Style) -> Vec<Span<'static>> {
    if positions.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut buf = String::new();
    let pos_set: std::collections::HashSet<u32> = positions.iter().copied().collect();

    let mut in_highlight = false;
    for (i, ch) in chars.iter().enumerate() {
        let is_match = pos_set.contains(&(i as u32));
        if is_match != in_highlight {
            if !buf.is_empty() {
                let style = if in_highlight { hl_style } else { base_style };
                spans.push(Span::styled(std::mem::take(&mut buf), style));
            }
            in_highlight = is_match;
        }
        buf.push(*ch);
    }
    if !buf.is_empty() {
        let style = if in_highlight { hl_style } else { base_style };
        spans.push(Span::styled(buf, style));
    }
    spans
}

/// Pad a Line with trailing spaces so it fills the given total width.
fn pad_line(line: Line<'static>, total_width: usize) -> Line<'static> {
    let content_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    if content_width < total_width {
        let mut spans: Vec<Span<'static>> = line.spans.into_iter().collect();
        spans.push(Span::raw(" ".repeat(total_width - content_width)));
        Line::from(spans)
    } else {
        line
    }
}

fn draw_panel(f: &mut Frame, area: ratatui::layout::Rect, panel: &mut PanelState, is_active: bool, query: &str, theme: &Theme, cwd_filter: bool) {
    let col_width = area.width as usize;
    // 2 borders + 2 selector + 2 fav marker = 6 chars overhead
    let text_max_width = col_width.saturating_sub(6);
    let inner_width = col_width.saturating_sub(2); // inside borders

    let border_style = if is_active {
        theme.border_active
    } else {
        theme.border_inactive
    };

    let title_style = if is_active {
        theme.border_active.add_modifier(Modifier::BOLD)
    } else {
        theme.border_inactive
    };

    let count = panel.filtered_indices.len();
    let total = panel.display_texts.len();
    let cwd_tag = if cwd_filter && panel.mode == PickerMode::Commands { " [cwd]" } else { "" };
    let title = format!(" {} ({}/{}){}  ", panel.mode.label(), count, total, cwd_tag);

    let is_cmd_panel = panel.mode == PickerMode::Commands;
    // overhead for continuation lines: 2 borders + 2 selector + 2 fav = 6, but continuation has blank prefix
    let prefix_width = 4; // "> " or "  " (2) + fav marker (2) = 4

    let list_items: Vec<ListItem> = panel
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, (item_idx, _match_score, _is_substr))| {
            let item = &panel.items[*item_idx];
            let is_selected = is_active && display_idx == panel.selected;
            let fav_marker = if item.is_favorite { "* " } else { "  " };

            let not_exists = item.exists == Some(false);

            let style = theme.item_style(is_selected, not_exists, item.last_time);
            let hl_style = theme.highlight_style(style);

            let fav_style = if item.is_favorite {
                theme.favorite
            } else {
                style
            };

            let selector = if is_selected { "> " } else { "  " };

            if is_cmd_panel && is_selected && item.display.chars().count() > text_max_width {
                // Selected command: show multiline, max 5 rows
                let lines = wrap_command(&item.display, text_max_width, 5);
                let full_text: String = lines.concat();
                let positions = panel.fuzzy.match_positions(query, &full_text);
                let mut spans_lines: Vec<Line> = Vec::new();
                // Remap positions to wrapped lines
                let mut char_offset = 0usize;
                for (i, line_text) in lines.iter().enumerate() {
                    let line_len = line_text.chars().count();
                    let line_positions: Vec<u32> = positions
                        .iter()
                        .filter(|&&p| (p as usize) >= char_offset && (p as usize) < char_offset + line_len)
                        .map(|&p| p - char_offset as u32)
                        .collect();
                    let text_spans = highlighted_spans(line_text, &line_positions, style, hl_style);
                    if i == 0 {
                        let mut line_spans = vec![
                            Span::styled(selector, theme.selected),
                            Span::styled(fav_marker, fav_style),
                        ];
                        line_spans.extend(text_spans);
                        spans_lines.push(pad_line(Line::from(line_spans), inner_width));
                    } else {
                        let pad = " ".repeat(prefix_width);
                        let mut line_spans = vec![Span::raw(pad)];
                        line_spans.extend(text_spans);
                        spans_lines.push(pad_line(Line::from(line_spans), inner_width));
                    }
                    char_offset += line_len;
                }
                ListItem::new(spans_lines)
            } else {
                let text = if is_cmd_panel {
                    truncate_command(&item.display, text_max_width)
                } else {
                    truncate_path(&item.display, text_max_width)
                };

                // Compute match positions on the displayed (truncated) text
                let positions = panel.fuzzy.match_positions(query, &text);
                let text_spans = highlighted_spans(&text, &positions, style, hl_style);
                let mut line_spans = vec![
                    Span::styled(selector, theme.selected),
                    Span::styled(fav_marker, fav_style),
                ];
                line_spans.extend(text_spans);
                ListItem::new(pad_line(Line::from(line_spans), inner_width))
            }
        })
        .collect();

    if list_items.is_empty() {
        let msg = if panel.display_texts.is_empty() {
            "  (empty)"
        } else {
            "  No matches"
        };
        let no_match = Paragraph::new(Line::from(vec![
            Span::styled(msg, theme.age_old),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, title_style)),
        );
        f.render_widget(no_match, area);
    } else {
        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(title, title_style)),
        );

        let visible_height = area.height.saturating_sub(2) as usize; // minus top+bottom borders
        panel.ensure_visible(visible_height);

        let mut list_state = ListState::default();
        if is_active {
            list_state.select(Some(panel.selected));
        }
        *list_state.offset_mut() = panel.scroll_offset;
        f.render_stateful_widget(list, area, &mut list_state);
    }
}

pub fn draw(f: &mut Frame, state: &mut AppState) {
    f.render_widget(Clear, f.area());

    let theme = &state.theme;
    let compact = f.area().height < 40;
    let has_preview = !compact && state.active_panel().selected_index().is_some();
    let preview_height = if has_preview { 3 } else { 0 };
    let status_height: u16 = if compact { 0 } else { 1 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                  // search bar
            Constraint::Min(1),                    // panels
            Constraint::Length(preview_height),     // preview
            Constraint::Length(status_height),      // status bar
        ])
        .split(f.area());

    // Search bar
    let search_title = if state.confirm_delete {
        "ukrop — Delete? (y/n)".to_string()
    } else {
        "ukrop".to_string()
    };
    let query_chars: Vec<char> = state.query.chars().collect();
    let cursor_pos = state.cursor;
    let before_cursor: String = query_chars[..cursor_pos].iter().collect();
    let cursor_char: String = query_chars.get(cursor_pos).map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
    let after_cursor: String = if cursor_pos < query_chars.len() { query_chars[cursor_pos + 1..].iter().collect() } else { String::new() };
    let search = Paragraph::new(Line::from(vec![
        Span::styled("> ", theme.prompt),
        Span::raw(before_cursor),
        Span::styled(cursor_char, theme.cursor),
        Span::raw(after_cursor),
    ]))
    .block(Block::default().borders(Borders::ALL).title(search_title));
    f.render_widget(search, chunks[0]);

    // Two columns: left (configurable%) and right
    let left_pct = theme.left_panel_pct;
    let right_pct = 100u16.saturating_sub(left_pct);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(right_pct),
        ])
        .split(chunks[1]);

    // Left column: cd (configurable%) on top, ssh on bottom
    let cd_pct = theme.cd_panel_pct;
    let ssh_pct = 100u16.saturating_sub(cd_pct);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(cd_pct),
            Constraint::Percentage(ssh_pct),
        ])
        .split(cols[0]);

    let active = state.active;
    let query = state.query.clone();
    // Need to borrow theme fields before mutable borrow of panels
    let border_active = state.theme.border_active;
    let border_inactive = state.theme.border_inactive;
    let prompt = state.theme.prompt;
    let cursor_style = state.theme.cursor;
    let selected_style = state.theme.selected;
    let age_recent = state.theme.age_recent;
    let age_mid = state.theme.age_mid;
    let age_old = state.theme.age_old;
    let missing = state.theme.missing;
    let highlight_modifier = state.theme.highlight_modifier;
    let highlight_fg = state.theme.highlight_fg;
    let favorite = state.theme.favorite;
    let flash_style = state.theme.flash;
    let status_hint = state.theme.status_hint;

    // Create a temporary theme copy for panel drawing
    let theme_copy = Theme {
        border_active,
        border_inactive,
        prompt,
        cursor: cursor_style,
        selected: selected_style,
        age_recent,
        age_mid,
        age_old,
        missing,
        highlight_modifier,
        highlight_fg,
        favorite,
        section_header: state.theme.section_header,
        status_hint,
        flash: flash_style,
        dialog_border: state.theme.dialog_border,
        dialog_key: state.theme.dialog_key,
        dialog_desc: state.theme.dialog_desc,
        left_panel_pct: state.theme.left_panel_pct,
        cd_panel_pct: state.theme.cd_panel_pct,
    };

    let cwd_filter = state.cwd_filter;
    draw_panel(f, left[0], &mut state.panels[0], active == 0, &query, &theme_copy, false);        // cd
    draw_panel(f, left[1], &mut state.panels[2], active == 2, &query, &theme_copy, false);        // ssh
    draw_panel(f, cols[1], &mut state.panels[1], active == 1, &query, &theme_copy, cwd_filter);   // run

    // Preview bar for selected item in active panel (hidden in compact mode)
    let active_panel = state.active_panel();
    if has_preview {
    if let Some(idx) = active_panel.selected_index() {
        let item = &active_panel.items[idx];
        let exists_str = match item.exists {
            Some(true) => Span::styled(" exists", Style::default().fg(Color::White)),
            Some(false) => Span::styled(" MISSING", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            None => Span::raw(""),
        };
        let (label, count_label) = match active_panel.mode {
            PickerMode::SshHosts => ("  Host: ", "  │  Connections: "),
            PickerMode::Commands => ("  Cmd: ", "  │  Uses: "),
            PickerMode::Directories => ("  Path: ", "  │  Visits: "),
        };
        let mut spans = vec![
            Span::styled(label, Style::default().fg(Color::DarkGray)),
            Span::raw(&item.value),
            Span::styled(count_label, Style::default().fg(Color::DarkGray)),
            Span::raw(item.use_count.to_string()),
            Span::styled("  │  Last: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format_relative_time(item.last_time)),
        ];
        if let Some(ms) = item.duration_ms {
            spans.push(Span::styled("  │  Duration: ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::raw(format_duration(ms)));
        }
        spans.push(Span::styled("  │ ", Style::default().fg(Color::DarkGray)));
        spans.push(exists_str);
        let detail = Line::from(spans);
        let preview = Paragraph::new(detail)
            .block(Block::default().borders(Borders::TOP).title("Details"));
        f.render_widget(preview, chunks[2]);
    }
    }

    // Status bar — show flash message or default hints (hidden in compact mode)
    if !compact {
        let show_flash = state.flash_message.as_ref()
            .map(|(_, t)| t.elapsed() < Duration::from_millis(1500))
            .unwrap_or(false);
        if !show_flash {
            state.flash_message = None;
        }

        let status_line = if let Some((msg, _)) = &state.flash_message {
            Line::from(vec![
                Span::styled(format!(" {}", msg), flash_style),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    " F1 help  F2 edit  F5 paste  F8 del  F9 config  Tab  Up/Down  Enter run  ^Y copy  ^F fav  ^W cwd  Esc quit",
                    status_hint,
                ),
            ])
        };
        let status = Paragraph::new(status_line);
        f.render_widget(status, chunks[3]);
    }

    // Config dialog overlay
    if let Some(ref dialog) = state.show_config {
        draw_config(f, dialog, &theme_copy);
    }

    // Edit dialog overlay
    if let Some(ref dialog) = state.edit_dialog {
        draw_edit_dialog(f, dialog, &theme_copy);
    }

    // Help overlay
    if state.show_help {
        draw_help(f, &theme_copy);
    }
}

fn draw_config(f: &mut Frame, dialog: &ConfigDialog, theme: &Theme) {
    let area = f.area();
    let dialog_width: u16 = 56;
    // Calculate height: header + sections + fields + footer
    let content_lines = count_dialog_lines(dialog);
    let dialog_height: u16 = (content_lines as u16 + 4).min(area.height); // +4 for borders and footer
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let popup = ratatui::layout::Rect::new(x, y, dialog_width.min(area.width), dialog_height);

    f.render_widget(Clear, popup);

    let config_path_str = crate::config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "config.toml".to_string());

    let title = format!(" Config ({}) ", shorten_path(&config_path_str, 40));

    let mut lines: Vec<Line> = Vec::new();
    let inner_width = (dialog_width as usize).saturating_sub(4);

    let mut row = 0usize;
    let mut last_section = "";

    for field in &dialog.fields {
        // Section header
        if field.section != last_section {
            if !last_section.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", field.section),
                theme.section_header,
            )));
            last_section = field.section;
        }

        match &field.kind {
            FieldKind::Float { value } | FieldKind::Uint { value } => {
                let is_focused = dialog.focused == row;
                let val_style = if is_focused {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    theme.dialog_desc
                };
                let label_padded = format!("    {:18} ", format!("{}:", field.label));
                lines.push(Line::from(vec![
                    Span::styled(label_padded, theme.dialog_desc),
                    Span::styled(format!("{:<14}", value), val_style),
                ]));
                row += 1;
            }
            FieldKind::Enum { options, selected } => {
                let is_focused = dialog.focused == row;
                let val = options[*selected];
                let val_display = if is_focused {
                    format!("< {} >", val)
                } else {
                    val.to_string()
                };
                let val_style = if is_focused {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    theme.dialog_desc
                };
                let label_padded = format!("    {:18} ", format!("{}:", field.label));
                lines.push(Line::from(vec![
                    Span::styled(label_padded, theme.dialog_desc),
                    Span::styled(format!("{:<14}", val_display), val_style),
                ]));
                row += 1;
            }
            FieldKind::Bool { value } => {
                let is_focused = dialog.focused == row;
                let check = if *value { "[x]" } else { "[ ]" };
                let val_style = if is_focused {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    theme.dialog_desc
                };
                let label_padded = format!("    {:18} ", format!("{}:", field.label));
                lines.push(Line::from(vec![
                    Span::styled(label_padded, theme.dialog_desc),
                    Span::styled(check, val_style),
                ]));
                row += 1;
            }
            FieldKind::StringList { items, editing_buf, .. } => {
                row += 1; // label row (section header already printed)
                for (i, item) in items.iter().enumerate() {
                    let is_focused = dialog.focused == row;
                    let display_text = if is_focused {
                        if let Some(buf) = editing_buf {
                            format!("    {}. [{}_]", i + 1, buf)
                        } else {
                            format!("    {}. {}          [Del]", i + 1, item)
                        }
                    } else {
                        format!("    {}. {}", i + 1, item)
                    };
                    let st = if is_focused {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else {
                        theme.dialog_desc
                    };
                    lines.push(Line::from(Span::styled(
                        truncate_line(&display_text, inner_width),
                        st,
                    )));
                    row += 1;
                }
                // "add new" row
                let is_focused = dialog.focused == row;
                let add_text = if is_focused {
                    if let Some(buf) = editing_buf {
                        format!("    [new: {}_]", buf)
                    } else {
                        "    [+ Add new pattern]".to_string()
                    }
                } else {
                    "    [+ Add new pattern]".to_string()
                };
                let st = if is_focused {
                    Style::default().fg(Color::Black).bg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(Span::styled(
                    truncate_line(&add_text, inner_width),
                    st,
                )));
                row += 1;
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  F9", theme.dialog_key),
        Span::styled("/", theme.dialog_desc),
        Span::styled("^S", theme.dialog_key),
        Span::styled(" save   ", theme.dialog_desc),
        Span::styled("Esc", theme.dialog_key),
        Span::styled(" cancel   ", theme.dialog_desc),
        Span::styled("↑↓", theme.dialog_key),
        Span::styled(" navigate", theme.dialog_desc),
    ]));

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.dialog_border)
            .title(Span::styled(title, theme.section_header)),
    );
    f.render_widget(help, popup);
}

fn count_dialog_lines(dialog: &ConfigDialog) -> usize {
    let mut lines = 0;
    let mut last_section = "";
    for field in &dialog.fields {
        if field.section != last_section {
            if !last_section.is_empty() {
                lines += 1; // blank line between sections
            }
            lines += 1; // section header
            last_section = field.section;
        }
        match &field.kind {
            FieldKind::StringList { items, .. } => {
                lines += items.len() + 1; // items + "add new"
            }
            _ => lines += 1,
        }
    }
    lines += 2; // footer
    lines
}

fn truncate_line(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        text.chars().take(max).collect()
    }
}

fn shorten_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_string();
    }
    if let Some(home) = dirs::home_dir() {
        let home_str = home.display().to_string();
        if let Some(rest) = path.strip_prefix(&home_str) {
            let shortened = format!("~{}", rest);
            if shortened.chars().count() <= max {
                return shortened;
            }
        }
    }
    truncate_path(path, max)
}

fn draw_edit_dialog(f: &mut Frame, dialog: &EditDialog, theme: &Theme) {
    let area = f.area();
    let dialog_width: u16 = 72.min(area.width.saturating_sub(4));
    let dialog_height: u16 = 14.min(area.height.saturating_sub(2)); // 10 content + 2 borders + 2 footer
    let x = area.width.saturating_sub(dialog_width) / 2;
    let y = area.height.saturating_sub(dialog_height) / 2;
    let popup = ratatui::layout::Rect::new(x, y, dialog_width, dialog_height);

    f.render_widget(Clear, popup);

    let inner_width = (dialog_width as usize).saturating_sub(4); // borders + padding
    let content_height = (dialog_height as usize).saturating_sub(4); // borders + footer

    // Wrap the command text into lines for display
    let chars: Vec<char> = dialog.text.chars().collect();
    let mut lines: Vec<Line> = Vec::new();

    // Split by explicit newlines first, then wrap each line
    let text_lines: Vec<String> = dialog.text.split('\n').map(|s| s.to_string()).collect();
    let mut wrapped: Vec<String> = Vec::new();
    let mut cursor_line = 0;
    let mut cursor_col = 0;
    let mut char_pos = 0;
    for (li, tline) in text_lines.iter().enumerate() {
        let line_chars: Vec<char> = tline.chars().collect();
        if line_chars.is_empty() {
            if dialog.cursor == char_pos {
                cursor_line = wrapped.len();
                cursor_col = 0;
            }
            wrapped.push(String::new());
        } else {
            let mut pos = 0;
            while pos < line_chars.len() {
                let end = (pos + inner_width).min(line_chars.len());
                let seg: String = line_chars[pos..end].iter().collect();
                let abs_start = char_pos + pos;
                let abs_end = char_pos + end;
                if dialog.cursor >= abs_start && dialog.cursor < abs_end {
                    cursor_line = wrapped.len();
                    cursor_col = dialog.cursor - abs_start;
                } else if dialog.cursor == abs_end && end == line_chars.len() && li == text_lines.len() - 1 {
                    cursor_line = wrapped.len();
                    cursor_col = end - pos;
                }
                wrapped.push(seg);
                pos = end;
            }
        }
        char_pos += line_chars.len() + 1; // +1 for the newline
    }
    // Handle cursor at very end (after last newline)
    if dialog.cursor == chars.len() && !text_lines.is_empty() {
        let last_line = text_lines.last().unwrap();
        if last_line.is_empty() || dialog.text.ends_with('\n') {
            // cursor is on a new empty line after trailing newline
            if dialog.text.ends_with('\n') && dialog.cursor == chars.len() {
                cursor_line = wrapped.len().saturating_sub(1);
                cursor_col = 0;
                // Add empty line if text ends with newline and cursor is at end
                if !wrapped.last().map(|s| s.is_empty()).unwrap_or(false) {
                    wrapped.push(String::new());
                    cursor_line = wrapped.len() - 1;
                }
            }
        } else {
            cursor_line = wrapped.len().saturating_sub(1);
            cursor_col = wrapped.last().map(|s| s.chars().count()).unwrap_or(0);
        }
    }
    if wrapped.is_empty() {
        wrapped.push(String::new());
    }

    let text_style = theme.dialog_desc;
    let cursor_style = Style::default().fg(Color::Black).bg(Color::White);

    for (i, line_text) in wrapped.iter().enumerate() {
        if i >= content_height {
            break;
        }
        if i == cursor_line {
            // Render this line with the cursor highlighted
            let line_chars: Vec<char> = line_text.chars().collect();
            let before_c: String = line_chars[..cursor_col.min(line_chars.len())].iter().collect();
            let c_ch: String = line_chars.get(cursor_col).map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
            let after_c: String = if cursor_col < line_chars.len() { line_chars[cursor_col + 1..].iter().collect() } else { String::new() };
            lines.push(Line::from(vec![
                Span::styled("  ", text_style),
                Span::styled(before_c, text_style),
                Span::styled(c_ch, cursor_style),
                Span::styled(after_c, text_style),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ", text_style),
                Span::styled(line_text.clone(), text_style),
            ]));
        }
    }

    // Pad remaining lines
    while lines.len() < content_height {
        lines.push(Line::from(""));
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  F5", theme.dialog_key),
        Span::styled(" execute   ", theme.dialog_desc),
        Span::styled("Enter", theme.dialog_key),
        Span::styled(" newline   ", theme.dialog_desc),
        Span::styled("Esc", theme.dialog_key),
        Span::styled(" cancel", theme.dialog_desc),
    ]));

    let edit = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.dialog_border)
            .title(Span::styled(" Edit Command ", theme.section_header)),
    );
    f.render_widget(edit, popup);
}

fn draw_help(f: &mut Frame, theme: &Theme) {
    let area = f.area();
    let help_width: u16 = 54;
    let help_height: u16 = 24;
    let x = area.width.saturating_sub(help_width) / 2;
    let y = area.height.saturating_sub(help_height) / 2;
    let popup = ratatui::layout::Rect::new(x, y, help_width.min(area.width), help_height.min(area.height));

    f.render_widget(Clear, popup);

    let key_style = theme.dialog_key;
    let desc_style = theme.dialog_desc;
    let header_style = theme.section_header;

    let lines = vec![
        Line::from(Span::styled(" Keyboard Shortcuts", header_style)),
        Line::from(""),
        Line::from(vec![Span::styled("  Enter       ", key_style), Span::styled("Select and run", desc_style)]),
        Line::from(vec![Span::styled("  S+Enter/F5  ", key_style), Span::styled("Paste to terminal for editing", desc_style)]),
        Line::from(vec![Span::styled("  Esc         ", key_style), Span::styled("Quit", desc_style)]),
        Line::from(vec![Span::styled("  Tab         ", key_style), Span::styled("Next panel", desc_style)]),
        Line::from(vec![Span::styled("  Shift+Tab   ", key_style), Span::styled("Previous panel", desc_style)]),
        Line::from(vec![Span::styled("  Up/Down     ", key_style), Span::styled("Navigate list", desc_style)]),
        Line::from(vec![Span::styled("  PgUp/PgDn   ", key_style), Span::styled("Scroll page", desc_style)]),
        Line::from(vec![Span::styled("  Left/Right  ", key_style), Span::styled("Move cursor in search bar", desc_style)]),
        Line::from(vec![Span::styled("  Home/End    ", key_style), Span::styled("Cursor to start/end of search", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+A/E    ", key_style), Span::styled("Cursor to start/end of search", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+W      ", key_style), Span::styled("Toggle CWD filter (run panel)", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+P/N    ", key_style), Span::styled("Navigate up/down", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+Y      ", key_style), Span::styled("Copy to clipboard", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+F      ", key_style), Span::styled("Toggle favorite", desc_style)]),
        Line::from(vec![Span::styled("  F8/Ctrl+Del ", key_style), Span::styled("Delete entry", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+U      ", key_style), Span::styled("Clear search", desc_style)]),
        Line::from(vec![Span::styled("  Ctrl+C/D    ", key_style), Span::styled("Quit", desc_style)]),
        Line::from(vec![Span::styled("  F2          ", key_style), Span::styled("Edit selected command", desc_style)]),
        Line::from(vec![Span::styled("  F9          ", key_style), Span::styled("Open config editor", desc_style)]),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.dialog_border)
            .title(Span::styled(" Help ", theme.section_header)),
    );
    f.render_widget(help, popup);
}
