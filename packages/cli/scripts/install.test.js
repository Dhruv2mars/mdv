#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  assetNameFor,
  checksumsAssetNameFor,
  checksumsAssetNameFromBinaryAsset,
  findAssetUrl,
  parseChecksumForAsset,
  resolveReleaseAssetBundle,
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

test('checksumsAssetNameFor maps platform+arch', () => {
  assert.equal(checksumsAssetNameFor('linux', 'x64'), 'checksums-linux-x64.txt');
  assert.equal(checksumsAssetNameFor('darwin', 'arm64'), 'checksums-darwin-arm64.txt');
  assert.equal(checksumsAssetNameFor('win32', 'x64'), 'checksums-win32-x64.txt');
});

test('checksumsAssetNameFromBinaryAsset maps known binary names', () => {
  assert.equal(
    checksumsAssetNameFromBinaryAsset('mdv-linux-x64'),
    'checksums-linux-x64.txt'
  );
  assert.equal(
    checksumsAssetNameFromBinaryAsset('mdv-win32-x64.exe'),
    'checksums-win32-x64.txt'
  );
  assert.equal(checksumsAssetNameFromBinaryAsset('bad-name'), null);
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
          assets: [
            { name: 'mdv-darwin-arm64', browser_download_url: 'https://example.com/v0.0.5' },
            {
              name: 'checksums-darwin-arm64.txt',
              browser_download_url: 'https://example.com/v0.0.5-sum'
            }
          ]
        };
      }
      return {
        assets: [
          { name: 'mdv-darwin-arm64', browser_download_url: 'https://example.com/latest' },
          {
            name: 'checksums-darwin-arm64.txt',
            browser_download_url: 'https://example.com/latest-sum'
          }
        ]
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
          assets: [
            { name: 'mdv-linux-x64', browser_download_url: 'https://example.com/v0.0.4' },
            {
              name: 'checksums-linux-x64.txt',
              browser_download_url: 'https://example.com/v0.0.4-sum'
            }
          ]
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

test('parseChecksumForAsset reads shasum and sha256sum formats', () => {
  const text = [
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  mdv-linux-x64',
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb *mdv-win32-x64.exe'
  ].join('\n');

  assert.equal(
    parseChecksumForAsset(text, 'mdv-linux-x64'),
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
  );
  assert.equal(
    parseChecksumForAsset(text, 'mdv-win32-x64.exe'),
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
  );
  assert.equal(parseChecksumForAsset(text, 'missing'), null);
});

test('resolveReleaseAssetBundle prefers tagged release with checksum', async () => {
  const bundle = await resolveReleaseAssetBundle({
    version: '0.0.9',
    asset: 'mdv-linux-x64',
    checksumsAsset: 'checksums-linux-x64.txt',
    getRelease: async (kind) => {
      if (kind !== 'tags/v0.0.9') throw new Error('unexpected');
      return {
        assets: [
          { name: 'mdv-linux-x64', browser_download_url: 'https://example.com/tag/bin' },
          { name: 'checksums-linux-x64.txt', browser_download_url: 'https://example.com/tag/sum' }
        ]
      };
    }
  });

  assert.deepEqual(bundle, {
    binaryUrl: 'https://example.com/tag/bin',
    checksumsUrl: 'https://example.com/tag/sum'
  });
});

test('resolveReleaseAssetBundle falls back to latest release', async () => {
  const bundle = await resolveReleaseAssetBundle({
    version: '0.0.9',
    asset: 'mdv-linux-x64',
    checksumsAsset: 'checksums-linux-x64.txt',
    getRelease: async (kind) => {
      if (kind === 'tags/v0.0.9') {
        throw new Error('404');
      }
      if (kind === 'latest') {
        return {
          assets: [
            { name: 'mdv-linux-x64', browser_download_url: 'https://example.com/latest/bin' },
            {
              name: 'checksums-linux-x64.txt',
              browser_download_url: 'https://example.com/latest/sum'
            }
          ]
        };
      }
      throw new Error(`unexpected kind: ${kind}`);
    }
  });

  assert.deepEqual(bundle, {
    binaryUrl: 'https://example.com/latest/bin',
    checksumsUrl: 'https://example.com/latest/sum'
  });
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
