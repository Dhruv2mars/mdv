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
- Ship prebuilt Rust binaries per platform via npm packages.

## CI/CD
- Use GitHub Actions.
- CI must run tests, coverage, lint/check, perf smoke checks.
- Release flow builds multi-platform binaries and publishes npm package.

## Package Releases
- Use `autoship` for releases.
- Repo release flow must use Changesets.
- Autoship already configured for this repo.
- Requirements at release time: `gh` auth, `AI_GATEWAY_API_KEY`.
- Autoship run (manual): `bun run release:autoship`.
- Autoship run (CI): `.github/workflows/autoship.yml` (push main + workflow_dispatch).
- Expected flow (per autoship): changeset PR -> wait CI -> merge -> version packages PR -> merge -> publish -> tag -> binaries.

## References
- Autoship repo: `https://github.com/vercel-labs/autoship`.
