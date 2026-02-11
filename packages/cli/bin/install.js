#!/usr/bin/env node
import {
  chmodSync,
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  renameSync
} from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import https from 'node:https';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { assetNameFor, resolveReleaseAssetUrl } from './install-lib.js';

const REPO = 'Dhruv2mars/mdv';

const installRoot = process.env.MDV_INSTALL_ROOT || join(homedir(), '.mdv');
const binDir = join(installRoot, 'bin');
const binName = process.platform === 'win32' ? 'mdv.exe' : 'mdv';
const dest = join(binDir, binName);

if (process.env.MDV_SKIP_DOWNLOAD === '1') process.exit(0);
if (existsSync(dest)) process.exit(0);

mkdirSync(binDir, { recursive: true });

const version = pkgVersion();
const asset = assetNameFor();
const url = `https://github.com/${REPO}/releases/download/v${version}/${asset}`;
const tmp = `${dest}.tmp-${Date.now()}`;

try {
  console.error(`mdv: download ${asset} v${version}`);
  await downloadWithFallback(url, version, asset, tmp);
  if (process.platform !== 'win32') chmodSync(tmp, 0o755);
  renameSync(tmp, dest);
  process.exit(0);
} catch (err) {
  try { rmSync(tmp, { force: true }); } catch {}

  console.error(`mdv: download failed (${String(err)})`);

  if (process.env.MDV_ALLOW_CARGO_FALLBACK === '1') {
    if (cargoInstallFallback()) {
      process.exit(0);
    }
  }

  console.error('mdv: install incomplete. re-run with MDV_ALLOW_CARGO_FALLBACK=1 or wait for GitHub release assets.');
  process.exit(0);
}

function pkgVersion() {
  try {
    const here = fileURLToPath(new URL('.', import.meta.url));
    const p = readFileSync(join(here, '..', 'package.json'), 'utf8');
    return JSON.parse(p).version;
  } catch {
    return process.env.npm_package_version || '0.0.0';
  }
}

function download(url, outPath) {
  return new Promise((resolve, reject) => {
    const req = https.get(
      url,
      { headers: { 'User-Agent': 'mdv-installer', Accept: 'application/octet-stream' } },
      (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          download(res.headers.location, outPath).then(resolve, reject);
          return;
        }

        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`http ${res.statusCode}`));
          return;
        }

        const file = createWriteStream(outPath);
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
        file.on('error', reject);
      }
    );

    req.on('error', reject);
  });
}

async function downloadWithFallback(primaryUrl, version, asset, outPath) {
  try {
    await download(primaryUrl, outPath);
    return;
  } catch (primaryErr) {
    const fallbackUrl = await resolveReleaseAssetUrl({
      version,
      asset,
      getRelease: getRelease
    });

    if (!fallbackUrl || fallbackUrl === primaryUrl) {
      throw primaryErr;
    }

    console.error(`mdv: fallback download ${fallbackUrl}`);
    await download(fallbackUrl, outPath);
  }
}

async function getRelease(kind) {
  const base = `https://api.github.com/repos/${REPO}/releases`;
  return requestJson(`${base}/${kind}`);
}

function requestJson(url) {
  return new Promise((resolve, reject) => {
    const headers = {
      'User-Agent': 'mdv-installer',
      Accept: 'application/vnd.github+json'
    };

    if (process.env.GITHUB_TOKEN) {
      headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    }

    const req = https.get(url, { headers }, (res) => {
      let data = '';
      res.setEncoding('utf8');
      res.on('data', (chunk) => {
        data += chunk;
      });
      res.on('end', () => {
        if ((res.statusCode || 500) >= 400) {
          const err = new Error(`http ${res.statusCode}`);
          err.status = res.statusCode;
          reject(err);
          return;
        }
        try {
          resolve(JSON.parse(data || '{}'));
        } catch (err) {
          reject(err);
        }
      });
    });

    req.on('error', reject);
  });
}

function cargoInstallFallback() {
  const probe = spawnSync('cargo', ['--version'], { stdio: 'ignore' });
  if (probe.status !== 0) return false;

  console.error('mdv: cargo fallback install...');

  const root = installRoot;
  const install = spawnSync(
    'cargo',
    [
      'install',
      'mdv-cli',
      '--git',
      `https://github.com/${REPO}.git`,
      '--locked',
      '--root',
      root,
      '--config',
      'net.git-fetch-with-cli=true'
    ],
    { stdio: 'inherit', env: { ...process.env, CARGO_NET_GIT_FETCH_WITH_CLI: 'true' } }
  );

  if (install.status !== 0) return false;

  const built = join(root, 'bin', process.platform === 'win32' ? 'mdv-cli.exe' : 'mdv-cli');
  if (!existsSync(built)) return false;

  try {
    copyFileSync(built, dest);
    if (process.platform !== 'win32') chmodSync(dest, 0o755);
  } catch {
    return false;
  }

  return true;
}
