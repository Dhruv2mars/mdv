pub mod action;
pub mod input;
pub mod state;
pub mod update;

use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use mdv_core::{EditorBuffer, SegmentKind, render_preview_lines, render_preview_segments};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

#[cfg(not(test))]
use crate::stream;
use crate::stream::StreamMessage;
use crate::ui::help;
use crate::ui::layout::{LayoutKind, compute_pane_layout};
use crate::ui::render::{compose_status, truncate_middle};
use crate::ui::theme::{ThemeTokens, build_theme, style_for_segment};
use crate::watcher::{self, WatchMessage};
use action::Action;
use state::UiState;
pub use state::{PaneFocus, ThemeChoice};

const SCROLL_STEP_LINES: usize = 3;

pub struct App {
    path: Option<PathBuf>,
    readonly: bool,
    watch_enabled: bool,
    home_mode: bool,
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
    preview_height: usize,
    draw_time_us: u128,
    watch_event_count: u64,
    stream_event_count: u64,
    stream_done: bool,
    interactive_input: bool,
    search_mode: bool,
    search_query: String,
    last_search_query: String,
    goto_mode: bool,
    goto_query: String,
    replace_find_mode: bool,
    replace_find_query: String,
    replace_with_mode: bool,
    replace_with_query: String,
    replace_target: String,
    home_query: String,
    selected_conflict_hunk: usize,
    preview_cache: Option<PreviewCache>,
    ui: UiState,
    term_width: u16,
    #[cfg(test)]
    test_next_key: Option<KeyEvent>,
    #[cfg(test)]
    test_next_key_result: Option<io::Result<Option<KeyEvent>>>,
    #[cfg(test)]
    test_draw_error: Option<io::Error>,
    #[cfg(test)]
    test_preview_cache_hits: u64,
    #[cfg(test)]
    test_preview_cache_misses: u64,
}

