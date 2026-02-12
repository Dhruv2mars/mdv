# mdv

Terminal markdown visualizer/editor (Rust TUI).

## Install

```bash
npm i -g @dhruv2mars/mdv
```

First run downloads the native `mdv` binary into `~/.mdv/bin/mdv`.
Binary artifacts are published on GitHub Releases per platform.
Installer keeps verified cache under `~/.mdv/cache`.

## Use

```bash
mdv README.md
```

Manual upgrade:
```bash
mdv update
```
Uses detected install manager (bun/pnpm/yarn/npm), prefers original installer metadata.

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

## Installer Env

- `MDV_INSTALL_DEBUG=1` local installer debug logs
- `MDV_INSTALL_TIMEOUT_MS` request timeout (default `15000`)
- `MDV_INSTALL_RETRY_ATTEMPTS` retries (default `3`)

## Dev/Contrib

See `CONTRIBUTING.md`.
