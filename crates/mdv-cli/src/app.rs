use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use mdv_core::{EditorBuffer, render_preview_lines};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

#[cfg(not(test))]
use crate::stream;
use crate::stream::StreamMessage;
use crate::watcher::{self, WatchMessage};

pub struct App {
    path: Option<PathBuf>,
    readonly: bool,
    watch_enabled: bool,
    stream_mode: bool,
    perf_mode: bool,
    editor: EditorBuffer,
    status: String,
    _watcher: Option<notify::RecommendedWatcher>,
    watch_rx: Option<std::sync::mpsc::Receiver<WatchMessage>>,
    stream_rx: Option<std::sync::mpsc::Receiver<StreamMessage>>,
    editor_scroll: usize,
    preview_scroll: usize,
    editor_height: usize,
    draw_time_us: u128,
    watch_event_count: u64,
    stream_event_count: u64,
    stream_done: bool,
    interactive_input: bool,
    search_mode: bool,
    search_query: String,
    goto_mode: bool,
    goto_query: String,
    #[cfg(test)]
    test_next_key: Option<KeyEvent>,
    #[cfg(test)]
    test_next_key_result: Option<io::Result<Option<KeyEvent>>>,
    #[cfg(test)]
    test_draw_error: Option<io::Error>,
}

impl App {
    pub fn new_file(
        path: PathBuf,
        readonly: bool,
        watch_enabled: bool,
        perf_mode: bool,
        initial_text: String,
    ) -> Result<Self> {
        let (watcher, watch_rx) = if watch_enabled {
            let (watcher, watch_rx) = watcher::start(&path)?;
            (Some(watcher), Some(watch_rx))
        } else {
            (None, None)
        };

        Ok(Self {
            path: Some(path),
            readonly,
            watch_enabled,
            stream_mode: false,
            perf_mode,
            editor: EditorBuffer::new(initial_text),
            status: "Ctrl+Q quit | Ctrl+S save | Ctrl+R reload | Ctrl+F search | Ctrl+G goto | Ctrl+K keep | Ctrl+M merge".into(),
            _watcher: watcher,
            watch_rx,
            stream_rx: None,
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: io::stdin().is_terminal(),
            search_mode: false,
            search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            #[cfg(test)]
            test_next_key: None,
            #[cfg(test)]
            test_next_key_result: None,
            #[cfg(test)]
            test_draw_error: None,
        })
    }

    #[cfg(not(test))]
    pub fn new_stream(perf_mode: bool) -> Result<Self> {
        Ok(Self {
            path: None,
            readonly: true,
            watch_enabled: false,
            stream_mode: true,
            perf_mode,
            editor: EditorBuffer::new(String::new()),
            status: "stream mode: stdin -> preview | Ctrl+Q quit".into(),
            _watcher: None,
            watch_rx: None,
            stream_rx: Some(stream::start()),
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: io::stdin().is_terminal(),
            search_mode: false,
            search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            #[cfg(test)]
            test_next_key: None,
            #[cfg(test)]
            test_next_key_result: None,
            #[cfg(test)]
            test_draw_error: None,
        })
    }

    #[cfg(test)]
    pub fn new_stream(perf_mode: bool) -> Result<Self> {
        Ok(Self::new_stream_for_test(perf_mode))
    }