#[derive(Clone)]
struct PreviewCache {
    key: u64,
    lines: Arc<Vec<String>>,
    selected_anchor: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputEvent {
    Key(KeyEvent),
    ScrollUp,
    ScrollDown,
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
            home_mode: false,
            stream_mode: false,
            perf_mode,
            editor: EditorBuffer::new(initial_text),
            status: "Ready".into(),
            _watcher: watcher,
            watch_rx,
            stream_rx: None,
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            preview_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: io::stdin().is_terminal(),
            search_mode: false,
            search_query: String::new(),
            last_search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            replace_find_mode: false,
            replace_find_query: String::new(),
            replace_with_mode: false,
            replace_with_query: String::new(),
            replace_target: String::new(),
            home_query: String::new(),
            selected_conflict_hunk: 0,
            preview_cache: None,
            ui: UiState::default(),
            term_width: 120,
            #[cfg(test)]
            test_next_key: None,
            #[cfg(test)]
            test_next_key_result: None,
            #[cfg(test)]
            test_draw_error: None,
            #[cfg(test)]
            test_preview_cache_hits: 0,
            #[cfg(test)]
            test_preview_cache_misses: 0,
        })
    }

    #[cfg(not(test))]
    pub fn new_stream(perf_mode: bool) -> Result<Self> {
        Ok(Self {
            path: None,
            readonly: true,
            watch_enabled: false,
            home_mode: false,
            stream_mode: true,
            perf_mode,
            editor: EditorBuffer::new(String::new()),
            status: "Stream mode".into(),
            _watcher: None,
            watch_rx: None,
            stream_rx: Some(stream::start()),
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            preview_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: io::stdin().is_terminal(),
            search_mode: false,
            search_query: String::new(),
            last_search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            replace_find_mode: false,
            replace_find_query: String::new(),
            replace_with_mode: false,
            replace_with_query: String::new(),
            replace_target: String::new(),
            home_query: String::new(),
            selected_conflict_hunk: 0,
            preview_cache: None,
            ui: UiState::default(),
            term_width: 120,
            #[cfg(test)]
            test_next_key: None,
            #[cfg(test)]
            test_next_key_result: None,
            #[cfg(test)]
            test_draw_error: None,
            #[cfg(test)]
            test_preview_cache_hits: 0,
            #[cfg(test)]
            test_preview_cache_misses: 0,
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
            home_mode: false,
            stream_mode: true,
            perf_mode,
            editor: EditorBuffer::new(String::new()),
            status: "Stream mode".into(),
            _watcher: None,
            watch_rx: None,
            stream_rx: None,
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            preview_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: false,
            search_mode: false,
            search_query: String::new(),
            last_search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            replace_find_mode: false,
            replace_find_query: String::new(),
            replace_with_mode: false,
            replace_with_query: String::new(),
            replace_target: String::new(),
            home_query: String::new(),
            selected_conflict_hunk: 0,
            preview_cache: None,
            ui: UiState::default(),
            term_width: 120,
            test_next_key: None,
            test_next_key_result: None,
            test_draw_error: None,
            test_preview_cache_hits: 0,
            test_preview_cache_misses: 0,
        }
    }

    pub fn new_home(readonly: bool, watch_enabled: bool, perf_mode: bool) -> Result<Self> {
        Ok(Self {
            path: None,
            readonly,
            watch_enabled,
            home_mode: true,
            stream_mode: false,
            perf_mode,
            editor: EditorBuffer::new(String::new()),
            status: "Home".into(),
            _watcher: None,
            watch_rx: None,
            stream_rx: None,
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            preview_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
            stream_event_count: 0,
            stream_done: false,
            interactive_input: io::stdin().is_terminal(),
            search_mode: false,
            search_query: String::new(),
            last_search_query: String::new(),
            goto_mode: false,
            goto_query: String::new(),
            replace_find_mode: false,
            replace_find_query: String::new(),
            replace_with_mode: false,
            replace_with_query: String::new(),
            replace_target: String::new(),
            home_query: String::new(),
            selected_conflict_hunk: 0,
            preview_cache: None,
            ui: UiState::default(),
            term_width: 120,
            #[cfg(test)]
            test_next_key: None,
            #[cfg(test)]
            test_next_key_result: None,
            #[cfg(test)]
            test_draw_error: None,
            #[cfg(test)]
            test_preview_cache_hits: 0,
            #[cfg(test)]
            test_preview_cache_misses: 0,
        })
    }

    #[cfg(test)]
    fn new_home_for_test(readonly: bool, watch_enabled: bool, perf_mode: bool) -> Self {
        let mut app = Self::new_home(readonly, watch_enabled, perf_mode).expect("home app");
        app.interactive_input = false;
        app
    }

    pub fn set_theme(&mut self, theme: ThemeChoice) {
        let focus = self.ui.focus;
        let no_color = self.ui.no_color;
        update::apply_action(
            &mut self.ui,
            Action::ApplyPrefs {
                focus,
                theme,
                no_color,
            },
            self.term_width,
        );
    }

    pub fn set_no_color(&mut self, no_color: bool) {
        let focus = self.ui.focus;
        let theme = self.ui.theme;
        update::apply_action(
            &mut self.ui,
            Action::ApplyPrefs {
                focus,
                theme,
                no_color,
            },
            self.term_width,
        );
    }

    pub fn set_initial_focus(&mut self, focus: PaneFocus) {
        let theme = self.ui.theme;
        let no_color = self.ui.no_color;
        update::apply_action(
            &mut self.ui,
            Action::ApplyPrefs {
                focus,
                theme,
                no_color,
            },
            self.term_width,
        );
    }

    pub fn run(&mut self) -> Result<()> {
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        if self.interactive_input {
            stdout.execute(EnableMouseCapture)?;
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        toggle_raw_mode(self.interactive_input, enable_raw_mode)?;

        let loop_result = self.run_loop(&mut terminal);

        toggle_raw_mode(self.interactive_input, disable_raw_mode)?;
        if self.interactive_input {
            terminal.backend_mut().execute(DisableMouseCapture)?;
        }
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
                && let Some(input_event) = self.next_input_event()?
            {
                match input_event {
                    InputEvent::Key(key) => self.handle_key(key, &mut running)?,
                    InputEvent::ScrollUp => self.scroll_active_viewport(-1),
                    InputEvent::ScrollDown => self.scroll_active_viewport(1),
                }
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
            self.sync_conflict_hunk_selection();
            self.ensure_cursor_visible();
            if self.editor.is_conflicted() {
                self.status =
                    "External update conflict: Ctrl+J/Ctrl+U hunk | Ctrl+E apply | Ctrl+K keep | Ctrl+R reload | Ctrl+M merge".into();
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

        let mut latest: Option<(String, bool)> = None;

        while let Ok(msg) = stream_rx.try_recv() {
            self.stream_event_count += 1;
            match msg {
                StreamMessage::Update { text, truncated } => {
                    latest = Some((text, truncated));
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

        if let Some((text, truncated)) = latest {
            if text == self.editor.text() {
                return;
            }

            self.editor.on_external_change(text);
            self.sync_conflict_hunk_selection();
            self.ensure_cursor_visible();

            if !self.stream_done {
                self.status = if truncated {
                    "stream update received (trimmed)".into()
                } else {
                    "stream update received".into()
                };
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent, running: &mut bool) -> Result<()> {
        if let Some(action) = input::map_global_key(key) {
            update::apply_action(&mut self.ui, action, self.term_width);
            match action {
                Action::ToggleFocus => {
                    if self.ui.focus == PaneFocus::Preview {
                        self.preview_scroll = self.editor_scroll;
                    }
                    self.status = match self.ui.focus {
                        PaneFocus::Editor => "Mode: editor".into(),
                        PaneFocus::Preview => "Mode: view".into(),
                    };
                }
                Action::ToggleHelp => {
                    self.status = if self.ui.help_open {
                        "Settings opened".into()
                    } else {
                        "Settings closed".into()
                    };
                }
                Action::ApplyPrefs { .. } => {}
            }
            return Ok(());
        }

        if self.ui.help_open {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => {
                    self.ui.help_open = false;
                    self.status = "Settings closed".into();
                }
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.ui.help_open = false;
                    *running = false;
                }
                _ => {}
            }
            return Ok(());
        }

        if self.home_mode {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    *running = false;
                }
                (KeyCode::Enter, _) => {
                    if self.home_query.trim().is_empty() {
                        self.status = "Home: type a path".into();
                    } else {
                        self.open_home_path(PathBuf::from(self.home_query.trim()));
                    }
                }
                (KeyCode::Esc, _) => {
                    self.home_query.clear();
                    self.status = "Home: query cleared".into();
                }
                (KeyCode::Backspace, _) => {
                    self.home_query.pop();
                    self.status = format!("Home path: {}", self.home_query);
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.home_query.push(c);
                    self.status = format!("Home path: {}", self.home_query);
                }
                _ => {}
            }
            return Ok(());
        }

        if self.replace_find_mode {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.clear_replace_mode();
                    *running = false;
                }
                (KeyCode::Esc, _) => {
                    self.clear_replace_mode();
                    self.status = "Replace cancelled".into();
                }
                (KeyCode::Enter, _) => {
                    if self.replace_find_query.is_empty() {
                        self.status = "Replace query empty".into();
                    } else {
                        self.replace_find_mode = false;
                        self.replace_with_mode = true;
                        self.replace_target = std::mem::take(&mut self.replace_find_query);
                        self.replace_with_query.clear();
                        self.status = "Replace with: ".into();
                    }
                }
                (KeyCode::Backspace, _) => {
                    self.replace_find_query.pop();
                    self.status = format!("Replace find: {}", self.replace_find_query);
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.replace_find_query.push(c);
                    self.status = format!("Replace find: {}", self.replace_find_query);
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

        if self.replace_with_mode {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                    self.clear_replace_mode();
                    *running = false;
                }
                (KeyCode::Esc, _) => {
                    self.clear_replace_mode();
                    self.status = "Replace cancelled".into();
                }
                (KeyCode::Enter, _) => {
                    self.apply_replace_next();
                }
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    self.apply_replace_all();
                }
                (KeyCode::Backspace, _) => {
                    self.replace_with_query.pop();
                    self.status = format!("Replace with: {}", self.replace_with_query);
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.replace_with_query.push(c);
                    self.status = format!("Replace with: {}", self.replace_with_query);
                }
                _ => {}
            }
            self.ensure_cursor_visible();
            return Ok(());
        }

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
                    } else {
                        self.last_search_query = query.clone();
                        if self.editor.find_next(&query) {
                            self.status = format!("Found: {query}");
                        } else {
                            self.status = format!("Not found: {query}");
                        }
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
                    self.sync_conflict_hunk_selection();
                    self.status = "Reloaded external".into();
                } else if let Some(path) = &self.path {
                    let disk = fs::read_to_string(path).unwrap_or_default();
                    self.editor.on_external_change(disk);
                    self.sync_conflict_hunk_selection();
                    self.status = "Reloaded from disk".into();
                }
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                if self.editor.is_conflicted() {
                    self.editor.keep_local();
                    self.sync_conflict_hunk_selection();
                    self.status = "Kept local".into();
                } else {
                    self.status = "No conflict to keep".into();
                }
            }
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                if self.editor.is_conflicted() {
                    self.editor.merge_external();
                    self.sync_conflict_hunk_selection();
                    self.status = "Merged with conflict markers".into();
                } else {
                    self.status = "No conflict to merge".into();
                }
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.search_mode = true;
                self.goto_mode = false;
                self.clear_replace_mode();
                self.search_query.clear();
                self.status = "Search: ".into();
            }
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.search_mode = false;
                self.goto_mode = false;
                self.replace_find_mode = true;
                self.replace_with_mode = false;
                self.replace_find_query.clear();
                self.replace_with_query.clear();
                self.replace_target.clear();
                self.status = "Replace find: ".into();
            }
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                self.goto_mode = true;
                self.search_mode = false;
                self.clear_replace_mode();
                self.goto_query.clear();
                self.status = "Goto: ".into();
            }
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                if self.editor.undo() {
                    self.sync_conflict_hunk_selection();
                    self.status = "Undo".into();
                } else {
                    self.status = "Nothing to undo".into();
                }
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                if self.editor.redo() {
                    self.sync_conflict_hunk_selection();
                    self.status = "Redo".into();
                } else {
                    self.status = "Nothing to redo".into();
                }
            }
            (KeyCode::F(3), KeyModifiers::NONE) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.repeat_search_next();
            }
            (KeyCode::F(3), KeyModifiers::SHIFT) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.repeat_search_prev();
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.move_conflict_hunk(1);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.move_conflict_hunk(-1);
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.apply_selected_conflict_hunk();
            }
            (KeyCode::Left, _) => self.editor.move_left(),
            (KeyCode::Right, _) => self.editor.move_right(),
            (KeyCode::Up, _) => self.editor.move_up(),
            (KeyCode::Down, _) => self.editor.move_down(),
            (KeyCode::PageUp, _) => {
                if self.ui.focus == PaneFocus::Editor {
                    self.editor_scroll =
                        self.editor_scroll.saturating_sub(self.editor_height.max(1));
                } else {
                    self.preview_scroll = self
                        .preview_scroll
                        .saturating_sub(self.preview_height.max(1));
                }
            }
            (KeyCode::PageDown, _) => {
                if self.ui.focus == PaneFocus::Editor {
                    self.editor_scroll += self.editor_height.max(1);
                } else {
                    self.preview_scroll += self.preview_height.max(1);
                }
            }
            (KeyCode::Enter, _) => {
                if !self.readonly {
                    self.editor.insert_newline();
                    self.sync_conflict_hunk_selection();
                }
            }
            (KeyCode::Backspace, _) => {
                if !self.readonly {
                    self.editor.backspace();
                    self.sync_conflict_hunk_selection();
                }
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if !self.readonly {
                    self.editor.insert_char(c);
                    self.sync_conflict_hunk_selection();
                }
            }
            _ => {}
        }

        self.ensure_cursor_visible();
        Ok(())
    }

    fn next_input_event(&mut self) -> Result<Option<InputEvent>> {
        #[cfg(test)]
        {
            if let Some(result) = self.test_next_key_result.take() {
                return result
                    .map(|event| event.map(InputEvent::Key))
                    .map_err(Into::into);
            }
            if let Some(key) = self.test_next_key.take() {
                return Ok(Some(InputEvent::Key(key)));
            }
        }
        next_terminal_input(event::poll, event::read)
    }

    fn ensure_cursor_visible(&mut self) {
        let (cursor_line, _) = self.editor.line_col_at_cursor();
        if cursor_line < self.editor_scroll {
            self.editor_scroll = cursor_line;
        } else if cursor_line >= self.editor_scroll + self.editor_height {
            self.editor_scroll = cursor_line.saturating_sub(self.editor_height.saturating_sub(1));
        }
    }

    fn open_home_path(&mut self, path: PathBuf) {
        let existed = path.exists();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
            Err(err) => {
                self.status = format!("open error: {err}");
                return;
            }
        };

        if self.watch_enabled && existed {
            match watcher::start(&path) {
                Ok((watcher, rx)) => {
                    self._watcher = Some(watcher);
                    self.watch_rx = Some(rx);
                }
                Err(err) => {
                    self._watcher = None;
                    self.watch_rx = None;
                    self.status = format!("watch error: {err}");
                }
            }
        } else {
            self._watcher = None;
            self.watch_rx = None;
        }

        self.path = Some(path.clone());
        self.editor = EditorBuffer::new(text);
        self.preview_cache = None;
        self.home_mode = false;
        self.search_mode = false;
        self.goto_mode = false;
        self.clear_replace_mode();
        self.home_query.clear();
        self.editor_scroll = 0;
        self.preview_scroll = 0;
        self.sync_conflict_hunk_selection();
        self.status = if existed {
            format!("Opened {}", path.display())
        } else {
            format!("New file {}", path.display())
        };
    }

    fn scroll_active_viewport(&mut self, direction: i8) {
        let amount = SCROLL_STEP_LINES;
        if self.ui.focus == PaneFocus::Editor {
            let total = to_lines(self.editor.text()).len();
            if direction < 0 {
                self.editor_scroll = self.editor_scroll.saturating_sub(amount);
            } else {
                self.editor_scroll = self.editor_scroll.saturating_add(amount);
            }
            self.editor_scroll = clamp_scroll(self.editor_scroll, total, self.editor_height.max(1));
            return;
        }

        let preview_width = self.term_width.saturating_sub(2).max(1);
        let (preview_lines, _) = self.preview_lines_cached(preview_width);
        if direction < 0 {
            self.preview_scroll = self.preview_scroll.saturating_sub(amount);
        } else {
            self.preview_scroll = self.preview_scroll.saturating_add(amount);
        }
        self.preview_scroll = clamp_scroll(
            self.preview_scroll,
            preview_lines.len(),
            self.preview_height.max(1),
        );
    }

    fn clear_replace_mode(&mut self) {
        self.replace_find_mode = false;
        self.replace_with_mode = false;
        self.replace_find_query.clear();
        self.replace_with_query.clear();
        self.replace_target.clear();
    }

    fn apply_replace_next(&mut self) {
        let find = self.replace_target.clone();
        let replacement = std::mem::take(&mut self.replace_with_query);
        self.clear_replace_mode();

        if find.is_empty() {
            self.status = "Replace query empty".into();
            return;
        }
        if self.readonly {
            self.status = "Readonly: replace disabled".into();
            return;
        }

        if self.editor.replace_next(&find, &replacement) {
            self.last_search_query = find.clone();
            self.sync_conflict_hunk_selection();
            self.status = format!("Replaced: {find} -> {replacement}");
        } else {
            self.status = format!("Not found: {find}");
        }
    }

    fn apply_replace_all(&mut self) {
        let find = self.replace_target.clone();
        let replacement = std::mem::take(&mut self.replace_with_query);
        self.clear_replace_mode();

        if find.is_empty() {
            self.status = "Replace query empty".into();
            return;
        }
        if self.readonly {
            self.status = "Readonly: replace disabled".into();
            return;
        }

        let count = self.editor.replace_all(&find, &replacement);
        if count > 0 {
            self.last_search_query = find.clone();
            self.sync_conflict_hunk_selection();
            self.status = format!("Replaced all {count}: {find} -> {replacement}");
        } else {
            self.status = format!("Not found: {find}");
        }
    }

    fn repeat_search_next(&mut self) {
        if self.last_search_query.is_empty() {
            self.status = "No prior search".into();
            return;
        }

        if self.editor.find_next(&self.last_search_query) {
            self.status = format!("Found next: {}", self.last_search_query);
            self.ensure_cursor_visible();
        } else {
            self.status = format!("Not found: {}", self.last_search_query);
        }
    }

    fn repeat_search_prev(&mut self) {
        if self.last_search_query.is_empty() {
            self.status = "No prior search".into();
            return;
        }

        if self.editor.find_prev(&self.last_search_query) {
            self.status = format!("Found previous: {}", self.last_search_query);
            self.ensure_cursor_visible();
        } else {
            self.status = format!("Not found: {}", self.last_search_query);
        }
    }

    fn move_conflict_hunk(&mut self, direction: i32) {
        let Some(conflict) = self.editor.conflict() else {
            self.status = "No conflict hunks".into();
            return;
        };
        if conflict.hunks.is_empty() {
            self.status = "No conflict hunks".into();
            return;
        }

        let len = conflict.hunks.len();
        if direction < 0 {
            if self.selected_conflict_hunk == 0 {
                self.selected_conflict_hunk = len - 1;
            } else {
                self.selected_conflict_hunk -= 1;
            }
        } else {
            self.selected_conflict_hunk = (self.selected_conflict_hunk + 1) % len;
        }
        self.status = format!("Conflict hunk {}/{}", self.selected_conflict_hunk + 1, len);
    }

    fn apply_selected_conflict_hunk(&mut self) {
        if !self.editor.is_conflicted() {
            self.status = "No conflict hunks".into();
            return;
        }

        if self.editor.apply_external_hunk(self.selected_conflict_hunk) {
            self.sync_conflict_hunk_selection();
            self.ensure_cursor_visible();
            if self.editor.is_conflicted() {
                self.status = "Applied external hunk".into();
            } else {
                self.status = "Resolved conflict from hunks".into();
            }
        } else {
            self.status = "No conflict hunks".into();
        }
    }

    fn sync_conflict_hunk_selection(&mut self) {
        let Some(conflict) = self.editor.conflict() else {
            self.selected_conflict_hunk = 0;
            return;
        };
        if conflict.hunks.is_empty() {
            self.selected_conflict_hunk = 0;
        } else {
            self.selected_conflict_hunk = self
                .selected_conflict_hunk
                .min(conflict.hunks.len().saturating_sub(1));
        }
    }

    fn build_preview_lines(&self, preview_width: u16) -> (Vec<String>, Option<usize>) {
        let mut preview_lines = render_preview_lines(self.editor.text(), preview_width);
        let mut selected_anchor = None;

        if let Some(conflict) = self.editor.conflict() {
            preview_lines.push(String::new());
            if conflict.hunks.is_empty() {
                preview_lines.push("--- Local block ---".into());
                preview_lines.extend(self.editor.text().lines().map(ToString::to_string));
                preview_lines.push("--- External block ---".into());
                preview_lines.extend(conflict.external.lines().map(ToString::to_string));
            } else {
                for (idx, hunk) in conflict.hunks.iter().enumerate() {
                    let selected = idx == self.selected_conflict_hunk;
                    if selected {
                        selected_anchor = Some(preview_lines.len());
                        preview_lines
                            .push(format!(">>> Local block @L{} <<<", hunk.local_start + 1));
                    } else {
                        preview_lines
                            .push(format!("--- Local block @L{} ---", hunk.local_start + 1));
                    }
                    if hunk.local_lines.is_empty() {
                        preview_lines.push("(no local lines)".into());
                    } else {
                        preview_lines.extend(hunk.local_lines.iter().cloned());
                    }
                    if selected {
                        preview_lines.push(format!(
                            ">>> External block @L{} <<<",
                            hunk.external_start + 1
                        ));
                    } else {
                        preview_lines.push(format!(
                            "--- External block @L{} ---",
                            hunk.external_start + 1
                        ));
                    }
                    if hunk.external_lines.is_empty() {
                        preview_lines.push("(no external lines)".into());
                    } else {
                        preview_lines.extend(hunk.external_lines.iter().cloned());
                    }
                    preview_lines.push(String::new());
                }
            }
        }

        (preview_lines, selected_anchor)
    }

    fn preview_cache_key(&self, preview_width: u16) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        preview_width.hash(&mut hasher);
        self.selected_conflict_hunk.hash(&mut hasher);
        self.editor.text().hash(&mut hasher);
        if let Some(conflict) = self.editor.conflict() {
            conflict.external.hash(&mut hasher);
            for hunk in &conflict.hunks {
                hunk.local_start.hash(&mut hasher);
                hunk.external_start.hash(&mut hasher);
                for line in &hunk.local_lines {
                    line.hash(&mut hasher);
                }
                for line in &hunk.external_lines {
                    line.hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }

    fn preview_lines_cached(&mut self, preview_width: u16) -> (Arc<Vec<String>>, Option<usize>) {
        let key = self.preview_cache_key(preview_width);
        if let Some(cache) = &self.preview_cache
            && cache.key == key
        {
            #[cfg(test)]
            {
                self.test_preview_cache_hits += 1;
            }
            return (Arc::clone(&cache.lines), cache.selected_anchor);
        }

        let (lines, selected_anchor) = self.build_preview_lines(preview_width);
        self.preview_cache = Some(PreviewCache {
            key,
            lines: Arc::new(lines),
            selected_anchor,
        });
        #[cfg(test)]
        {
            self.test_preview_cache_misses += 1;
        }
        (
            Arc::clone(
                &self
                    .preview_cache
                    .as_ref()
                    .expect("preview cache populated")
                    .lines,
            ),
            selected_anchor,
        )
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        self.term_width = area.width.max(1);
        let theme = build_theme(self.ui.theme, self.ui.no_color);

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        let info = self.info_line(vertical[0].width as usize);
        frame.render_widget(Paragraph::new(info).style(theme.top_bar), vertical[0]);

        let pane_layout = compute_pane_layout(vertical[1], self.ui.focus);
        let compact = pane_layout.kind == LayoutKind::Compact;

        if self.home_mode {
            self.draw_home(frame, vertical[1], &theme);
        } else {
            if pane_layout.editor.width > 0 && pane_layout.editor.height > 0 {
                let editor_height = pane_layout.editor.height.saturating_sub(2) as usize;
                self.editor_height = editor_height.max(1);
                let editor_lines = to_lines(self.editor.text());
                self.editor_scroll =
                    clamp_scroll(self.editor_scroll, editor_lines.len(), self.editor_height);
                let editor_visible =
                    slice_lines(&editor_lines, self.editor_scroll, self.editor_height);

                let editor_border = pane_border_style(&theme, self.ui.focus == PaneFocus::Editor);
                let editor = Paragraph::new(editor_visible).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Editor")
                        .border_style(editor_border),
                );
                frame.render_widget(editor, pane_layout.editor);
            }

            if pane_layout.preview.width > 0 && pane_layout.preview.height > 0 {
                let preview_height = pane_layout.preview.height.saturating_sub(2) as usize;
                self.preview_height = preview_height.max(1);
                let preview_width = pane_layout.preview.width.saturating_sub(2);
                let (preview_lines, selected_anchor) = self.preview_lines_cached(preview_width);

                if let Some(anchor) = selected_anchor {
                    if anchor < self.preview_scroll {
                        self.preview_scroll = anchor;
                    } else if anchor >= self.preview_scroll + preview_height.max(1) {
                        self.preview_scroll = anchor.saturating_sub(preview_height / 2);
                    }
                }
                self.preview_scroll = clamp_scroll(
                    self.preview_scroll,
                    preview_lines.len(),
                    self.preview_height,
                );

                let mut in_code = code_open_before(preview_lines.as_ref(), self.preview_scroll);
                let preview_visible = preview_lines
                    .iter()
                    .skip(self.preview_scroll)
                    .take(self.preview_height)
                    .map(|line| styled_preview_line(line, preview_width, &theme, &mut in_code))
                    .collect::<Vec<_>>();

                let preview_title =
                    preview_title(self.selected_conflict_hunk, self.editor.conflict());
                let preview_border = pane_border_style(&theme, self.ui.focus == PaneFocus::Preview);
                let preview = Paragraph::new(preview_visible)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(preview_title)
                            .border_style(preview_border),
                    )
                    .wrap(Wrap { trim: false });
                frame.render_widget(preview, pane_layout.preview);
            }
        }

        let mut base_status = status_line(
            &self.status,
            self.perf_mode,
            self.draw_time_us,
            self.watch_event_count,
            self.stream_event_count,
        );
        if compact {
            base_status = format!("compact terminal (<80x24) | {base_status}");
        }
        let right_hint = self.status_hint();
        let status_text = compose_status(&base_status, &right_hint, vertical[2].width as usize);
        frame.render_widget(
            Paragraph::new(status_text).style(status_style(
                &theme,
                self.editor.is_conflicted(),
                &self.status,
            )),
            vertical[2],
        );

        if self.ui.help_open {
            let popup = centered_popup(76, 13, area);
            frame.render_widget(Clear, popup);
            let help_widget = Paragraph::new(help::help_text()).style(theme.help).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Settings")
                    .border_style(theme.pane_focus),
            );
            frame.render_widget(help_widget, popup);
        }

        if !self.ui.help_open {
            if self.home_mode {
                let popup = centered_popup(72, 12, vertical[1]);
                if popup.width > 10 && popup.height > 7 {
                    let x = popup
                        .x
                        .saturating_add(1)
                        .saturating_add(6)
                        .saturating_add(self.home_query.chars().count() as u16)
                        .min(popup.x + popup.width.saturating_sub(2));
                    let y = popup.y + 5;
                    frame.set_cursor_position((x, y));
                }
            } else if !self.readonly
                && self.ui.focus == PaneFocus::Editor
                && pane_layout.editor.width > 0
            {
                let cursor = cursor_rect(pane_layout.editor);
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

    fn draw_home(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &ThemeTokens) {
        let popup = centered_popup(72, 12, area);
        frame.render_widget(Clear, popup);
        let lines = vec![
            Line::from(Span::styled("mdv", theme.heading)),
            Line::from("Terminal-first markdown visualizer/editor"),
            Line::from(""),
            Line::from(Span::styled("Open Markdown File", theme.heading)),
            Line::from(format!("Path: {}", self.home_query)),
            Line::from(""),
            Line::from("Enter open | Ctrl+/ settings | Ctrl+Q quit"),
            Line::from("Tip: Use --focus view to start in preview mode"),
        ];
        let home = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Home")
                .border_style(theme.pane_focus),
        );
        frame.render_widget(home, popup);
    }

    fn info_line(&self, width: usize) -> String {
        let path = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| {
                if self.home_mode {
                    "<home>".into()
                } else {
                    "<stream>".into()
                }
            });
        let mode = mode_label(self);
        let ro = if self.readonly { "RO" } else { "RW" };
        let dirty = if self.editor.dirty { "dirty" } else { "clean" };
        let view_mode = match self.ui.focus {
            PaneFocus::Editor => "editor",
            PaneFocus::Preview => "view",
        };
        let line = format!(
            "{} | {ro} | {dirty} | mode={mode} | view={view_mode}",
            truncate_middle(&path, width.saturating_sub(32).max(12))
        );
        if line.chars().count() > width {
            truncate_middle(&line, width).into_owned()
        } else {
            line
        }
    }

    fn status_hint(&self) -> String {
        let base = if self.replace_find_mode {
            "replace: enter find"
        } else if self.replace_with_mode {
            "replace: enter with | Ctrl+A all"
        } else if self.search_mode {
            "search: Enter apply"
        } else if self.goto_mode {
            "goto: Enter apply"
        } else if self.home_mode {
            "home: type path + Enter"
        } else if self.ui.help_open {
            "Esc close settings"
        } else {
            "Tab mode | Ctrl+/ settings"
        };

        if let Some(conflict) = self.editor.conflict()
            && !conflict.hunks.is_empty()
        {
            return format!(
                "{base} | hunk {}/{}",
                self.selected_conflict_hunk + 1,
                conflict.hunks.len()
            );
        }
        base.into()
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

fn mode_label(app: &App) -> &'static str {
    if app.home_mode {
        "home"
    } else if app.replace_find_mode || app.replace_with_mode {
        "replace"
    } else if app.search_mode {
        "search"
    } else if app.goto_mode {
        "goto"
    } else if app.stream_mode {
        "stream"
    } else if app.editor.is_conflicted() {
        "conflict"
    } else {
        "normal"
    }
}

