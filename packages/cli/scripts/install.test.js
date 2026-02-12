#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  assetNameFor,
  findAssetUrl,
  resolveReleaseAssetUrl,
  shouldUseFallbackUrl
} from '../bin/install-lib.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');

test('assetNameFor maps platform+arch', () => {
  assert.equal(assetNameFor('linux', 'x64'), 'mdv-linux-x64');
  assert.equal(assetNameFor('darwin', 'arm64'), 'mdv-darwin-arm64');
  assert.equal(assetNameFor('win32', 'x64'), 'mdv-win32-x64.exe');
});

test('resolveReleaseAssetUrl uses tag release asset first', async () => {
  const calls = [];
  const url = await resolveReleaseAssetUrl({
    version: '0.0.5',
    asset: 'mdv-darwin-arm64',
    getRelease: async (kind) => {
      calls.push(kind);
      if (kind === 'tags/v0.0.5') {
        return {
          assets: [{ name: 'mdv-darwin-arm64', browser_download_url: 'https://example.com/v0.0.5' }]
        };
      }
      return {
        assets: [{ name: 'mdv-darwin-arm64', browser_download_url: 'https://example.com/latest' }]
      };
    }
  });

  assert.equal(url, 'https://example.com/v0.0.5');
  assert.deepEqual(calls, ['tags/v0.0.5']);
});

test('resolveReleaseAssetUrl falls back to latest release asset', async () => {
  const url = await resolveReleaseAssetUrl({
    version: '0.0.5',
    asset: 'mdv-linux-x64',
    getRelease: async (kind) => {
      if (kind === 'tags/v0.0.5') {
        const err = new Error('not found');
        err.status = 404;
        throw err;
      }
      if (kind === 'latest') {
        return {
          assets: [{ name: 'mdv-linux-x64', browser_download_url: 'https://example.com/v0.0.4' }]
        };
      }
      throw new Error(`unexpected kind: ${kind}`);
    }
  });

  assert.equal(url, 'https://example.com/v0.0.4');
});

test('resolveReleaseAssetUrl returns null when no asset found', async () => {
  const url = await resolveReleaseAssetUrl({
    version: '0.0.5',
    asset: 'mdv-linux-x64',
    getRelease: async () => ({ assets: [] })
  });

  assert.equal(url, null);
});

test('findAssetUrl skips unusable matching assets', () => {
  const release = {
    assets: [
      { name: 'mdv-linux-x64', browser_download_url: '' },
      { name: 'mdv-linux-x64' },
      { name: 'mdv-linux-x64', browser_download_url: 'https://example.com/good' }
    ]
  };
  assert.equal(findAssetUrl(release, 'mdv-linux-x64'), 'https://example.com/good');
});

test('resolveReleaseAssetUrl returns null when both release lookups fail', async () => {
  const url = await resolveReleaseAssetUrl({
    version: '0.0.9',
    asset: 'mdv-linux-x64',
    getRelease: async () => {
      throw new Error('network');
    }
  });

  assert.equal(url, null);
});

test('shouldUseFallbackUrl rejects empty/same and accepts different urls', () => {
  assert.equal(shouldUseFallbackUrl('https://a/b', ''), false);
  assert.equal(shouldUseFallbackUrl('https://a/b', null), false);
  assert.equal(shouldUseFallbackUrl('https://a/b', 'https://a/b'), false);
  assert.equal(shouldUseFallbackUrl('https://a/b', 'https://a/c'), true);
});

test('package has minimal user README', () => {
  const readmePath = join(packageRoot, 'README.md');
  assert.equal(existsSync(readmePath), true);
  const text = readFileSync(readmePath, 'utf8');
  assert.match(text, /^# @dhruv2mars\/mdv/m);
  assert.match(text, /^## Install/m);
  assert.match(text, /^## Usage/m);
  assert.match(text, /^## Keybinds/m);
});
