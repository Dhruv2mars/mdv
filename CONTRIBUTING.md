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

Then run autoship:
```bash
bun run release:autoship
```