fn pane_border_style(theme: &ThemeTokens, focused: bool) -> Style {
    if focused {
        theme.pane_focus
    } else {
        theme.pane_border
    }
}

fn status_style(theme: &ThemeTokens, conflicted: bool, status: &str) -> Style {
    if status.contains("error") || status.contains("Error") {
        return theme.status_error;
    }
    if conflicted || status.contains("conflict") {
        return theme.status_warn;
    }
    theme.status_ok
}

fn preview_title(
    selected_conflict_hunk: usize,
    conflict: Option<&mdv_core::ConflictState>,
) -> String {
    if let Some(conflict) = conflict
        && !conflict.hunks.is_empty()
    {
        return format!(
            "Preview [conflict {}/{}]",
            selected_conflict_hunk + 1,
            conflict.hunks.len()
        );
    }
    "Preview".into()
}

fn styled_preview_line(
    line: &str,
    width: u16,
    theme: &ThemeTokens,
    in_code_block: &mut bool,
) -> Line<'static> {
    if line.trim_start().starts_with("```") || line.trim() == "$$" {
        *in_code_block = !*in_code_block;
        return Line::from(Span::styled(
            line.to_string(),
            style_for_segment(theme, SegmentKind::Code),
        ));
    }
    if *in_code_block {
        return Line::from(Span::styled(
            line.to_string(),
            style_for_segment(theme, SegmentKind::Code),
        ));
    }

    let kind = if line.contains("Local block") {
        SegmentKind::ConflictLocal
    } else if line.contains("External block") {
        SegmentKind::ConflictExternal
    } else {
        render_preview_segments(line, width)
            .first()
            .and_then(|l| l.segments.first())
            .map(|s| s.kind)
            .unwrap_or(SegmentKind::Plain)
    };

    if kind == SegmentKind::ListBullet {
        let (bullet, rest) = if let Some(stripped) = line.strip_prefix("- ") {
            ("- ".to_string(), stripped.to_string())
        } else {
            match line.split_once(". ") {
                Some((lhs, rhs)) if lhs.chars().all(|c| c.is_ascii_digit()) => {
                    (format!("{lhs}. "), rhs.to_string())
                }
                _ => (line.to_string(), String::new()),
            }
        };
        let mut spans = Vec::new();
        spans.push(Span::styled(
            bullet,
            style_for_segment(theme, SegmentKind::ListBullet),
        ));
        if !rest.is_empty() {
            spans.push(Span::styled(
                rest,
                style_for_segment(theme, SegmentKind::Plain),
            ));
        }
        return Line::from(spans);
    }

    Line::from(Span::styled(
        line.to_string(),
        style_for_segment(theme, kind),
    ))
}

