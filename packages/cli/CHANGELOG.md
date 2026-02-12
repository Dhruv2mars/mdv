# @dhruv2mars/mdv

## 0.0.18

### Patch Changes

- Fix stale global binary drift: launcher/installer now enforce installed binary version == package version.

## 0.0.17

### Patch Changes

- Show help + exit 0 when `mdv` runs with no args (instead of hard error).

## 0.0.16

### Patch Changes

- Fix runtime launcher crash on install path (`ReferenceError: join is not defined`) and add regression test.

## 0.0.15

### Patch Changes

- Make `mdv update` package-manager aware: prefer original installer manager (bun/pnpm/yarn/npm), fallback by env hint, then npm.

## 0.0.14

### Patch Changes

- Harden installer reliability (cache-first verified installs, tuned retries/timeouts, checksum recovery guidance, debug tracing) and add manual updater command `mdv update`.

## 0.0.13

### Patch Changes

- Improve runtime reliability and perf guardrails: stricter file-read/write error handling, bounded stream buffer, bounded undo history memory, and lower-copy preview cache path.

## 0.0.12

### Patch Changes

- Polish markdown math and footnote rendering with deterministic delimiters.

## 0.0.11

### Patch Changes

- Add Ctrl+H in-app replace prompt with replace-next and replace-all actions.

## 0.0.10

### Patch Changes

- Add repeat-search keys, conflict-hunk navigation/apply actions, and preview caching for faster redraws.

## 0.0.9

### Patch Changes

- Release v0.0.9 with editor workflow upgrades and markdown rendering improvements.

## 0.0.8

### Patch Changes

- Ship v1 alpha gap-closure: non-tty one-shot path mode, block conflict diff UX, expanded CommonMark/GFM rendering, stronger CI perf+coverage gates, and installer/release contract checks.

## 0.0.7

### Patch Changes

- Add package README so npm page shows user docs.

## 0.0.6

### Patch Changes

- Fix trailing blank line handling in editor line splitting.

## 0.0.5

### Patch Changes

- a3d917e: Release patch

## 0.0.4

### Patch Changes

- cee5368: Release patch