    #[cfg(test)]
    fn new_stream_for_test(perf_mode: bool) -> Self {
        Self {
            path: None,
            readonly: true,
            watch_enabled: false,
            stream_mode: true,
            perf_mode,
            editor: EditorBuffer::new(String::new()),
            status: "stream mode: stdin -> preview | Ctrl+Q quit".into(),
            _watcher: None,
            watch_rx: None,
            stream_rx: None,
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: false,
            search_mode: false,
            search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            test_next_key: None,
            test_next_key_result: None,
            test_draw_error: None,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        toggle_raw_mode(self.interactive_input, enable_raw_mode)?;

        let loop_result = self.run_loop(&mut terminal);

        toggle_raw_mode(self.interactive_input, disable_raw_mode)?;
        terminal.backend_mut().execute(LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        loop_result
    }

    fn run_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let mut running = true;

        while running {
            self.handle_watch_updates();
            self.handle_stream_updates();

            if !self.interactive_input && self.stream_mode && self.stream_done {
                running = false;
            }

            let started = Instant::now();
            #[cfg(test)]
            if let Some(err) = self.test_draw_error.take() {
                return Err(err.into());
            }
            terminal.draw(|frame| self.draw(frame))?;
            self.draw_time_us = started.elapsed().as_micros();

            if !self.interactive_input && !self.stream_mode {
                running = false;
            }

            if self.interactive_input
                && let Some(key) = self.next_key_event()?
            {
                self.handle_key(key, &mut running)?;
            }
        }

        Ok(())
    }

    fn handle_watch_updates(&mut self) {
        if !self.watch_enabled {
            return;
        }

        let Some(watch_rx) = &self.watch_rx else {
            return;
        };

        let mut latest_external: Option<String> = None;

        while let Ok(msg) = watch_rx.try_recv() {
            self.watch_event_count += 1;
            match msg {
                WatchMessage::ExternalUpdate(text) => {
                    latest_external = Some(text);
                }
                WatchMessage::Error(err) => {
                    self.status = format!("watch error: {err}");
                }
            }
        }

        if let Some(external) = latest_external {
            if external == self.editor.text() {
                return;
            }
            self.editor.on_external_change(external);
            self.ensure_cursor_visible();
            if self.editor.is_conflicted() {
                self.status =
                    "External update conflict: Ctrl+K keep | Ctrl+R reload | Ctrl+M merge".into();
            } else {
                self.status = "File refreshed from disk".into();
            }
        }
    }

    fn handle_stream_updates(&mut self) {
        if !self.stream_mode {
            return;
        }

        let Some(stream_rx) = &self.stream_rx else {
            return;
        };

        let mut latest: Option<String> = None;

        while let Ok(msg) = stream_rx.try_recv() {
            self.stream_event_count += 1;
            match msg {
                StreamMessage::Update(text) => {
                    latest = Some(text);
                }
                StreamMessage::End => {
                    self.stream_done = true;
                    self.status = "stdin closed | Ctrl+Q quit".into();
                }
                StreamMessage::Error(err) => {
                    self.status = format!("stream error: {err}");
                }
            }
        }

        if let Some(text) = latest {
            if text == self.editor.text() {
                return;
            }

            self.editor.on_external_change(text);
            self.ensure_cursor_visible();

            if !self.stream_done {
                self.status = "stream update received".into();
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent, running: &mut bool) -> Result<()> {
        if self.search_mode {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.search_mode = false;
                    self.search_query.clear();
                    *running = false;
                }
                (KeyCode::Esc, _) => {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.status = "Search cancelled".into();
                }
                (KeyCode::Enter, _) => {
                    self.search_mode = false;
                    let query = std::mem::take(&mut self.search_query);
                    if query.is_empty() {
                        self.status = "Search query empty".into();
                    } else if self.editor.find_next(&query) {
                        self.status = format!("Found: {query}");
                    } else {
                        self.status = format!("Not found: {query}");
                    }
                }
                (KeyCode::Backspace, _) => {
                    self.search_query.pop();
                    self.status = format!("Search: {}", self.search_query);
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.search_query.push(c);
                    self.status = format!("Search: {}", self.search_query);
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

        if self.goto_mode {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.goto_mode = false;
                    self.goto_query.clear();
                    *running = false;
                }
                (KeyCode::Esc, _) => {
                    self.goto_mode = false;
                    self.goto_query.clear();
                    self.status = "Goto cancelled".into();
                }
                (KeyCode::Enter, _) => {
                    self.goto_mode = false;
                    let query = std::mem::take(&mut self.goto_query);
                    if query.is_empty() {
                        self.status = "Goto line empty".into();
                    } else if let Ok(line_number) = query.parse::<usize>() {
                        if self.editor.goto_line(line_number) {
                            self.status = format!("Line {line_number}");
                        } else {
                            self.status = format!("Line out of range: {query}");
                        }
                    } else {
                        self.status = format!("Line out of range: {query}");
                    }
                }
                (KeyCode::Backspace, _) => {
                    self.goto_query.pop();
                    self.status = format!("Goto: {}", self.goto_query);
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    if c.is_ascii_digit() {
                        self.goto_query.push(c);
                        self.status = format!("Goto: {}", self.goto_query);
                    }
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                *running = false;
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if self.readonly {
                    self.status = "Readonly: save disabled".into();
                } else if let Some(path) = &self.path {
                    self.editor.save_to_path(path)?;
                    self.status = "Saved".into();
                } else {
                    self.status = "No path: save disabled".into();
                }
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                if self.stream_mode {
                    self.status = "Stream mode: reload disabled".into();
                } else if self.editor.is_conflicted() {
                    self.editor.reload_external();
                    self.status = "Reloaded external".into();
                } else if let Some(path) = &self.path {
                    let disk = fs::read_to_string(path).unwrap_or_default();
                    self.editor.on_external_change(disk);
                    self.status = "Reloaded from disk".into();
                }
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                if self.editor.is_conflicted() {
                    self.editor.keep_local();
                    self.status = "Kept local".into();
                } else {
                    self.status = "No conflict to keep".into();
                }
            }
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                if self.editor.is_conflicted() {
                    self.editor.merge_external();
                    self.status = "Merged with conflict markers".into();
                } else {
                    self.status = "No conflict to merge".into();
                }
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.search_mode = true;
                self.goto_mode = false;
                self.search_query.clear();
                self.status = "Search: ".into();
            }
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.goto_mode = true;
                self.search_mode = false;
                self.goto_query.clear();
                self.status = "Goto: ".into();
            }
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                if self.editor.undo() {
                    self.status = "Undo".into();
                } else {
                    self.status = "Nothing to undo".into();
                }
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                if self.editor.redo() {
                    self.status = "Redo".into();
                } else {
                    self.status = "Nothing to redo".into();
                }
            }
            (KeyCode::Left, _) => self.editor.move_left(),
            (KeyCode::Right, _) => self.editor.move_right(),
            (KeyCode::Up, _) => self.editor.move_up(),
            (KeyCode::Down, _) => self.editor.move_down(),
            (KeyCode::PageUp, _) => {
                self.editor_scroll = self.editor_scroll.saturating_sub(self.editor_height.max(1));
            }
            (KeyCode::PageDown, _) => {
                self.editor_scroll += self.editor_height.max(1);
            }
            (KeyCode::Enter, _) => {
                if !self.readonly {
                    self.editor.insert_newline();
                }
            }
            (KeyCode::Backspace, _) => {
                if !self.readonly {
                    self.editor.backspace();
                }
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if !self.readonly {
                    self.editor.insert_char(c);
                }
            }
            _ => {}
        }

        self.ensure_cursor_visible();
        Ok(())
    }

