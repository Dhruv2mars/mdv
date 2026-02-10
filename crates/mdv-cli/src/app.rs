use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use mdv_core::{EditorBuffer, render_preview_lines};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use crate::watcher::{self, WatchMessage};

pub struct App {
    path: PathBuf,
    readonly: bool,
    watch_enabled: bool,
    perf_mode: bool,
    editor: EditorBuffer,
    status: String,
    _watcher: Option<notify::RecommendedWatcher>,
    watch_rx: Option<std::sync::mpsc::Receiver<WatchMessage>>,
    editor_scroll: usize,
    preview_scroll: usize,
    editor_height: usize,
    draw_time_us: u128,
    watch_event_count: u64,
}

impl App {
    pub fn new(
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
            path,
            readonly,
            watch_enabled,
            perf_mode,
            editor: EditorBuffer::new(initial_text),
            status: "Ctrl+Q quit | Ctrl+S save | Ctrl+R reload | Ctrl+K keep | Ctrl+M merge".into(),
            _watcher: watcher,
            watch_rx,
            editor_scroll: 0,
            preview_scroll: 0,
            editor_height: 1,
            draw_time_us: 0,
            watch_event_count: 0,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let loop_result = self.run_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        loop_result
    }

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut running = true;

        while running {
            self.handle_watch_updates();

            let started = Instant::now();
            terminal.draw(|frame| self.draw(frame))?;
            self.draw_time_us = started.elapsed().as_micros();

            if event::poll(Duration::from_millis(30))?
                && let Event::Key(key) = event::read()?
                && key.kind == event::KeyEventKind::Press
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

    fn handle_key(&mut self, key: KeyEvent, running: &mut bool) -> Result<()> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                *running = false;
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if self.readonly {
                    self.status = "Readonly: save disabled".into();
                } else {
                    self.editor.save_to_path(&self.path)?;
                    self.status = "Saved".into();
                }
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                if self.editor.is_conflicted() {
                    self.editor.reload_external();
                    self.status = "Reloaded external".into();
                } else {
                    let disk = fs::read_to_string(&self.path).unwrap_or_default();
                    self.editor.on_external_change(disk);
                    self.status = "Reloaded from disk".into();
                }
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.editor.keep_local();
                self.status = "Kept local".into();
            }
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                self.editor.merge_external();
                self.status = "Merged with conflict markers".into();
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
            preview_lines.push("--- Local (unsaved) ---".into());
            preview_lines.extend(self.editor.text().lines().map(ToString::to_string));
            preview_lines.push(String::new());
            preview_lines.push("--- External (updated below) ---".into());
            preview_lines.extend(conflict.external.lines().map(ToString::to_string));
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

fn status_line(base: &str, perf_mode: bool, draw_time_us: u128, watch_events: u64) -> String {
    if !perf_mode {
        return base.to_string();
    }

    format!("{base} | perf draw={draw_time_us}us watch_events={watch_events}")
}

fn to_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    text.lines().map(ToString::to_string).collect()
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

#[cfg(test)]
mod tests {
    use super::{clamp_scroll, slice_lines, status_line, to_lines};

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
        assert_eq!(status_line("ok", false, 12, 3), "ok");
        assert_eq!(
            status_line("ok", true, 12, 3),
            "ok | perf draw=12us watch_events=3"
        );
    }

    #[test]
    fn to_lines_returns_single_empty_line_for_empty_text() {
        assert_eq!(to_lines(""), vec![String::new()]);
    }
}
