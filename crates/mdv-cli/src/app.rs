use std::io;
use std::path::PathBuf;
use std::time::Duration;

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
    editor: EditorBuffer,
    status: String,
    watcher: notify::RecommendedWatcher,
    watch_rx: std::sync::mpsc::Receiver<WatchMessage>,
}

impl App {
    pub fn new(path: PathBuf, readonly: bool, initial_text: String) -> Result<Self> {
        let (watcher, watch_rx) = watcher::start(&path)?;
        Ok(Self {
            path,
            readonly,
            editor: EditorBuffer::new(initial_text),
            status: "Ctrl+Q quit | Ctrl+S save | Ctrl+R reload | Ctrl+K keep | Ctrl+M merge".into(),
            watcher,
            watch_rx,
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
            terminal.draw(|frame| self.draw(frame))?;

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
        let mut latest_external: Option<String> = None;

        while let Ok(msg) = self.watch_rx.try_recv() {
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
                self.editor.reload_external();
                self.status = "Reloaded external".into();
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

        Ok(())
    }

    fn draw(&self, frame: &mut Frame<'_>) {
        let _keep_watcher_alive = &self.watcher;
        let area = frame.area();

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(vertical[0]);

        let editor = Paragraph::new(self.editor.text())
            .block(Block::default().borders(Borders::ALL).title("Editor"));
        frame.render_widget(editor, panes[0]);

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

        let preview = Paragraph::new(preview_lines.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("Preview"));
        frame.render_widget(preview, panes[1]);

        let status_style = if self.editor.is_conflicted() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let status = Paragraph::new(self.status.as_str()).style(status_style);
        frame.render_widget(status, vertical[1]);

        if !self.readonly {
            let cursor = cursor_rect(panes[0]);
            let (line, col) = self.editor.line_col_at_cursor();
            let x = (cursor.x + col as u16).min(cursor.x + cursor.width.saturating_sub(1));
            let y = (cursor.y + line as u16).min(cursor.y + cursor.height.saturating_sub(1));
            frame.set_cursor_position((x, y));
        }
    }
}

fn cursor_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}
