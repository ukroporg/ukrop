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
