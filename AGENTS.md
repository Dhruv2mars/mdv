# AGENTS.md

## Purpose
Build `mdv`: terminal-first markdown visualizer/editor in Rust.
Core diff: very fast startup/render/scroll, in-app edit, realtime file sync.

## Repo + Product
- Monorepo.
- Start with CLI/TUI.
- v1 default UX: split view (editor + preview), synced scroll.
- Markdown target: CommonMark + GFM.
- Realtime: if file changes externally, TUI updates live.
- Conflict policy (dirty buffer + external change): show inline stacked diff (old block then new block), let user choose keep/reload/merge.

## Distribution
- Primary install: npm.
- Package name: `@dhruv2mars/mdv`.
- Binary command exposed as: `mdv`.
- Build prebuilt Rust binaries per platform in GitHub Releases.
- npm package installs lightweight launcher; first run downloads matching release asset.

## CI/CD
- Use GitHub Actions.
- CI must run tests, coverage, lint/check, perf smoke checks.
- Release flow builds multi-platform binaries and publishes npm package.

## Package Releases
- Repo release flow must use Changesets.
- Release trigger is git tag `vX.Y.Z` only.
- Single release workflow: `.github/workflows/release.yml`.
- Workflow validates tag == `packages/cli` version, creates GitHub release, uploads binaries, publishes npm.
- npm auth mode: Trusted Publisher (OIDC) only.
- Do not run `npm publish` manually from local.

## References
- GitHub release workflow: `.github/workflows/release.yml`.
