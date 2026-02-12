#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import {
  detectInstalledPackageManager,
  resolveUpdateCommand,
  shouldInstallBinary,
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

test('shouldInstallBinary handles missing/current/stale install states', () => {
  assert.equal(
    shouldInstallBinary({ binExists: false, installedVersion: null, packageVersion: '0.0.17' }),
    true
  );
  assert.equal(
    shouldInstallBinary({ binExists: true, installedVersion: '0.0.17', packageVersion: '0.0.17' }),
    false
  );
  assert.equal(
    shouldInstallBinary({ binExists: true, installedVersion: '0.0.16', packageVersion: '0.0.17' }),
    true
  );
  assert.equal(
    shouldInstallBinary({ binExists: true, installedVersion: null, packageVersion: '' }),
    false
  );
});

test('launcher install-miss path reports install missing without runtime crash', () => {
  const root = mkdtempSync(join(tmpdir(), 'mdv-launcher-miss-'));
  const launcher = fileURLToPath(new URL('../bin/mdv.js', import.meta.url));
  const res = spawnSync(process.execPath, [launcher], {
    encoding: 'utf8',
    env: {
      ...process.env,
      MDV_INSTALL_ROOT: root,
      MDV_SKIP_DOWNLOAD: '1'
    }
  });

  const output = `${res.stdout || ''}\n${res.stderr || ''}`;
  assert.notEqual(res.status, 0);
  assert.match(output, /mdv: install missing/);
  assert.doesNotMatch(output, /ReferenceError/i);
});

test('launcher rejects stale installed binary when version mismatches', () => {
  if (process.platform === 'win32') return;

  const root = mkdtempSync(join(tmpdir(), 'mdv-launcher-stale-'));
  const bin = join(root, 'bin', 'mdv');
  mkdirSync(join(root, 'bin'), { recursive: true });
  writeFileSync(bin, '#!/bin/sh\necho STALE-BIN\n', 'utf8');
  chmodSync(bin, 0o755);
  writeFileSync(
    join(root, 'install-meta.json'),
    JSON.stringify({ packageManager: 'bun', version: '0.0.1', savedAt: '2026-01-01T00:00:00Z' }),
    'utf8'
  );

  const launcher = fileURLToPath(new URL('../bin/mdv.js', import.meta.url));
  const res = spawnSync(process.execPath, [launcher], {
    encoding: 'utf8',
    env: {
      ...process.env,
      MDV_INSTALL_ROOT: root,
      MDV_SKIP_DOWNLOAD: '1'
    }
  });

  const output = `${res.stdout || ''}\n${res.stderr || ''}`;
  assert.notEqual(res.status, 0);
  assert.match(output, /mdv: install missing/);
  assert.doesNotMatch(output, /STALE-BIN/);
});
