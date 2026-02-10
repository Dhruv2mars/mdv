use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictState {
    pub external: String,
}

#[derive(Debug, Clone)]
pub struct EditorBuffer {
    text: String,
    cursor: usize,
    pub dirty: bool,
    conflict: Option<ConflictState>,
}

impl EditorBuffer {
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            dirty: false,
            conflict: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn conflict(&self) -> Option<&ConflictState> {
        self.conflict.as_ref()
    }

    pub fn is_conflicted(&self) -> bool {
        self.conflict.is_some()
    }

    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.dirty = true;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.prev_char_boundary(self.cursor);
        self.text.replace_range(prev..self.cursor, "");
        self.cursor = prev;
        self.dirty = true;
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.prev_char_boundary(self.cursor);
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        self.cursor = self.next_char_boundary(self.cursor);
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.line_col_at(self.cursor);
        if line == 0 {
            return;
        }
        self.cursor = self.index_at_line_col(line - 1, col);
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.line_col_at(self.cursor);
        let total_lines = self.text.lines().count().max(1);
        if line + 1 >= total_lines {
            return;
        }
        self.cursor = self.index_at_line_col(line + 1, col);
    }

    pub fn line_col_at_cursor(&self) -> (usize, usize) {
        self.line_col_at(self.cursor)
    }

    pub fn on_external_change(&mut self, external: String) {
        if self.dirty {
            self.conflict = Some(ConflictState { external });
            return;
        }
        self.set_from_disk(external);
    }

    pub fn keep_local(&mut self) {
        self.conflict = None;
    }

    pub fn reload_external(&mut self) {
        if let Some(conflict) = self.conflict.take() {
            self.set_from_disk(conflict.external);
        }
    }

    pub fn merge_external(&mut self) {
        let Some(conflict) = self.conflict.take() else {
            return;
        };

        self.text = format!(
            "<<<<<<< local\n{}\n=======\n{}\n>>>>>>> external\n",
            self.text, conflict.external
        );
        self.cursor = self.text.len();
        self.dirty = true;
    }

    pub fn save_to_path(&mut self, path: &Path) -> io::Result<()> {
        fs::write(path, &self.text)?;
        self.dirty = false;
        self.conflict = None;
        Ok(())
    }

    fn set_from_disk(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.len();
        self.dirty = false;
        self.conflict = None;
    }

    fn line_col_at(&self, byte_index: usize) -> (usize, usize) {
        let clamped = byte_index.min(self.text.len());
        let mut line = 0usize;
        let mut col = 0usize;
        for (idx, ch) in self.text.char_indices() {
            if idx >= clamped {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn index_at_line_col(&self, target_line: usize, target_col: usize) -> usize {
        let mut line = 0usize;
        let mut col = 0usize;

        for (idx, ch) in self.text.char_indices() {
            if line == target_line && col == target_col {
                return idx;
            }
            if ch == '\n' {
                if line == target_line {
                    return idx;
                }
                line += 1;
                col = 0;
                if line > target_line {
                    return idx;
                }
            } else {
                col += 1;
            }
        }

        self.text.len()
    }

    fn prev_char_boundary(&self, i: usize) -> usize {
        let mut idx = i.saturating_sub(1);
        while idx > 0 && !self.text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    fn next_char_boundary(&self, i: usize) -> usize {
        let mut idx = (i + 1).min(self.text.len());
        while idx < self.text.len() && !self.text.is_char_boundary(idx) {
            idx += 1;
        }
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::EditorBuffer;

    #[test]
    fn insert_and_backspace_work() {
        let mut buf = EditorBuffer::new("abc".into());
        buf.insert_char('d');
        assert_eq!(buf.text(), "abcd");
        assert!(buf.dirty);

        buf.backspace();
        assert_eq!(buf.text(), "abc");
    }

    #[test]
    fn external_change_reloads_when_clean() {
        let mut buf = EditorBuffer::new("old".into());
        buf.on_external_change("new".into());
        assert_eq!(buf.text(), "new");
        assert!(!buf.dirty);
        assert!(!buf.is_conflicted());
    }

    #[test]
    fn external_change_sets_conflict_when_dirty() {
        let mut buf = EditorBuffer::new("old".into());
        buf.insert_char('!');
        buf.on_external_change("new".into());

        assert_eq!(buf.text(), "old!");
        assert!(buf.is_conflicted());
        assert_eq!(buf.conflict().expect("conflict").external, "new");

        buf.keep_local();
        assert!(!buf.is_conflicted());
        assert_eq!(buf.text(), "old!");
    }

    #[test]
    fn reload_external_accepts_disk_state() {
        let mut buf = EditorBuffer::new("old".into());
        buf.insert_char('!');
        buf.on_external_change("disk".into());
        buf.reload_external();

        assert_eq!(buf.text(), "disk");
        assert!(!buf.dirty);
        assert!(!buf.is_conflicted());
    }

    #[test]
    fn merge_external_creates_conflict_markers() {
        let mut buf = EditorBuffer::new("local".into());
        buf.insert_char('!');
        buf.on_external_change("external".into());
        buf.merge_external();

        assert!(buf.text().contains("<<<<<<< local"));
        assert!(buf.text().contains("======="));
        assert!(buf.text().contains(">>>>>>> external"));
        assert!(!buf.is_conflicted());
        assert!(buf.dirty);
    }

    #[test]
    fn move_up_down_keeps_column_when_possible() {
        let mut buf = EditorBuffer::new("ab\n1234\nxy".into());
        buf.move_left();
        buf.move_left();
        let (_, col_before) = buf.line_col_at_cursor();

        buf.move_up();
        let (_, col_up) = buf.line_col_at_cursor();
        assert_eq!(col_before, col_up);

        buf.move_down();
        let (_, col_down) = buf.line_col_at_cursor();
        assert_eq!(col_before, col_down);
    }
}
