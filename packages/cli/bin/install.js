#!/usr/bin/env node
import {
  chmodSync,
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync
} from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import https from 'node:https';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const REPO = 'Dhruv2mars/mdv';

const installRoot = process.env.MDV_INSTALL_ROOT || join(homedir(), '.mdv');
const binDir = join(installRoot, 'bin');
const binName = process.platform === 'win32' ? 'mdv.exe' : 'mdv';
const dest = join(binDir, binName);

if (process.env.MDV_SKIP_DOWNLOAD === '1') process.exit(0);
if (existsSync(dest)) process.exit(0);

mkdirSync(binDir, { recursive: true });

const version = pkgVersion();
const asset = assetName();
const url = `https://github.com/${REPO}/releases/download/v${version}/${asset}`;
const tmp = `${dest}.tmp-${Date.now()}`;

try {
  console.error(`mdv: download ${asset} v${version}`);
  await download(url, tmp);
  if (process.platform !== 'win32') chmodSync(tmp, 0o755);
  renameSync(tmp, dest);
  process.exit(0);
} catch (err) {
  try {
    if (existsSync(tmp)) {
      // best-effort cleanup; ignore
    }
  } catch {}

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

function assetName() {
  const platform = process.platform;
  const arch = process.arch;
  const ext = platform === 'win32' ? '.exe' : '';
  return `mdv-${platform}-${arch}${ext}`;
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
