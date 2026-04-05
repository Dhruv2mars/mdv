# mdv

mdv is a simple Markdown app for your terminal.

You can open a file, type notes on the left, and see the formatted preview on the right.
If you are new to TUIs, that is fine: `mdv` starts with a Home screen and an in-app guide.

## Install

```bash
npm i -g @dhruv2mars/mdv
```

First run downloads the native `mdv` binary into `~/.mdv/bin/mdv`.
Binary artifacts are published on GitHub Releases per platform.
Installer keeps verified cache under `~/.mdv/cache`.

Supported release binaries:
- `darwin-arm64`
- `darwin-x64`
- `linux-arm64`
- `linux-x64`
- `win32-arm64`
- `win32-x64`

## Quickstart

```bash
mdv
```

What happens next:
- Type a file name like `notes.md`
- Press `Enter`
- Start typing
- Press `Ctrl+S` to save

If the file does not exist yet, `mdv` creates it on your first save.

Open an existing file directly:

```bash
mdv README.md
```

Open the in-app docs any time:
- macOS: `Cmd+,`
- Windows/Linux: `Ctrl+,`

## If You Are New To Markdown

You do not need to know Markdown to start.
Plain text works.

Useful basics:
- `# Heading` makes a heading
- `- item` makes a bullet list
- A blank line starts a new paragraph

Example:

```md
# Shopping List

- milk
- bread
- fruit
```

## Everyday Tasks

Create a new note:
- Run `mdv`
- Type `notes/today.md`
- Press `Enter`
- Type your note
- Press `Ctrl+S`

Read a file without editing:
- Run `mdv --readonly README.md`

Watch streamed Markdown from another command:
```bash
tail -f notes.md | mdv --stream
```

Update the installed launcher:
```bash
mdv update
```
Uses the detected install manager and prefers the one that installed `mdv`.

## Keybinds

In-app docs: `Cmd+,/Ctrl+,` (Docs + Settings modal).

Quick ref:
- `Ctrl+Q` quit
- `Ctrl+S` save
- `Ctrl+R` reload from disk
- `Shift+Tab` or `Ctrl+T` switch between typing and preview scrolling
- `Ctrl+F` search, `Ctrl+H` replace, `Ctrl+G` goto line
- `F3`/`Shift+F3` next/prev search result
- Conflict flow: `Ctrl+J`/`Ctrl+U` hunk nav, `Ctrl+E` apply, `Ctrl+K` keep local, `Ctrl+M` merge

Beginner tip:
- If arrow keys or mouse wheel are moving the wrong side, press `Shift+Tab` to switch focus.

## Flags

- `--readonly` disable editing
- `--no-watch` disable file watcher
- `--stream` read markdown from stdin (no `PATH` arg)
- `--perf` show perf info in status line
- `--theme <auto|default|high-contrast>` set color theme
- `--no-color` disable ANSI color
- `--focus <editor|view>` initial focused pane

## Need Help?

- Start with `mdv` and follow the first-run guide
- Open in-app docs with `Cmd+,` / `Ctrl+,`
- Use `Ctrl+Q` to quit safely at any time

## Installer Env

- `MDV_INSTALL_DEBUG=1` local installer debug logs
- `MDV_INSTALL_TIMEOUT_MS` request timeout (default `15000`)
- `MDV_INSTALL_RETRY_ATTEMPTS` retries (default `3`)

## Dev/Contrib

See `CONTRIBUTING.md`.
