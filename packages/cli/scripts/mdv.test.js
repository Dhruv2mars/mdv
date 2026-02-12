#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';

import {
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

