mod app;
mod stream;
mod watcher;

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Parser;
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.stream {
        if cli.path.is_some() {
            bail!("path arg not allowed with --stream");
        }

        if !io::stdout().is_terminal() {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            let width = std::env::var("COLUMNS")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(80);
            let lines = render_preview_lines(&buf, width);
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    println!();
                }
                print!("{line}");
            }
            return Ok(());
        }

        let mut app = app::App::new_stream(cli.perf)?;
        return app.run();
    }

    let Some(path) = cli.path else {
        bail!("path required unless --stream used");
    };

    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut app = app::App::new_file(path, cli.readonly, !cli.no_watch, cli.perf, text)?;
    app.run()
}