    fn next_key_event(&mut self) -> Result<Option<KeyEvent>> {
        #[cfg(test)]
        {
            if let Some(result) = self.test_next_key_result.take() {
                return result.map_err(Into::into);
            }
            if let Some(key) = self.test_next_key.take() {
                return Ok(Some(key));
            }
        }
        next_pressed_key(event::poll, event::read)
    }

    fn ensure_cursor_visible(&mut self) {
        let (cursor_line, _) = self.editor.line_col_at_cursor();
        if cursor_line < self.editor_scroll {
            self.editor_scroll = cursor_line;
        } else if cursor_line >= self.editor_scroll + self.editor_height {
            self.editor_scroll = cursor_line.saturating_sub(self.editor_height.saturating_sub(1));
        }

        self.preview_scroll = self.editor_scroll;
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(vertical[0]);

        let editor_height = panes[0].height.saturating_sub(2) as usize;
        self.editor_height = editor_height.max(1);

        let editor_lines = to_lines(self.editor.text());
        self.editor_scroll =
            clamp_scroll(self.editor_scroll, editor_lines.len(), self.editor_height);
        let editor_visible = slice_lines(&editor_lines, self.editor_scroll, self.editor_height);

        let editor = Paragraph::new(editor_visible)
            .block(Block::default().borders(Borders::ALL).title("Editor"));
        frame.render_widget(editor, panes[0]);

        let preview_height = panes[1].height.saturating_sub(2) as usize;
        let preview_width = panes[1].width.saturating_sub(2);
        let mut preview_lines = render_preview_lines(self.editor.text(), preview_width);

        if let Some(conflict) = self.editor.conflict() {
            preview_lines.push(String::new());
            if conflict.hunks.is_empty() {
                preview_lines.push("--- Local block ---".into());
                preview_lines.extend(self.editor.text().lines().map(ToString::to_string));
                preview_lines.push("--- External block ---".into());
                preview_lines.extend(conflict.external.lines().map(ToString::to_string));
            } else {
                for hunk in &conflict.hunks {
                    preview_lines.push(format!("--- Local block @L{} ---", hunk.local_start + 1));
                    if hunk.local_lines.is_empty() {
                        preview_lines.push("(no local lines)".into());
                    } else {
                        preview_lines.extend(hunk.local_lines.iter().cloned());
                    }
                    preview_lines.push(format!(
                        "--- External block @L{} ---",
                        hunk.external_start + 1
                    ));
                    if hunk.external_lines.is_empty() {
                        preview_lines.push("(no external lines)".into());
                    } else {
                        preview_lines.extend(hunk.external_lines.iter().cloned());
                    }
                    preview_lines.push(String::new());
                }
            }
        }

        self.preview_scroll = clamp_scroll(
            self.preview_scroll,
            preview_lines.len(),
            preview_height.max(1),
        );

        let preview_visible =
            slice_lines(&preview_lines, self.preview_scroll, preview_height.max(1));

        let preview = Paragraph::new(preview_visible)
            .block(Block::default().borders(Borders::ALL).title("Preview"));
        frame.render_widget(preview, panes[1]);

        let status_style = if self.editor.is_conflicted() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let status = Paragraph::new(status_line(
            &self.status,
            self.perf_mode,
            self.draw_time_us,
            self.watch_event_count,
            self.stream_event_count,
        ))
        .style(status_style);
        frame.render_widget(status, vertical[1]);

        if !self.readonly {
            let cursor = cursor_rect(panes[0]);
            let (line, col) = self.editor.line_col_at_cursor();
            let visible_line = line.saturating_sub(self.editor_scroll) as u16;
            if visible_line < cursor.height {
                let x = (cursor.x + col as u16).min(cursor.x + cursor.width.saturating_sub(1));
                let y = cursor.y + visible_line;
                frame.set_cursor_position((x, y));
            }
        }
    }
}

