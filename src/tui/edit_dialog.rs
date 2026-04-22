pub struct EditDialog {
    pub text: String,
    pub cursor: usize,
}

impl EditDialog {
    pub fn new(text: String) -> Self {
        let cursor = text.chars().count();
        EditDialog { text, cursor }
    }

    pub fn insert(&mut self, c: char) {
        let byte_pos = self.text.char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        self.text.insert(byte_pos, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let byte_pos = self.text.char_indices()
                .nth(self.cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.text.remove(byte_pos);
            self.cursor -= 1;
        }
    }

    pub fn delete(&mut self) {
        let len = self.text.chars().count();
        if self.cursor < len {
            let byte_pos = self.text.char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.text.len());
            self.text.remove(byte_pos);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.text.chars().count() {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        // Find current line start and column
        let chars: Vec<char> = self.text.chars().collect();
        let before_cursor: String = chars[..self.cursor].iter().collect();
        if let Some(nl_pos) = before_cursor.rfind('\n') {
            // There is a previous line
            let col = self.cursor - before_cursor[..nl_pos].chars().count() - 1;
            let prev_before: &str = &before_cursor[..nl_pos];
            let prev_line_start = prev_before.rfind('\n').map(|p| prev_before[..p].chars().count() + 1).unwrap_or(0);
            let prev_line_len = prev_before[prev_before.rfind('\n').map(|p| p + 1).unwrap_or(0)..].chars().count();
            self.cursor = prev_line_start + col.min(prev_line_len);
        }
        // else: already on first line, do nothing
    }

    pub fn move_down(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let before_cursor: String = chars[..self.cursor].iter().collect();
        let col = if let Some(nl_pos) = before_cursor.rfind('\n') {
            self.cursor - before_cursor[..nl_pos].chars().count() - 1
        } else {
            self.cursor
        };
        // Find end of current line
        let after_cursor: String = chars[self.cursor..].iter().collect();
        if let Some(nl_offset) = after_cursor.find('\n') {
            let next_line_start = self.cursor + after_cursor[..nl_offset].chars().count() + 1;
            let remaining: String = chars[next_line_start..].iter().collect();
            let next_line_len = remaining.find('\n').map(|p| remaining[..p].chars().count()).unwrap_or(remaining.chars().count());
            self.cursor = next_line_start + col.min(next_line_len);
        }
        // else: already on last line, do nothing
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.text.chars().count();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }
}
