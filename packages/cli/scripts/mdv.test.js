#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  detectInstalledPackageManager,
  resolveUpdateCommand,
  shouldRunUpdateCommand
} from '../bin/mdv-lib.js';

test('shouldRunUpdateCommand only matches first arg update', () => {
  assert.equal(shouldRunUpdateCommand(['update']), true);
  assert.equal(shouldRunUpdateCommand(['update', '--check']), true);
  assert.equal(shouldRunUpdateCommand([]), false);
  assert.equal(shouldRunUpdateCommand(['README.md']), false);
});

test('resolveUpdateCommand prefers npm_execpath node launch when present', () => {
  const cmd = resolveUpdateCommand({
    npm_execpath: '/usr/local/lib/node_modules/npm/bin/npm-cli.js'
  });
  assert.equal(cmd.command, process.execPath);
  assert.equal(cmd.args[0], '/usr/local/lib/node_modules/npm/bin/npm-cli.js');
  assert.deepEqual(cmd.args.slice(1), ['install', '-g', '@dhruv2mars/mdv@latest']);
});

test('resolveUpdateCommand falls back to npm binary', () => {
  const cmd = resolveUpdateCommand({});
  assert.equal(cmd.command, 'npm');
  assert.deepEqual(cmd.args, ['install', '-g', '@dhruv2mars/mdv@latest']);
});

test('resolveUpdateCommand prefers manager from install metadata', () => {
  const root = mkdtempSync(join(tmpdir(), 'mdv-meta-'));
  writeFileSync(
    join(root, 'install-meta.json'),
    JSON.stringify({ packageManager: 'bun', savedAt: '2026-01-01T00:00:00Z' }),
    'utf8'
  );

  const cmd = resolveUpdateCommand({ MDV_INSTALL_ROOT: root });
  assert.equal(cmd.command, 'bun');
  assert.deepEqual(cmd.args, ['add', '-g', '@dhruv2mars/mdv@latest']);
});

test('resolveUpdateCommand prefers env manager when metadata missing', () => {
  const cmd = resolveUpdateCommand({
    npm_config_user_agent: 'pnpm/9.0.0 npm/? node/v22.0.0'
  });
  assert.equal(cmd.command, 'pnpm');
  assert.deepEqual(cmd.args, ['add', '-g', '@dhruv2mars/mdv@latest']);
});

test('detectInstalledPackageManager probes globally installed package', () => {
  const manager = detectInstalledPackageManager((command) => {
    if (command === 'pnpm') return { status: 0, stdout: '@dhruv2mars/mdv 0.0.14' };
    return { status: 1, stdout: '' };
  }, null);
  assert.equal(manager, 'pnpm');
});
