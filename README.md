# mdv

Terminal markdown visualizer/editor (Rust TUI).

## Install

```bash
npm i -g @dhruv2mars/mdv
```

First run downloads the native `mdv` binary into `~/.mdv/bin/mdv`.

## Use

```bash
mdv README.md
```

Stream mode (stdin):
```bash
tail -f notes.md | mdv --stream
```

## Keys

- `Ctrl+Q` quit
- `Ctrl+S` save
- `Ctrl+R` reload from disk
- `Ctrl+K` keep local (on external change conflict)
- `Ctrl+M` insert merge markers (on conflict)

## Flags

- `--readonly` disable editing
- `--no-watch` disable file watcher
- `--stream` read markdown from stdin (no `PATH` arg)
- `--perf` show perf info in status line

## Dev/Contrib

See `CONTRIBUTING.md`.
