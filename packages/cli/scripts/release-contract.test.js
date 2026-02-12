#!/usr/bin/env node
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { assetNameFor, checksumsAssetNameFor } from '../bin/install-lib.js';

const scriptsDir = fileURLToPath(new URL('.', import.meta.url));
const packageRoot = join(scriptsDir, '..');
const repoRoot = join(packageRoot, '..', '..');
const releaseWorkflow = join(repoRoot, '.github', 'workflows', 'release.yml');

function parseReleaseAssets(workflowText) {
  const includeBlocks = workflowText.match(/-\s+os:[\s\S]*?(?=\n\s*-\s+os:|\n\s*runs-on:|\n\s*steps:)/g) || [];
  const assets = [];
  for (const block of includeBlocks) {
    const platform = block.match(/platform:\s*([a-z0-9]+)/i)?.[1];
    const arch = block.match(/arch:\s*([a-z0-9_]+)/i)?.[1];
    const ext = block.match(/ext:\s*'([^']*)'/)?.[1] ?? '';
    if (!platform || !arch) continue;
    assets.push({ platform, arch, ext, name: `mdv-${platform}-${arch}${ext}` });
  }
  return assets;
}

test('release workflow declares expected installer asset matrix', () => {
  const text = readFileSync(releaseWorkflow, 'utf8');
  const assets = parseReleaseAssets(text);
  const names = new Set(assets.map((x) => x.name));

  const expected = new Set([
    'mdv-linux-x64',
    'mdv-win32-x64.exe',
    'mdv-darwin-arm64',
    'mdv-darwin-x64'
  ]);

  assert.deepEqual(names, expected);
});

test('installer assetNameFor agrees with release matrix', () => {
  const text = readFileSync(releaseWorkflow, 'utf8');
  const assets = parseReleaseAssets(text);

  for (const asset of assets) {
    assert.equal(assetNameFor(asset.platform, asset.arch), asset.name);
  }
});

test('installer checksumsAssetNameFor agrees with release matrix', () => {
  const text = readFileSync(releaseWorkflow, 'utf8');
  const assets = parseReleaseAssets(text);

  for (const asset of assets) {
    assert.equal(
      checksumsAssetNameFor(asset.platform, asset.arch),
      `checksums-${asset.platform}-${asset.arch}.txt`
    );
  }
});

test('release workflow keeps tag and version validation contract', () => {
  const text = readFileSync(releaseWorkflow, 'utf8');
  assert.match(text, /tags:\s*\n\s*-\s*'v\*'/);
  assert.match(text, /Validate tag matches package version/);
});
