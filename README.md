# mdv

Terminal-first markdown visualizer/editor in Rust.

## Install

Global npm install:
```bash
npm i -g @dhruv2mars/mdv
```

Run:
```bash
mdv README.md
```

First run behavior:
- postinstall downloads prebuilt `mdv` into `~/.mdv/bin/mdv`
- `MDV_SKIP_DOWNLOAD=1` skips download

Overrides:
- `MDV_BIN`: force exact binary path
- `MDV_INSTALL_ROOT`: custom install root instead of `~/.mdv`

## Features

- split TUI: editor + preview
- live file-watch refresh
- conflict flow: keep/reload/merge
- stream mode for agent/log markdown pipes
- perf overlay mode

## Usage

File mode:
```bash
mdv ./notes.md
```

Stream mode:
```bash
tail -f AGENT_LOG.md | mdv --stream --perf
```

Dev run:
```bash
cargo run -p mdv-cli -- README.md
```

Flags:
- `--readonly`
- `--no-watch`
- `--perf`
- `--stream`

Keys:
- `Ctrl+Q` quit
- `Ctrl+S` save
- `Ctrl+R` reload
- `Ctrl+K` keep local
- `Ctrl+M` merge markers
- arrows move cursor
- `PageUp/PageDown` scroll panes

## Project Layout

- `crates/mdv-core`: editor + markdown core
- `crates/mdv-cli`: app/runtime/tui
- `packages/cli`: npm wrapper package (`@dhruv2mars/mdv`)

## Quality Gates

CI enforces:
- `cargo fmt --check`
- `cargo clippy -D warnings`
- `cargo test --workspace`
- coverage export (`cargo llvm-cov`)
- package smoke (`npm pack` + launcher help run)

## Releases

Versioning:
- early phase: `0.0.x`

Release flow:
- changesets + autoship
- autoship entry: `bun run release:autoship`
- CI release workflow publishes npm package from version PR merges (OIDC trusted publishing)
- binary assets built by `binaries` workflow on tag `vX.Y.Z`

One-time npm setup:
- In npm package settings for `@dhruv2mars/mdv`, add a Trusted Publisher for this GitHub repo + `release` workflow on `main`.
- After trust is configured, GitHub Actions publishes without long-lived npm tokens.

Required repo secrets:
- `AI_GATEWAY_API_KEY` (autoship runtime)

## References

- Autoship: https://github.com/vercel-labs/autoship
