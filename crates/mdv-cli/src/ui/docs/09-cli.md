# CLI Usage

## Main Modes

- `mdv <path>` opens a file
- `mdv` starts on the Home screen
- `mdv --stream` renders Markdown coming from another command

Examples:
- `mdv notes.md`
- `mdv --readonly README.md`
- `tail -f notes.md | mdv --stream`

## Helpful Flags

- `--readonly` open without editing
- `--no-watch` ignore outside file changes
- `--theme <auto|default|high-contrast>` choose colors
- `--no-color` use plain terminal text
- `--focus <editor|view>` choose which pane starts focused