fn status_line(
    base: &str,
    perf_mode: bool,
    draw_time_us: u128,
    watch_events: u64,
    stream_events: u64,
) -> String {
    if !perf_mode {
        return base.to_string();
    }

    format!(
        "{base} | perf draw={draw_time_us}us watch_events={watch_events} stream_events={stream_events}"
    )
}

fn to_lines(text: &str) -> Vec<String> {
    text.split('\n').map(ToString::to_string).collect()
}

fn clamp_scroll(scroll: usize, total: usize, height: usize) -> usize {
    if total <= height {
        0
    } else {
        scroll.min(total - height)
    }
}

fn slice_lines(lines: &[String], scroll: usize, height: usize) -> String {
    lines
        .iter()
        .skip(scroll)
        .take(height)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n")
}

fn cursor_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn toggle_raw_mode<F>(interactive: bool, mut f: F) -> Result<()>
where
    F: FnMut() -> io::Result<()>,
{
    if interactive {
        f()?;
    }
    Ok(())
}

fn next_pressed_key<P, R>(mut poll: P, mut read: R) -> Result<Option<KeyEvent>>
where
    P: FnMut(Duration) -> io::Result<bool>,
    R: FnMut() -> io::Result<Event>,
{
    if !poll(Duration::from_millis(30))? {
        return Ok(None);
    }
    let event = read()?;
    if let Event::Key(key) = event
        && key.kind == event::KeyEventKind::Press
    {
        return Ok(Some(key));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::stream::StreamMessage;
    use crate::watcher::WatchMessage;

    use super::{
        App, clamp_scroll, cursor_rect, next_pressed_key, slice_lines, status_line, to_lines,
        toggle_raw_mode,
    };

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("mdv-app-test-{name}-{nanos}.md"))
    }

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn clamp_scroll_bounds() {
        assert_eq!(clamp_scroll(0, 3, 5), 0);
        assert_eq!(clamp_scroll(1, 10, 4), 1);
        assert_eq!(clamp_scroll(20, 10, 4), 6);
    }

    #[test]
    fn slice_lines_respects_window() {
        let lines = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(slice_lines(&lines, 1, 2), "b\nc");
    }

    #[test]
    fn status_line_perf_suffix() {
        assert_eq!(status_line("ok", false, 12, 3, 2), "ok");
        assert_eq!(
            status_line("ok", true, 12, 3, 2),
            "ok | perf draw=12us watch_events=3 stream_events=2"
        );
    }

    #[test]
    fn to_lines_returns_single_empty_line_for_empty_text() {
        assert_eq!(to_lines(""), vec![String::new()]);
    }

    #[test]
    fn to_lines_keeps_trailing_blank_line() {
        assert_eq!(to_lines("a\n"), vec!["a".to_string(), String::new()]);
    }

    #[test]
    fn handle_key_save_reload_merge_and_keep() {
        let path = temp_path("save");
        fs::write(&path, "one").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "one".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE), &mut running)
            .expect("type");
        app.handle_key(key(KeyCode::Char('s'), KeyModifiers::CONTROL), &mut running)
            .expect("save");
        assert_eq!(fs::read_to_string(&path).expect("saved"), "onex");
        assert_eq!(app.status, "Saved");

        app.editor.insert_char('!');
        app.editor.on_external_change("disk".into());
        assert!(app.editor.is_conflicted());
        app.handle_key(key(KeyCode::Char('k'), KeyModifiers::CONTROL), &mut running)
            .expect("keep");
        assert_eq!(app.status, "Kept local");

        app.editor.on_external_change("external".into());
        app.handle_key(key(KeyCode::Char('m'), KeyModifiers::CONTROL), &mut running)
            .expect("merge");
        assert!(app.editor.text().contains("<<<<<<< local"));
        assert_eq!(app.status, "Merged with conflict markers");

        app.handle_key(key(KeyCode::Char('r'), KeyModifiers::CONTROL), &mut running)
            .expect("reload");
        assert_eq!(app.status, "Reloaded from disk");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_undo_redo() {
        let path = temp_path("undo-redo");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE), &mut running)
            .expect("insert");
        assert_eq!(app.editor.text(), "xa");

        app.handle_key(key(KeyCode::Char('z'), KeyModifiers::CONTROL), &mut running)
            .expect("undo");
        assert_eq!(app.editor.text(), "x");
        assert_eq!(app.status, "Undo");

        app.handle_key(key(KeyCode::Char('y'), KeyModifiers::CONTROL), &mut running)
            .expect("redo");
        assert_eq!(app.editor.text(), "xa");
        assert_eq!(app.status, "Redo");

        app.handle_key(key(KeyCode::Char('z'), KeyModifiers::CONTROL), &mut running)
            .expect("undo 2");
        app.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE), &mut running)
            .expect("insert 2");
        app.handle_key(key(KeyCode::Char('y'), KeyModifiers::CONTROL), &mut running)
            .expect("redo empty");
        assert_eq!(app.status, "Nothing to redo");
        assert_eq!(app.editor.text(), "xb");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_search_mode() {
        let path = temp_path("search");
        fs::write(&path, "one two one").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "one two one".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('f'), KeyModifiers::CONTROL), &mut running)
            .expect("start search");
        app.handle_key(key(KeyCode::Char('o'), KeyModifiers::NONE), &mut running)
            .expect("q1");
        app.handle_key(key(KeyCode::Char('n'), KeyModifiers::NONE), &mut running)
            .expect("q2");
        app.handle_key(key(KeyCode::Char('e'), KeyModifiers::NONE), &mut running)
            .expect("q3");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("exec search");
        assert_eq!(app.status, "Found: one");
        assert_eq!(app.editor.cursor(), 0);

        app.handle_key(key(KeyCode::Char('f'), KeyModifiers::CONTROL), &mut running)
            .expect("start search 2");
        app.handle_key(key(KeyCode::Char('z'), KeyModifiers::NONE), &mut running)
            .expect("qz");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("exec search 2");
        assert_eq!(app.status, "Not found: z");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_goto_mode() {
        let path = temp_path("goto");
        fs::write(&path, "a\nb\nc").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "a\nb\nc".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL), &mut running)
            .expect("start goto");
        app.handle_key(key(KeyCode::Char('2'), KeyModifiers::NONE), &mut running)
            .expect("line");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("exec goto");
        assert_eq!(app.editor.line_col_at_cursor(), (1, 0));
        assert_eq!(app.status, "Line 2");

        app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL), &mut running)
            .expect("start goto 2");
        app.handle_key(key(KeyCode::Char('9'), KeyModifiers::NONE), &mut running)
            .expect("line 9");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("exec goto 2");
        assert_eq!(app.status, "Line out of range: 9");

        app.handle_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL), &mut running)
            .expect("start goto 3");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("exec goto 3");
        assert_eq!(app.status, "Goto line empty");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_quit_works_in_prompt_modes() {
        let path = temp_path("quit-prompts");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('f'), KeyModifiers::CONTROL), &mut running)
            .expect("search mode");
        app.handle_key(key(KeyCode::Char('q'), KeyModifiers::CONTROL), &mut running)
            .expect("quit search mode");
        assert!(!running);

        let mut app2 = App::new_file(path.clone(), false, false, false, "x".into()).expect("app2");
        app2.interactive_input = false;
        let mut running2 = true;
        app2.handle_key(
            key(KeyCode::Char('g'), KeyModifiers::CONTROL),
            &mut running2,
        )
        .expect("goto mode");
        app2.handle_key(
            key(KeyCode::Char('q'), KeyModifiers::CONTROL),
            &mut running2,
        )
        .expect("quit goto mode");
        assert!(!running2);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_readonly_and_no_path_branches() {
        let path = temp_path("readonly");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), true, false, false, "x".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('s'), KeyModifiers::CONTROL), &mut running)
            .expect("save");
        assert_eq!(app.status, "Readonly: save disabled");

        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("enter");
        assert_eq!(app.editor.text(), "x");

        app.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE), &mut running)
            .expect("backspace");
        assert_eq!(app.editor.text(), "x");

        let mut stream_app = App::new_stream_for_test(false);
        stream_app.readonly = false;
        stream_app.interactive_input = false;
        stream_app
            .handle_key(key(KeyCode::Char('s'), KeyModifiers::CONTROL), &mut running)
            .expect("save no path");
        assert_eq!(stream_app.status, "No path: save disabled");

        stream_app
            .handle_key(key(KeyCode::Char('r'), KeyModifiers::CONTROL), &mut running)
            .expect("reload stream");
        assert_eq!(stream_app.status, "Stream mode: reload disabled");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_conflict_actions_no_conflict_status() {
        let path = temp_path("no-conflict-actions");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('k'), KeyModifiers::CONTROL), &mut running)
            .expect("keep");
        assert_eq!(app.status, "No conflict to keep");

        app.handle_key(key(KeyCode::Char('m'), KeyModifiers::CONTROL), &mut running)
            .expect("merge");
        assert_eq!(app.status, "No conflict to merge");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_navigation_and_quit() {
        let path = temp_path("nav");
        fs::write(&path, "ab\ncd").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "ab\ncd".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Left, KeyModifiers::NONE), &mut running)
            .expect("left");
        app.handle_key(key(KeyCode::Right, KeyModifiers::NONE), &mut running)
            .expect("right");
        app.handle_key(key(KeyCode::Up, KeyModifiers::NONE), &mut running)
            .expect("up");
        app.handle_key(key(KeyCode::Down, KeyModifiers::NONE), &mut running)
            .expect("down");
        app.handle_key(key(KeyCode::PageDown, KeyModifiers::NONE), &mut running)
            .expect("pgdn");
        app.handle_key(key(KeyCode::PageUp, KeyModifiers::NONE), &mut running)
            .expect("pgup");
        app.handle_key(key(KeyCode::Char('q'), KeyModifiers::CONTROL), &mut running)
            .expect("quit");
        assert!(!running);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_watch_updates_sets_status_and_conflict() {
        let path = temp_path("watch");
        fs::write(&path, "local").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "local".into()).expect("app");
        app.watch_enabled = true;
        let (tx, rx) = mpsc::channel();
        app.watch_rx = Some(rx);

        tx.send(WatchMessage::ExternalUpdate("local".into()))
            .expect("send same");
        app.handle_watch_updates();
        assert_eq!(
            app.status,
            "Ctrl+Q quit | Ctrl+S save | Ctrl+R reload | Ctrl+F search | Ctrl+G goto | Ctrl+K keep | Ctrl+M merge"
        );

        tx.send(WatchMessage::ExternalUpdate("disk".into()))
            .expect("send update");
        app.handle_watch_updates();
        assert_eq!(app.status, "File refreshed from disk");
        assert_eq!(app.editor.text(), "disk");

        app.editor.insert_char('!');
        tx.send(WatchMessage::ExternalUpdate("disk2".into()))
            .expect("send conflict");
        app.handle_watch_updates();
        assert!(app.editor.is_conflicted());
        assert_eq!(
            app.status,
            "External update conflict: Ctrl+K keep | Ctrl+R reload | Ctrl+M merge"
        );

        tx.send(WatchMessage::Error("bad".into()))
            .expect("send error");
        app.handle_watch_updates();
        assert!(app.status.contains("watch error"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_stream_updates_sets_status() {
        let mut app = App::new_stream_for_test(false);
        app.interactive_input = false;
        let (tx, rx) = mpsc::channel();
        app.stream_rx = Some(rx);

        tx.send(StreamMessage::Update("one".into()))
            .expect("send update");
        app.handle_stream_updates();
        assert_eq!(app.editor.text(), "one");
        assert_eq!(app.status, "stream update received");

        tx.send(StreamMessage::Error("e".into())).expect("send err");
        app.handle_stream_updates();
        assert!(app.status.contains("stream error"));

        tx.send(StreamMessage::End).expect("send end");
        app.handle_stream_updates();
        assert!(app.stream_done);
        assert_eq!(app.status, "stdin closed | Ctrl+Q quit");
    }

    #[test]
    fn draw_renders_conflict_blocks() {
        let path = temp_path("draw");
        fs::write(&path, "a\nb").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, true, "a\nb".into()).expect("app");
        app.editor.insert_char('!');
        app.editor.on_external_change("a\nB\nc".into());
        app.editor_scroll = 0;
        app.preview_scroll = 0;

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");

        let rendered = terminal.backend().buffer().content();
        assert!(rendered.iter().any(|cell| cell.symbol() == "L"));
        assert!(rendered.iter().any(|cell| cell.symbol() == "E"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn draw_handles_empty_hunks_branch() {
        let path = temp_path("draw-empty");
        fs::write(&path, "same").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "same".into()).expect("app");
        app.editor.dirty = true;
        app.editor.on_external_change("same".into());

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn run_loop_and_run_exit_non_interactive_file_mode() {
        let path = temp_path("run");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = false;
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        app.run_loop(&mut terminal).expect("run_loop");
        let _ = app.run();

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn cursor_rect_offsets_inner_area() {
        let r = cursor_rect(ratatui::layout::Rect {
            x: 1,
            y: 2,
            width: 10,
            height: 5,
        });
        assert_eq!(r.x, 2);
        assert_eq!(r.y, 3);
        assert_eq!(r.width, 8);
        assert_eq!(r.height, 3);
    }

    #[test]
    fn new_file_with_watcher_enabled_starts() {
        let path = temp_path("new-watch");
        fs::write(&path, "x").expect("seed");
        let app = App::new_file(path.clone(), false, true, false, "x".into());
        assert!(app.is_ok());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn new_file_with_watcher_enabled_returns_error_for_missing_path() {
        let path = temp_path("new-watch-missing");
        let app = App::new_file(path, false, true, false, String::new());
        assert!(app.is_err());
    }

    #[test]
    fn handle_watch_and_stream_none_receivers_return() {
        let path = temp_path("none-rx");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.watch_enabled = true;
        app.watch_rx = None;
        app.handle_watch_updates();

        let mut stream_app = App::new_stream_for_test(false);
        stream_app.stream_rx = None;
        stream_app.handle_stream_updates();

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_reload_conflict_branch_and_edit_keys() {
        let path = temp_path("reload-conflict");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.editor.insert_char('!');
        app.editor.on_external_change("disk".into());
        let mut running = true;
        app.handle_key(key(KeyCode::Char('r'), KeyModifiers::CONTROL), &mut running)
            .expect("reload conflict");
        assert_eq!(app.status, "Reloaded external");
        assert_eq!(app.editor.text(), "disk");

        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("enter");
        app.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE), &mut running)
            .expect("backspace");
        app.handle_key(key(KeyCode::Tab, KeyModifiers::NONE), &mut running)
            .expect("unknown key");

        // Exercise branch where reload is requested with no path and not stream mode.
        let mut no_path = App::new_stream_for_test(false);
        no_path.stream_mode = false;
        no_path
            .handle_key(key(KeyCode::Char('r'), KeyModifiers::CONTROL), &mut running)
            .expect("reload no-path no-stream");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn run_loop_handles_stream_done_non_interactive() {
        let mut app = App::new_stream_for_test(false);
        app.interactive_input = false;
        app.stream_done = true;
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        app.run_loop(&mut terminal).expect("run loop stream done");
    }

    #[test]
    fn run_loop_interactive_consumes_queued_key() {
        let path = temp_path("interactive-loop");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = true;
        app.test_next_key = Some(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        app.run_loop(&mut terminal).expect("run loop");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn run_loop_propagates_draw_error() {
        let path = temp_path("draw-error");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = false;
        app.test_draw_error = Some(io::Error::other("draw failed"));

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        let err = app.run_loop(&mut terminal).expect_err("draw error");
        assert!(err.to_string().contains("draw failed"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn run_loop_propagates_next_key_event_error() {
        let path = temp_path("next-key-error");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = true;
        app.test_next_key_result = Some(Err(io::Error::other("poll failed")));

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        let err = app.run_loop(&mut terminal).expect_err("next key error");
        assert!(err.to_string().contains("poll failed"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn run_loop_propagates_handle_key_save_error() {
        let path = temp_path("save-dir");
        fs::create_dir(&path).expect("mkdir");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = true;
        app.test_next_key = Some(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        assert!(app.run_loop(&mut terminal).is_err());

        let _ = fs::remove_dir(&path);
    }

    #[test]
    fn next_pressed_key_branches() {
        fn resize_event() -> io::Result<Event> {
            Ok(Event::Resize(80, 24))
        }

        let key = next_pressed_key(|_| Ok(false), resize_event).expect("none");
        assert!(key.is_none());

        let key = next_pressed_key(|_| Ok(true), resize_event).expect("resize");
        assert!(key.is_none());

        let key = next_pressed_key(
            |_| Ok(true),
            || {
                Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('x'),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Release,
                    state: KeyEventState::NONE,
                }))
            },
        )
        .expect("release");
        assert!(key.is_none());

        let key = next_pressed_key(
            |_| Ok(true),
            || {
                Ok(Event::Key(KeyEvent::new(
                    KeyCode::Char('q'),
                    KeyModifiers::CONTROL,
                )))
            },
        )
        .expect("press");
        assert!(matches!(key, Some(k) if k.code == KeyCode::Char('q')));
    }

    #[test]
    fn next_key_event_falls_back_to_terminal_poll() {
        let mut app = App::new_stream_for_test(false);
        app.test_next_key = None;
        let _ = app.next_key_event();
    }

    #[test]
    fn new_stream_builds_stream_mode_app() {
        let app = App::new_stream(false).expect("app");
        assert!(app.stream_mode);
    }

    #[test]
    fn next_pressed_key_read_error_branch() {
        let err = next_pressed_key(|_| Ok(true), || Err(io::Error::other("read failed")))
            .expect_err("read error");
        assert!(err.to_string().contains("read failed"));
    }

    #[test]
    fn handle_key_shift_char_inserts() {
        let path = temp_path("shift-char");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        let mut running = true;
        app.handle_key(key(KeyCode::Char('A'), KeyModifiers::SHIFT), &mut running)
            .expect("shift char");
        assert_eq!(app.editor.text(), "xA");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_char_readonly_does_not_insert() {
        let path = temp_path("readonly-char");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), true, false, false, "x".into()).expect("app");
        let mut running = true;
        app.handle_key(key(KeyCode::Char('z'), KeyModifiers::NONE), &mut running)
            .expect("readonly char");
        assert_eq!(app.editor.text(), "x");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn draw_sets_cursor_when_visible() {
        let path = temp_path("draw-cursor");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.editor_scroll = 0;
        app.preview_scroll = 0;

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn draw_skips_cursor_when_not_visible() {
        let path = temp_path("draw-cursor-offscreen");
        let text = (0..40)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, &text).expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, text).expect("app");
        app.editor_scroll = 0;
        app.preview_scroll = 0;

        let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn toggle_raw_mode_propagates_error() {
        let err = toggle_raw_mode(true, || Err(io::Error::other("raw mode failed")))
            .expect_err("expected error");
        assert!(err.to_string().contains("raw mode failed"));
    }

    #[test]
    fn toggle_raw_mode_success_branches() {
        fn ok_raw_mode() -> io::Result<()> {
            Ok(())
        }

        toggle_raw_mode(false, ok_raw_mode).expect("false branch");
        toggle_raw_mode(true, ok_raw_mode).expect("true noop");

        let mut called = false;
        toggle_raw_mode(true, || {
            called = true;
            Ok(())
        })
        .expect("true branch");
        assert!(called);
    }

    #[test]
    fn stream_update_same_text_returns_early() {
        let mut app = App::new_stream_for_test(false);
        app.interactive_input = false;
        let (tx, rx) = mpsc::channel();
        app.stream_rx = Some(rx);
        app.editor.on_external_change("same".into());
        tx.send(StreamMessage::Update("same".into())).expect("send");
        app.handle_stream_updates();
        assert_eq!(app.editor.text(), "same");
    }

    #[test]
    fn draw_renders_no_local_or_external_markers() {
        let path = temp_path("draw-markers");
        fs::write(&path, "a").expect("seed");

        let mut insert_only =
            App::new_file(path.clone(), false, false, false, "a".into()).expect("app");
        insert_only.editor.dirty = true;
        insert_only.editor.on_external_change("a\nb".into());
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        terminal
            .draw(|frame| insert_only.draw(frame))
            .expect("draw insert");

        let mut delete_only =
            App::new_file(path.clone(), false, false, false, "a\nb".into()).expect("app");
        delete_only.editor.dirty = true;
        delete_only.editor.on_external_change("a".into());
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        terminal
            .draw(|frame| delete_only.draw(frame))
            .expect("draw delete");

        let _ = fs::remove_file(&path);
    }
}
