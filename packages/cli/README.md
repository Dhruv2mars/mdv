# @dhruv2mars/mdv

mdv is a simple Markdown app for your terminal.

Type on the left, read the preview on the right, and use the built-in guide if you are new.

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
mdv
```

Quick first run:
- Type a file name like `notes.md`
- Press `Enter`
- Type your note
- Press `Ctrl+S`

Open an existing file directly:

```bash
mdv README.md
```

If the file does not exist yet, `mdv` creates it on your first save.

Open the in-app docs any time:
- macOS: `Cmd+,`
- Windows/Linux: `Ctrl+,`

If you do not know Markdown yet, start with plain text.
Common basics:
- `# Heading`
- `- bullet item`
- blank line for a new paragraph

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

In-app docs: `Cmd+,/Ctrl+,` (Docs + Settings modal).

Quick ref:
- `Ctrl+Q` quit
- `Ctrl+S` save
- `Ctrl+R` reload
- `Shift+Tab` or `Ctrl+T` switch between typing and preview scrolling
- `Ctrl+F` search, `Ctrl+H` replace, `Ctrl+G` goto
- Conflict flow: `Ctrl+J`/`Ctrl+U` hunk nav, `Ctrl+E` apply, `Ctrl+K` keep local, `Ctrl+M` merge

Beginner tip:
- If scrolling is moving the preview instead of the editor, press `Shift+Tab`.

## Flags

- `--readonly` disable editing
- `--no-watch` disable file watch
- `--stream` read markdown from stdin
- `--perf` show perf stats

## Installer Env

- `MDV_INSTALL_DEBUG=1` local installer debug logs
- `MDV_INSTALL_TIMEOUT_MS` request timeout (default `15000`)
- `MDV_INSTALL_RETRY_ATTEMPTS` retries (default `3`)
