#!/usr/bin/env node
import {
  chmodSync,
  copyFileSync,
  createWriteStream,
  existsSync,
  createReadStream,
  mkdirSync,
  readFileSync,
  rmSync,
  renameSync
} from 'node:fs';
import { createHash } from 'node:crypto';
import { homedir } from 'node:os';
import { join } from 'node:path';
import https from 'node:https';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  assetNameFor,
  checksumsAssetNameFor,
  parseChecksumForAsset,
  resolveReleaseAssetBundle,
  shouldUseFallbackUrl
} from './install-lib.js';

const REPO = 'Dhruv2mars/mdv';

const installRoot = process.env.MDV_INSTALL_ROOT || join(homedir(), '.mdv');
const binDir = join(installRoot, 'bin');
const binName = process.platform === 'win32' ? 'mdv.exe' : 'mdv';
const dest = join(binDir, binName);
const retryAttempts = Number(process.env.MDV_INSTALL_RETRY_ATTEMPTS || 3);
const timeoutMs = Number(process.env.MDV_INSTALL_TIMEOUT_MS || 15000);
const backoffMs = Number(process.env.MDV_INSTALL_BACKOFF_MS || 250);

if (process.env.MDV_SKIP_DOWNLOAD === '1') process.exit(0);
if (existsSync(dest)) process.exit(0);

mkdirSync(binDir, { recursive: true });

const version = pkgVersion();
const asset = assetNameFor();
const checksumsAsset = checksumsAssetNameFor();
const url = `https://github.com/${REPO}/releases/download/v${version}/${asset}`;
const checksumsUrl = `https://github.com/${REPO}/releases/download/v${version}/${checksumsAsset}`;
const tmp = `${dest}.tmp-${Date.now()}`;

try {
  console.error(`mdv: download ${asset} v${version}`);
  await downloadWithFallback(
    { binaryUrl: url, checksumsUrl },
    version,
    asset,
    checksumsAsset,
    tmp
  );
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
  return downloadWithRedirects(url, outPath, 0);
}

function downloadWithRedirects(url, outPath, redirects) {
  return new Promise((resolve, reject) => {
    if (redirects > 5) {
      reject(new Error('too many redirects'));
      return;
    }

    const req = https.get(
      url,
      { headers: { 'User-Agent': 'mdv-installer', Accept: 'application/octet-stream' } },
      (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          downloadWithRedirects(res.headers.location, outPath, redirects + 1).then(resolve, reject);
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
        file.on('error', (err) => {
          try {
            rmSync(outPath, { force: true });
          } catch {}
          reject(err);
        });
      }
    );

    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`timeout ${timeoutMs}ms`));
    });
    req.on('error', reject);
  });
}

function requestText(url) {
  return new Promise((resolve, reject) => {
    const req = https.get(
      url,
      { headers: { 'User-Agent': 'mdv-installer', Accept: 'text/plain' } },
      (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          requestText(res.headers.location).then(resolve, reject);
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`http ${res.statusCode}`));
          return;
        }

        let data = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          data += chunk;
        });
        res.on('end', () => resolve(data));
      }
    );

    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`timeout ${timeoutMs}ms`));
    });
    req.on('error', reject);
  });
}

async function withRetry(label, fn) {
  let lastErr = null;
  for (let attempt = 1; attempt <= retryAttempts; attempt += 1) {
    try {
      return await fn();
    } catch (err) {
      lastErr = err;
      if (attempt >= retryAttempts) break;
      await sleep(backoffMs * attempt);
      console.error(`mdv: retry ${label} (${attempt + 1}/${retryAttempts})`);
    }
  }
  throw lastErr || new Error(`retry failed: ${label}`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function sha256File(path) {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    const inStream = createReadStream(path);
    inStream.on('error', reject);
    inStream.on('data', (chunk) => hash.update(chunk));
    inStream.on('end', () => resolve(hash.digest('hex')));
  });
}

async function downloadAndVerify({ binaryUrl, checksumsUrl }, asset, outPath) {
  await withRetry('binary', () => download(binaryUrl, outPath));
  const checksumsText = await withRetry('checksums', () => requestText(checksumsUrl));
  const expected = parseChecksumForAsset(checksumsText, asset);
  if (!expected) {
    throw new Error(`checksum missing for ${asset}`);
  }

  const actual = await sha256File(outPath);
  if (actual !== expected) {
    throw new Error(`checksum mismatch for ${asset}`);
  }
}

async function downloadWithFallback(primary, version, asset, checksumsAsset, outPath) {
  try {
    await downloadAndVerify(primary, asset, outPath);
    return;
  } catch (primaryErr) {
    const fallback = await resolveReleaseAssetBundle({
      version,
      asset,
      checksumsAsset,
      getRelease: (kind) => withRetry(`release:${kind}`, () => getRelease(kind))
    });

    if (!fallback || !shouldUseFallbackUrl(primary.binaryUrl, fallback.binaryUrl)) {
      throw primaryErr;
    }

    console.error(`mdv: fallback download ${fallback.binaryUrl}`);
    await downloadAndVerify(fallback, asset, outPath);
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

    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`timeout ${timeoutMs}ms`));
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
