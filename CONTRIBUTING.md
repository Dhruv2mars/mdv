# Contributing

## Rules

- TDD first: tests before behavior changes.
- Before every commit: run all tests + lint.
- Keep commits atomic with prefixes: `feat:`, `fix:`, `test:`.
- Use branch -> PR -> merge -> delete branch.

## Local checks

```bash
bun run lint
bun run test
```

Coverage:
```bash
bun run coverage
```

## Release changes

If change affects shipped package behavior, add a changeset:
```bash
bunx changeset
```

Version packages:
```bash
bun run release:version
```

Commit version files, then publish from tag:
```bash
TAG="v$(node -p \"require('./packages/cli/package.json').version\")"
git tag "$TAG"
git push origin "$TAG"
```

GitHub Release notes are auto-generated from merged PRs/commits when the tag release is created.
