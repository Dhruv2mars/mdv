# mdv

Terminal-first markdown visualizer/editor in Rust.

## Current scope
- Split TUI: editor + preview
- Live file-watch updates
- External-change conflict actions: keep/reload/merge
- npm package scaffold: `@dhruv2mars/mdv`

## Run (dev)
```bash
cargo run -p mdv-cli -- README.md
```

Or via npm workspace wrapper:
```bash
bun --cwd packages/cli run mdv -- README.md
```

Flags:
- `--readonly`
- `--no-watch`
- `--perf`
- `--stream` (read markdown continuously from stdin; readonly mode)

Stream example:
```bash
tail -f AGENT_LOG.md | cargo run -p mdv-cli -- --stream --perf
```

## Keys
- `Ctrl+Q` quit
- `Ctrl+S` save
- `Ctrl+R` reload external
- `Ctrl+K` keep local
- `Ctrl+M` merge conflict markers
- `↑/↓/←/→` move cursor
- `PageUp/PageDown` scroll panes
