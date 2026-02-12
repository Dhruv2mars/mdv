# @dhruv2mars/mdv

Fast terminal markdown viewer/editor.

## Install

```bash
npm i -g @dhruv2mars/mdv
```

First run downloads native `mdv` binary.
Assets are resolved from GitHub Releases for your platform/arch.

## Usage

```bash
mdv README.md
```

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
