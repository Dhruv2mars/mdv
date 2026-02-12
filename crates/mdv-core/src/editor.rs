use std::fs;
use std::io;
use std::path::Path;

use crate::conflict_diff::{ConflictHunk, compute_conflict_hunks};

const MAX_HISTORY_ENTRIES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictState {
    pub external: String,
    pub hunks: Vec<ConflictHunk>,
}

#[derive(Debug, Clone)]
pub struct EditorBuffer {
    text: String,
    cursor: usize,
    pub dirty: bool,
    conflict: Option<ConflictState>,
    undo_stack: Vec<HistoryState>,
    redo_stack: Vec<HistoryState>,
}

#[derive(Debug, Clone)]
struct HistoryState {
    text: String,
    cursor: usize,
    dirty: bool,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
        self.push_undo_snapshot();
        self.redo_stack.clear();
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
        self.push_undo_snapshot();
        self.redo_stack.clear();
        let prev = self.prev_char_boundary(self.cursor);
        self.text.replace_range(prev..self.cursor, "");
        self.cursor = prev;
        self.dirty = true;
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        let snapshot = self.snapshot();
        Self::push_history(&mut self.redo_stack, snapshot);
        self.restore(prev);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        let snapshot = self.snapshot();
        Self::push_history(&mut self.undo_stack, snapshot);
        self.restore(next);
        true
    }

    pub fn find_next(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }

        let start = if self.cursor >= self.text.len() {
            0
        } else {
            self.next_char_boundary(self.cursor)
        };

        if let Some(offset) = self.text[start..].find(needle) {
            self.cursor = start + offset;
            return true;
        }

        if let Some(offset) = self.text[..start].find(needle) {
            self.cursor = offset;
            return true;
        }

        false
    }

    pub fn find_prev(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }

        let start = if self.cursor == 0 {
            self.text.len()
        } else {
            self.prev_char_boundary(self.cursor)
        };

        if let Some(offset) = self.text[..start].rfind(needle) {
            self.cursor = offset;
            return true;
        }

        if let Some(offset) = self.text[start..].rfind(needle) {
            self.cursor = start + offset;
            return true;
        }

        false
    }

    pub fn replace_next(&mut self, needle: &str, replacement: &str) -> bool {
        if needle.is_empty() {
            return false;
        }

        let start = if self.cursor >= self.text.len() {
            0
        } else {
            self.next_char_boundary(self.cursor)
        };

        let found = self.text[start..]
            .find(needle)
            .map(|offset| start + offset)
            .or_else(|| self.text[..start].find(needle));

        let Some(match_start) = found else {
            return false;
        };

        self.push_undo_snapshot();
        self.redo_stack.clear();
        let match_end = match_start + needle.len();
        self.text.replace_range(match_start..match_end, replacement);
        self.cursor = match_start + replacement.len();
        self.dirty = true;
        true
    }

    pub fn replace_all(&mut self, needle: &str, replacement: &str) -> usize {
        if needle.is_empty() {
            return 0;
        }
        if !self.text.contains(needle) {
            return 0;
        }

        let count = self.text.matches(needle).count();
        self.push_undo_snapshot();
        self.redo_stack.clear();
        self.text = self.text.replace(needle, replacement);
        self.cursor = self.text.len();
        self.dirty = true;
        count
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

    pub fn goto_line(&mut self, line_number: usize) -> bool {
        if line_number == 0 {
            return false;
        }
        let total_lines = self.text.split('\n').count().max(1);
        if line_number > total_lines {
            return false;
        }
        self.cursor = self.index_at_line_col(line_number - 1, 0);
        true
    }

    pub fn on_external_change(&mut self, external: String) {
        if self.dirty {
            self.conflict = Some(ConflictState {
                hunks: compute_conflict_hunks(&self.text, &external),
                external,
            });
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

    pub fn apply_external_hunk(&mut self, hunk_index: usize) -> bool {
        let Some(conflict) = self.conflict.clone() else {
            return false;
        };
        let Some(hunk) = conflict.hunks.get(hunk_index).cloned() else {
            return false;
        };

        let mut lines: Vec<String> = self.text.split('\n').map(ToString::to_string).collect();
        let start = hunk.local_start.min(lines.len());
        let end = start
            .saturating_add(hunk.local_lines.len())
            .min(lines.len());
        lines.splice(start..end, hunk.external_lines);
        self.text = lines.join("\n");
        self.cursor = self.index_at_line_col(start, 0);
        self.dirty = true;

        let external = conflict.external;
        let hunks = compute_conflict_hunks(&self.text, &external);
        if hunks.is_empty() {
            self.conflict = None;
        } else {
            self.conflict = Some(ConflictState { external, hunks });
        }

        true
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

    fn snapshot(&self) -> HistoryState {
        HistoryState {
            text: self.text.clone(),
            cursor: self.cursor,
            dirty: self.dirty,
            conflict: self.conflict.clone(),
        }
    }

    fn restore(&mut self, state: HistoryState) {
        self.text = state.text;
        self.cursor = state.cursor;
        self.dirty = state.dirty;
        self.conflict = state.conflict;
    }

    fn push_undo_snapshot(&mut self) {
        let snapshot = self.snapshot();
        Self::push_history(&mut self.undo_stack, snapshot);
    }

    fn push_history(stack: &mut Vec<HistoryState>, state: HistoryState) {
        stack.push(state);
        if stack.len() > MAX_HISTORY_ENTRIES {
            let overflow = stack.len() - MAX_HISTORY_ENTRIES;
            stack.drain(0..overflow);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{EditorBuffer, MAX_HISTORY_ENTRIES};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("mdv-editor-test-{name}-{nanos}.md"))
    }

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

    #[test]
    fn no_op_movement_and_backspace_at_edges() {
        let mut buf = EditorBuffer::new(String::new());
        assert_eq!(buf.cursor(), 0);
        buf.backspace();
        buf.move_left();
        buf.move_right();
        buf.move_up();
        buf.move_down();
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn insert_newline_and_unicode_movement() {
        let mut buf = EditorBuffer::new("é".into());
        buf.insert_newline();
        assert_eq!(buf.text(), "é\n");
        buf.move_left();
        buf.move_left();
        assert_eq!(buf.cursor(), 0);
        buf.move_right();
        assert_eq!(buf.cursor(), "é".len());
    }

    #[test]
    fn move_up_to_shorter_line_clamps_to_newline() {
        let mut buf = EditorBuffer::new("x\nlonger".into());
        buf.move_left();
        buf.move_left();
        buf.move_left();
        buf.move_up();
        let (line, col) = buf.line_col_at_cursor();
        assert_eq!(line, 0);
        assert_eq!(col, 1);
    }

    #[test]
    fn move_down_to_shorter_last_line_clamps_to_end() {
        let mut buf = EditorBuffer::new("abcd\ne".into());
        buf.move_up();
        buf.move_right();
        buf.move_right();
        buf.move_right();
        assert_eq!(buf.line_col_at_cursor(), (0, 4));
        buf.move_down();
        assert_eq!(buf.line_col_at_cursor(), (1, 1));
    }

    #[test]
    fn merge_external_noop_when_not_conflicted() {
        let mut buf = EditorBuffer::new("x".into());
        buf.merge_external();
        assert_eq!(buf.text(), "x");
    }

    #[test]
    fn reload_external_noop_when_not_conflicted() {
        let mut buf = EditorBuffer::new("x".into());
        buf.reload_external();
        assert_eq!(buf.text(), "x");
    }

    #[test]
    fn save_to_path_persists_text_and_clears_state() {
        let path = temp_path("save");
        let mut buf = EditorBuffer::new("a".into());
        buf.insert_char('b');
        buf.on_external_change("external".into());
        assert!(buf.is_conflicted());

        buf.save_to_path(&path).expect("save");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "ab");
        assert!(!buf.dirty);
        assert!(!buf.is_conflicted());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_to_path_returns_error_for_directory() {
        let path = temp_path("save-dir");
        std::fs::create_dir(&path).expect("mkdir");
        let mut buf = EditorBuffer::new("abc".into());
        let err = buf.save_to_path(&path).expect_err("save error");
        assert!(!err.to_string().is_empty());
        let _ = std::fs::remove_dir(&path);
    }

    #[test]
    fn undo_redo_restores_text_and_cursor() {
        let mut buf = EditorBuffer::new("ab".into());
        assert_eq!(buf.cursor(), 2);

        buf.insert_char('c');
        assert_eq!(buf.text(), "abc");
        assert_eq!(buf.cursor(), 3);

        assert!(buf.undo());
        assert_eq!(buf.text(), "ab");
        assert_eq!(buf.cursor(), 2);

        assert!(buf.redo());
        assert_eq!(buf.text(), "abc");
        assert_eq!(buf.cursor(), 3);
    }

    #[test]
    fn redo_cleared_after_new_edit() {
        let mut buf = EditorBuffer::new("ab".into());
        buf.insert_char('c');
        assert!(buf.undo());
        assert_eq!(buf.text(), "ab");

        buf.insert_char('x');
        assert_eq!(buf.text(), "abx");
        assert!(!buf.redo());
        assert_eq!(buf.text(), "abx");
    }

    #[test]
    fn find_next_wraps_and_advances() {
        let mut buf = EditorBuffer::new("alpha beta alpha".into());
        assert!(buf.find_next("alpha"));
        assert_eq!(buf.cursor(), 0);

        assert!(buf.find_next("alpha"));
        assert_eq!(buf.cursor(), 11);

        assert!(!buf.find_next("zzz"));
        assert!(!buf.find_next(""));
    }

    #[test]
    fn find_prev_wraps_and_rewinds() {
        let mut buf = EditorBuffer::new("alpha beta alpha".into());
        buf.goto_line(1);
        assert!(buf.find_prev("alpha"));
        assert_eq!(buf.cursor(), 11);

        assert!(buf.find_prev("alpha"));
        assert_eq!(buf.cursor(), 0);

        assert!(!buf.find_prev("zzz"));
        assert!(!buf.find_prev(""));
    }

    #[test]
    fn replace_next_replaces_match_and_tracks_undo() {
        let mut buf = EditorBuffer::new("one two one".into());
        assert!(buf.replace_next("one", "ONE"));
        assert_eq!(buf.text(), "ONE two one");
        assert_eq!(buf.cursor(), 3);
        assert!(buf.dirty);

        assert!(buf.undo());
        assert_eq!(buf.text(), "one two one");

        assert!(!buf.replace_next("", "x"));
        assert!(!buf.replace_next("zzz", "x"));
    }

    #[test]
    fn replace_all_replaces_all_matches_and_handles_empty_query() {
        let mut buf = EditorBuffer::new("ab ab ab".into());
        assert_eq!(buf.replace_all("ab", "xy"), 3);
        assert_eq!(buf.text(), "xy xy xy");
        assert_eq!(buf.cursor(), buf.text().len());

        assert_eq!(buf.replace_all("", "noop"), 0);
        assert_eq!(buf.replace_all("zzz", "noop"), 0);
    }

    #[test]
    fn apply_external_hunk_updates_text_and_resolves_when_done() {
        let mut buf = EditorBuffer::new("a\nb\nc".into());
        buf.dirty = true;
        buf.on_external_change("a\nB\nc\nd".into());
        assert!(buf.is_conflicted());
        assert_eq!(buf.conflict().expect("conflict").hunks.len(), 2);

        assert!(buf.apply_external_hunk(1));
        assert!(buf.text().contains("\nd"));
        assert!(buf.is_conflicted());
        assert_eq!(buf.conflict().expect("conflict").hunks.len(), 1);

        assert!(buf.apply_external_hunk(0));
        assert_eq!(buf.text(), "a\nB\nc\nd");
        assert!(buf.dirty);
        assert!(!buf.is_conflicted());
    }

    #[test]
    fn apply_external_hunk_returns_false_for_invalid_index_or_no_conflict() {
        let mut buf = EditorBuffer::new("x".into());
        assert!(!buf.apply_external_hunk(0));

        buf.insert_char('!');
        buf.on_external_change("y".into());
        assert!(buf.is_conflicted());
        assert!(!buf.apply_external_hunk(99));
    }

    #[test]
    fn goto_line_jumps_and_bounds() {
        let mut buf = EditorBuffer::new("a\nb\nc".into());
        assert!(buf.goto_line(2));
        assert_eq!(buf.line_col_at_cursor(), (1, 0));

        assert!(buf.goto_line(3));
        assert_eq!(buf.line_col_at_cursor(), (2, 0));

        assert!(!buf.goto_line(0));
        assert!(!buf.goto_line(99));
        assert_eq!(buf.line_col_at_cursor(), (2, 0));
    }

    #[test]
    fn undo_history_is_bounded() {
        let mut buf = EditorBuffer::new(String::new());
        for _ in 0..300 {
            buf.insert_char('x');
        }

        let mut undo_count = 0usize;
        while buf.undo() {
            undo_count += 1;
        }
        assert_eq!(undo_count, MAX_HISTORY_ENTRIES);
    }
}