fn code_open_before(lines: &[String], scroll: usize) -> bool {
    let mut open = false;
    for line in lines.iter().take(scroll) {
        if line.trim_start().starts_with("```") || line.trim() == "$$" {
            open = !open;
        }
    }
    open
}

fn centered_popup(width_percent: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width.saturating_mul(width_percent) / 100;
    let popup_height = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width.max(10),
        height: popup_height.max(3),
    }
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

fn next_terminal_input<P, R>(mut poll: P, mut read: R) -> Result<Option<InputEvent>>
where
    P: FnMut(Duration) -> io::Result<bool>,
    R: FnMut() -> io::Result<Event>,
{
    if !poll(Duration::from_millis(30))? {
        return Ok(None);
    }
    let event = read()?;
    match event {
        Event::Key(key) if key.kind == event::KeyEventKind::Press => Ok(Some(InputEvent::Key(key))),
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollUp => {
            Ok(Some(InputEvent::ScrollUp))
        }
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollDown => {
            Ok(Some(InputEvent::ScrollDown))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
fn next_pressed_key<P, R>(poll: P, read: R) -> Result<Option<KeyEvent>>
where
    P: FnMut(Duration) -> io::Result<bool>,
    R: FnMut() -> io::Result<Event>,
{
    match next_terminal_input(poll, read)? {
        Some(InputEvent::Key(key)) => Ok(Some(key)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crossterm::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseEvent,
        MouseEventKind,
    };
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::stream::StreamMessage;
    use crate::ui::theme::build_theme;
    use crate::watcher::WatchMessage;

    use super::{
        App, InputEvent, PaneFocus, ThemeChoice, centered_popup, clamp_scroll, code_open_before,
        cursor_rect, mode_label, next_pressed_key, next_terminal_input, pane_border_style,
        preview_title, slice_lines, status_line, status_style, styled_preview_line, to_lines,
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
    fn handle_key_repeat_search_next_prev() {
        let path = temp_path("search-repeat");
        fs::write(&path, "one two one").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "one two one".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::F(3), KeyModifiers::NONE), &mut running)
            .expect("repeat empty");
        assert_eq!(app.status, "No prior search");

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

        app.handle_key(key(KeyCode::F(3), KeyModifiers::NONE), &mut running)
            .expect("repeat next");
        assert_eq!(app.editor.cursor(), 8);
        assert_eq!(app.status, "Found next: one");

        app.handle_key(key(KeyCode::F(3), KeyModifiers::SHIFT), &mut running)
            .expect("repeat prev");
        assert_eq!(app.editor.cursor(), 0);
        assert_eq!(app.status, "Found previous: one");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_replace_mode_replace_next_and_all() {
        let path = temp_path("replace-mode");
        fs::write(&path, "one two one").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "one two one".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('h'), KeyModifiers::CONTROL), &mut running)
            .expect("start replace");
        app.handle_key(key(KeyCode::Char('o'), KeyModifiers::NONE), &mut running)
            .expect("find1");
        app.handle_key(key(KeyCode::Char('n'), KeyModifiers::NONE), &mut running)
            .expect("find2");
        app.handle_key(key(KeyCode::Char('e'), KeyModifiers::NONE), &mut running)
            .expect("find3");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("to replacement");
        assert_eq!(app.status, "Replace with: ");

        app.handle_key(key(KeyCode::Char('O'), KeyModifiers::SHIFT), &mut running)
            .expect("rep1");
        app.handle_key(key(KeyCode::Char('N'), KeyModifiers::SHIFT), &mut running)
            .expect("rep2");
        app.handle_key(key(KeyCode::Char('E'), KeyModifiers::SHIFT), &mut running)
            .expect("rep3");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("replace next");
        assert_eq!(app.editor.text(), "ONE two one");
        assert_eq!(app.status, "Replaced: one -> ONE");

        app.handle_key(key(KeyCode::Char('h'), KeyModifiers::CONTROL), &mut running)
            .expect("start replace all");
        app.handle_key(key(KeyCode::Char('o'), KeyModifiers::NONE), &mut running)
            .expect("find all1");
        app.handle_key(key(KeyCode::Char('n'), KeyModifiers::NONE), &mut running)
            .expect("find all2");
        app.handle_key(key(KeyCode::Char('e'), KeyModifiers::NONE), &mut running)
            .expect("find all3");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("to replacement all");
        app.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE), &mut running)
            .expect("replacement all");
        app.handle_key(key(KeyCode::Char('a'), KeyModifiers::CONTROL), &mut running)
            .expect("replace all");
        assert_eq!(app.editor.text(), "ONE two x");
        assert_eq!(app.status, "Replaced all 1: one -> x");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn handle_key_replace_mode_readonly_and_cancel() {
        let path = temp_path("replace-readonly");
        fs::write(&path, "one").expect("seed");
        let mut app = App::new_file(path.clone(), true, false, false, "one".into()).expect("app");
        app.interactive_input = false;
        let mut running = true;

        app.handle_key(key(KeyCode::Char('h'), KeyModifiers::CONTROL), &mut running)
            .expect("start replace");
        app.handle_key(key(KeyCode::Char('o'), KeyModifiers::NONE), &mut running)
            .expect("find");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("to replacement");
        app.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE), &mut running)
            .expect("rep");
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("replace blocked");
        assert_eq!(app.status, "Readonly: replace disabled");
        assert_eq!(app.editor.text(), "one");

        app.handle_key(key(KeyCode::Char('h'), KeyModifiers::CONTROL), &mut running)
            .expect("start replace 2");
        app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE), &mut running)
            .expect("cancel replace");
        assert_eq!(app.status, "Replace cancelled");

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

        let mut app3 = App::new_file(path.clone(), false, false, false, "x".into()).expect("app3");
        app3.interactive_input = false;
        let mut running3 = true;
        app3.handle_key(
            key(KeyCode::Char('h'), KeyModifiers::CONTROL),
            &mut running3,
        )
        .expect("replace mode");
        app3.handle_key(
            key(KeyCode::Char('q'), KeyModifiers::CONTROL),
            &mut running3,
        )
        .expect("quit replace mode");
        assert!(!running3);

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
    fn handle_key_conflict_hunk_nav_and_apply() {
        let path = temp_path("conflict-hunk-nav");
        fs::write(&path, "a\nb\nc").expect("seed");
        let mut app =
            App::new_file(path.clone(), false, false, false, "a\nb\nc".into()).expect("app");
        app.editor.dirty = true;
        app.editor.on_external_change("a\nB\nc\nd".into());
        assert!(app.editor.is_conflicted());
        assert_eq!(app.editor.conflict().expect("conflict").hunks.len(), 2);

        let mut running = true;
        app.handle_key(key(KeyCode::Char('j'), KeyModifiers::CONTROL), &mut running)
            .expect("next hunk");
        assert_eq!(app.status, "Conflict hunk 2/2");

        app.handle_key(key(KeyCode::Char('e'), KeyModifiers::CONTROL), &mut running)
            .expect("apply hunk");
        assert_eq!(app.status, "Applied external hunk");
        assert!(app.editor.text().contains("\nd"));
        assert_eq!(app.editor.conflict().expect("conflict").hunks.len(), 1);

        app.handle_key(key(KeyCode::Char('u'), KeyModifiers::CONTROL), &mut running)
            .expect("prev hunk");
        assert_eq!(app.status, "Conflict hunk 1/1");

        app.handle_key(key(KeyCode::Char('e'), KeyModifiers::CONTROL), &mut running)
            .expect("apply final hunk");
        assert_eq!(app.status, "Resolved conflict from hunks");
        assert!(!app.editor.is_conflicted());

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
        assert_eq!(app.status, "Ready");

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
            "External update conflict: Ctrl+J/Ctrl+U hunk | Ctrl+E apply | Ctrl+K keep | Ctrl+R reload | Ctrl+M merge"
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

        tx.send(StreamMessage::Update {
            text: "one".into(),
            truncated: false,
        })
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
    fn handle_stream_updates_sets_truncated_status() {
        let mut app = App::new_stream_for_test(false);
        app.interactive_input = false;
        let (tx, rx) = mpsc::channel();
        app.stream_rx = Some(rx);

        tx.send(StreamMessage::Update {
            text: "trimmed".into(),
            truncated: true,
        })
        .expect("send update");
        app.handle_stream_updates();
        assert_eq!(app.editor.text(), "trimmed");
        assert_eq!(app.status, "stream update received (trimmed)");
    }

    #[test]
    fn draw_renders_conflict_blocks() {
        let path = temp_path("draw");
        fs::write(&path, "a\nb").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, true, "a\nb".into()).expect("app");
        app.set_initial_focus(PaneFocus::Preview);
        app.editor.insert_char('!');
        app.editor.on_external_change("a\nB\nc".into());
        app.editor_scroll = 0;
        app.preview_scroll = 0;

        let backend = TestBackend::new(120, 30);
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

        let backend = TestBackend::new(120, 30);
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
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
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
    fn tab_switches_mode_and_updates_status() {
        let path = temp_path("mode-toggle");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        let mut running = true;

        app.handle_key(key(KeyCode::Tab, KeyModifiers::NONE), &mut running)
            .expect("tab");

        assert_eq!(app.status, "Mode: view");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn settings_overlay_blocks_edit_keys_until_closed() {
        let path = temp_path("settings-overlay");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        let mut running = true;

        app.handle_key(key(KeyCode::Char('/'), KeyModifiers::CONTROL), &mut running)
            .expect("open settings");
        assert_eq!(app.status, "Settings opened");

        app.handle_key(key(KeyCode::Char('z'), KeyModifiers::NONE), &mut running)
            .expect("type while settings open");
        assert_eq!(app.editor.text(), "x");

        app.handle_key(key(KeyCode::Char('/'), KeyModifiers::CONTROL), &mut running)
            .expect("close settings");
        app.handle_key(key(KeyCode::Char('z'), KeyModifiers::NONE), &mut running)
            .expect("type after close");
        assert_eq!(app.editor.text(), "xz");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn page_scroll_in_view_mode_moves_viewport_not_cursor() {
        let path = temp_path("view-scroll");
        let text = (0..80)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, &text).expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, text).expect("app");
        let mut running = true;

        app.handle_key(key(KeyCode::Tab, KeyModifiers::NONE), &mut running)
            .expect("to view");
        let cursor_before = app.editor.line_col_at_cursor().0;

        app.handle_key(key(KeyCode::PageDown, KeyModifiers::NONE), &mut running)
            .expect("page down");

        assert!(app.preview_scroll > 0);
        assert_eq!(app.editor.line_col_at_cursor().0, cursor_before);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn wheel_scroll_in_view_mode_moves_viewport_not_cursor() {
        let path = temp_path("wheel-view-scroll");
        let text = (0..80)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&path, &text).expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, text).expect("app");
        let mut running = true;

        app.handle_key(key(KeyCode::Tab, KeyModifiers::NONE), &mut running)
            .expect("to view");
        let cursor_before = app.editor.line_col_at_cursor().0;
        app.preview_height = 12;

        app.scroll_active_viewport(1);

        assert!(app.preview_scroll > 0);
        assert_eq!(app.editor.line_col_at_cursor().0, cursor_before);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn run_loop_handles_stream_done_non_interactive() {
        let mut app = App::new_stream_for_test(false);
        app.interactive_input = false;
        app.stream_done = true;
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
        app.run_loop(&mut terminal).expect("run loop stream done");
    }

    #[test]
    fn run_loop_interactive_consumes_queued_key() {
        let path = temp_path("interactive-loop");
        fs::write(&path, "x").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "x".into()).expect("app");
        app.interactive_input = true;
        app.test_next_key = Some(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
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

        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
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

        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
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
    fn next_terminal_input_maps_mouse_scroll() {
        let up = next_terminal_input(
            |_| Ok(true),
            || {
                Ok(Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column: 1,
                    row: 1,
                    modifiers: KeyModifiers::NONE,
                }))
            },
        )
        .expect("scroll up");
        assert_eq!(up, Some(InputEvent::ScrollUp));

        let down = next_terminal_input(
            |_| Ok(true),
            || {
                Ok(Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 1,
                    row: 1,
                    modifiers: KeyModifiers::NONE,
                }))
            },
        )
        .expect("scroll down");
        assert_eq!(down, Some(InputEvent::ScrollDown));
    }

    #[test]
    fn next_key_event_falls_back_to_terminal_poll() {
        let mut app = App::new_stream_for_test(false);
        app.test_next_key = None;
        let _ = app.next_input_event();
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
        tx.send(StreamMessage::Update {
            text: "same".into(),
            truncated: false,
        })
        .expect("send");
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

    #[test]
    fn draw_preview_cache_hits_and_invalidates() {
        let path = temp_path("preview-cache");
        fs::write(&path, "one").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "one".into()).expect("app");
        app.set_initial_focus(PaneFocus::Preview);
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");

        terminal.draw(|frame| app.draw(frame)).expect("first draw");
        assert_eq!(app.test_preview_cache_misses, 1);
        assert_eq!(app.test_preview_cache_hits, 0);

        terminal.draw(|frame| app.draw(frame)).expect("second draw");
        assert_eq!(app.test_preview_cache_misses, 1);
        assert_eq!(app.test_preview_cache_hits, 1);

        let mut running = true;
        app.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE), &mut running)
            .expect("edit");
        terminal.draw(|frame| app.draw(frame)).expect("third draw");
        assert_eq!(app.test_preview_cache_misses, 2);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn preview_cache_reuses_arc_on_cache_hit() {
        let path = temp_path("preview-cache-arc");
        fs::write(&path, "one").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "one".into()).expect("app");
        let (first, _) = app.preview_lines_cached(40);
        let (second, _) = app.preview_lines_cached(40);
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn app_setters_apply_ui_prefs() {
        let mut app = App::new_stream_for_test(false);
        app.set_theme(ThemeChoice::HighContrast);
        app.set_no_color(true);
        app.set_initial_focus(PaneFocus::Preview);
        assert_eq!(app.ui.theme, ThemeChoice::HighContrast);
        assert!(app.ui.no_color);
        assert_eq!(app.ui.focus, PaneFocus::Preview);
    }

    #[test]
    fn new_home_builds_home_mode_app() {
        let app = App::new_home_for_test(false, false, false);
        assert!(app.home_mode);
        assert_eq!(app.path, None);
        assert_eq!(mode_label(&app), "home");
    }

    #[test]
    fn home_query_open_enters_editor_mode() {
        let path = temp_path("home-open");
        fs::write(&path, "# h\nx\n").expect("seed");
        let mut app = App::new_home_for_test(false, false, false);
        let mut running = true;

        for c in path.display().to_string().chars() {
            app.handle_key(key(KeyCode::Char(c), KeyModifiers::NONE), &mut running)
                .expect("type path");
        }
        app.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut running)
            .expect("open");

        assert!(!app.home_mode);
        assert!(app.path.is_some());
        assert!(app.editor.text().contains("# h"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn draw_home_renders_branding_and_search_prompt() {
        let mut app = App::new_home_for_test(false, false, false);
        app.home_query = "/tmp/x.md".into();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");
        let rendered = terminal.backend().buffer().content();
        assert!(rendered.iter().any(|cell| cell.symbol() == "m"));
        assert!(rendered.iter().any(|cell| cell.symbol() == "P"));
    }

    #[test]
    fn helper_mode_and_status_branches() {
        let mut app = App::new_stream_for_test(false);
        assert_eq!(mode_label(&app), "stream");
        assert_eq!(app.status_hint(), "Tab mode | Ctrl+/ settings");

        app.search_mode = true;
        assert_eq!(mode_label(&app), "search");
        assert_eq!(app.status_hint(), "search: Enter apply");

        app.search_mode = false;
        app.goto_mode = true;
        assert_eq!(mode_label(&app), "goto");
        assert_eq!(app.status_hint(), "goto: Enter apply");

        app.goto_mode = false;
        app.replace_find_mode = true;
        assert_eq!(mode_label(&app), "replace");
        assert_eq!(app.status_hint(), "replace: enter find");

        app.replace_find_mode = false;
        app.ui.help_open = true;
        assert_eq!(app.status_hint(), "Esc close settings");
    }

    #[test]
    fn helper_info_line_title_and_styles() {
        let path = temp_path("helpers");
        fs::write(&path, "a\nb").expect("seed");
        let mut app = App::new_file(path.clone(), false, false, false, "a\nb".into()).expect("app");
        app.editor.insert_char('!');
        app.editor.on_external_change("a\nB\nc".into());

        let info = app.info_line(140);
        assert!(info.contains("mode=conflict"));
        assert!(info.contains("view=editor"));

        let title = preview_title(0, app.editor.conflict());
        assert!(title.contains("Preview [conflict 1/"));
        assert_eq!(preview_title(0, None), "Preview");

        let theme = build_theme(ThemeChoice::Default, false);
        assert_eq!(pane_border_style(&theme, true).fg, theme.pane_focus.fg);
        assert_eq!(pane_border_style(&theme, false).fg, theme.pane_border.fg);
        assert_eq!(
            status_style(&theme, false, "watch error: x").fg,
            theme.status_error.fg
        );
        assert_eq!(status_style(&theme, true, "ok").fg, theme.status_warn.fg);
        assert_eq!(status_style(&theme, false, "ok").fg, theme.status_ok.fg);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn helper_preview_line_code_and_list_paths() {
        let theme = build_theme(ThemeChoice::Default, false);
        let mut in_code = false;

        let fence = styled_preview_line("```rs", 80, &theme, &mut in_code);
        assert!(in_code);
        assert_eq!(fence.spans[0].content, "```rs");

        let code = styled_preview_line("let x = 1;", 80, &theme, &mut in_code);
        assert_eq!(code.spans[0].content, "let x = 1;");

        let close = styled_preview_line("```", 80, &theme, &mut in_code);
        assert!(!in_code);
        assert_eq!(close.spans[0].content, "```");

        let bullet = styled_preview_line("- item", 80, &theme, &mut in_code);
        assert_eq!(bullet.spans[0].content, "- ");
        assert_eq!(bullet.spans[1].content, "item");

        let ordered = styled_preview_line("12. item", 80, &theme, &mut in_code);
        assert_eq!(ordered.spans[0].content, "12. ");
        assert_eq!(ordered.spans[1].content, "item");

        let malformed = styled_preview_line("xx. item", 80, &theme, &mut in_code);
        assert_eq!(malformed.spans[0].content, "xx. item");
    }

    #[test]
    fn helper_code_open_and_popup() {
        let lines = vec![
            "a".to_string(),
            "```".to_string(),
            "code".to_string(),
            "```".to_string(),
        ];
        assert!(!code_open_before(&lines, 1));
        assert!(code_open_before(&lines, 3));
        assert!(!code_open_before(&lines, 4));

        let popup = centered_popup(
            60,
            40,
            ratatui::layout::Rect {
                x: 1,
                y: 2,
                width: 100,
                height: 20,
            },
        );
        assert_eq!(popup.width, 60);
        assert_eq!(popup.height, 20);
        assert!(popup.x >= 1);
        assert!(popup.y >= 2);
    }
}
