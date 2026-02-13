# @dhruv2mars/mdv

Fast terminal markdown viewer/editor.

## Install

```bash
npm i -g @dhruv2mars/mdv
```

First run downloads native `mdv` binary.
Assets are resolved from GitHub Releases for your platform/arch.
Installer keeps verified cache under `~/.mdv/cache`.

Supported release binaries:
- `darwin-arm64`
- `darwin-x64`
- `linux-arm64`
- `linux-x64`
- `win32-arm64`
- `win32-x64`

## Usage

```bash
mdv README.md
```

Manual upgrade:

```bash
mdv update
```
Uses detected install manager (bun/pnpm/yarn/npm), prefers original installer metadata.

Stream stdin:

```bash
tail -f notes.md | mdv --stream
```

## Keybinds

- `Ctrl+Q` quit
- `Ctrl+S` save
- `Ctrl+R` reload
- `Ctrl+K` keep local on conflict
- `Ctrl+M` merge markers on conflict

## Flags

- `--readonly` disable editing
- `--no-watch` disable file watch
- `--stream` read markdown from stdin
- `--perf` show perf stats

## Installer Env

- `MDV_INSTALL_DEBUG=1` local installer debug logs
- `MDV_INSTALL_TIMEOUT_MS` request timeout (default `15000`)
- `MDV_INSTALL_RETRY_ATTEMPTS` retries (default `3`)
