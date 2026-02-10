mod app;
mod watcher;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "mdv", about = "Terminal markdown visualizer")]
struct Cli {
    /// Markdown file path
    path: PathBuf,

    /// Disable editing
    #[arg(long)]
    readonly: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let text = fs::read_to_string(&cli.path).unwrap_or_default();
    let mut app = app::App::new(cli.path, cli.readonly, text)?;
    app.run()
}
