#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';

import { assetNameFor, resolveReleaseAssetUrl } from '../bin/install-lib.js';

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
