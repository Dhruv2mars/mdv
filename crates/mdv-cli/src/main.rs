mod app;
mod stream;
mod ui;
mod watcher;

use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, ValueEnum};
use mdv_core::render_preview_lines;

#[derive(Debug, Parser)]
#[command(name = "mdv", about = "Terminal markdown visualizer")]
struct Cli {
    /// Markdown file path
    path: Option<PathBuf>,

    /// Stream markdown from stdin
    #[arg(long, default_value_t = false)]
    stream: bool,

    /// Disable editing
    #[arg(long)]
    readonly: bool,

    /// Disable file watcher
    #[arg(long, default_value_t = false)]
    no_watch: bool,

    /// Show perf info in status line
    #[arg(long, default_value_t = false)]
    perf: bool,

    /// Color theme
    #[arg(long, value_enum, default_value_t = CliTheme::Auto)]
    theme: CliTheme,

    /// Disable ANSI color
    #[arg(long, default_value_t = false)]
    no_color: bool,

    /// Initial view mode
    #[arg(long, value_enum, default_value_t = CliFocus::Editor)]
    focus: CliFocus,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum CliTheme {
    Auto,
    Default,
    HighContrast,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum CliFocus {
    Editor,
    #[value(alias = "preview")]
    View,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let force_tui = std::env::var("MDV_FORCE_TUI").ok().as_deref() == Some("1");

    if cli.stream {
        if cli.path.is_some() {
            bail!("path arg not allowed with --stream");
        }

        if !io::stdout().is_terminal() && !force_tui {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            print_preview(&buf)?;
            return Ok(());
        }

        let mut app = app::App::new_stream(cli.perf)?;
        apply_ui_flags(&mut app, cli.theme, cli.no_color, cli.focus);
        return app.run();
    }

    let Some(path) = cli.path else {
        if (!io::stdin().is_terminal() || !io::stdout().is_terminal()) && !force_tui {
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            return Ok(());
        }

        let mut app = app::App::new_home(cli.readonly, !cli.no_watch, cli.perf)?;
        apply_ui_flags(&mut app, cli.theme, cli.no_color, cli.focus);
        return app.run();
    };

    let text = read_initial_text(&path)?;
    if (!io::stdin().is_terminal() || !io::stdout().is_terminal()) && !force_tui {
        print_preview(&text)?;
        return Ok(());
    }

    let mut app = app::App::new_file(path, cli.readonly, !cli.no_watch, cli.perf, text)?;
    apply_ui_flags(&mut app, cli.theme, cli.no_color, cli.focus);
    app.run()
}

fn apply_ui_flags(app: &mut app::App, theme: CliTheme, no_color: bool, focus: CliFocus) {
    let theme = match theme {
        CliTheme::Auto => app::ThemeChoice::Auto,
        CliTheme::Default => app::ThemeChoice::Default,
        CliTheme::HighContrast => app::ThemeChoice::HighContrast,
    };
    let focus = match focus {
        CliFocus::Editor => app::PaneFocus::Editor,
        CliFocus::View => app::PaneFocus::Preview,
    };
    app.set_theme(theme);
    app.set_no_color(no_color);
    app.set_initial_focus(focus);
}

fn print_preview(text: &str) -> io::Result<()> {
    let width = preview_width_from_env();
    let stdout = io::stdout();
    let lock = stdout.lock();
    print_preview_to(text, width, io::BufWriter::new(lock))
}

fn preview_width_from_env() -> u16 {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(80)
}

fn read_initial_text(path: &PathBuf) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(err.into()),
    }
}

fn print_preview_to<W: Write>(text: &str, width: u16, mut out: W) -> io::Result<()> {
    let lines = render_preview_lines(text, width);
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.write_all(b"\n")?;
        }
        out.write_all(line.as_bytes())?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{preview_width_from_env, print_preview_to, read_initial_text};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("mdv-main-test-{name}-{nanos}.md"))
    }

    #[test]
    fn preview_width_from_env_handles_valid_invalid_and_missing() {
        let _guard = ENV_LOCK.lock().expect("env lock");

        unsafe { std::env::remove_var("COLUMNS") };
        assert_eq!(preview_width_from_env(), 80);

        unsafe { std::env::set_var("COLUMNS", "120") };
        assert_eq!(preview_width_from_env(), 120);

        unsafe { std::env::set_var("COLUMNS", "oops") };
        assert_eq!(preview_width_from_env(), 80);

        unsafe { std::env::remove_var("COLUMNS") };
    }

    #[test]
    fn read_initial_text_allows_missing_and_errors_on_dir() {
        let missing = temp_path("missing");
        assert_eq!(read_initial_text(&missing).expect("missing ok"), "");

        let dir = temp_path("dir");
        fs::create_dir(&dir).expect("mkdir");
        let err = read_initial_text(&dir).expect_err("dir err");
        assert!(!err.to_string().is_empty());
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn print_preview_to_writes_newline_separated_lines() {
        let mut out = Vec::new();
        print_preview_to("# a\nb\n", 80, &mut out).expect("print");
        let s = String::from_utf8(out).expect("utf8");
        assert_eq!(s, "# a\nb");

        let mut out2 = Vec::new();
        print_preview_to("", 80, &mut out2).expect("print2");
        assert_eq!(String::from_utf8(out2).expect("utf8"), "");
    }

    #[test]
    fn print_preview_to_propagates_write_errors() {
        struct FailWriter;
        impl io::Write for FailWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("write fail"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let err = print_preview_to("x", 80, FailWriter).expect_err("expected write err");
        assert!(err.to_string().contains("write fail"));
    }
}
