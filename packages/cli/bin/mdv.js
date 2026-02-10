#!/usr/bin/env node
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const REPO = 'https://github.com/Dhruv2mars/mdv.git';
const args = process.argv.slice(2);
const envBin = process.env.MDV_BIN;

if (envBin) run(envBin, args);

const installRoot = process.env.MDV_INSTALL_ROOT || join(homedir(), '.mdv');
const binName = process.platform === 'win32' ? 'mdv-cli.exe' : 'mdv-cli';
const installedBin = join(installRoot, 'bin', binName);

if (!existsSync(installedBin)) {
  ensureCargo();
  console.error('mdv: installing rust binary (first run)...');
  const install = spawnSync(
    'cargo',
    [
      'install',
      'mdv-cli',
      '--git',
      REPO,
      '--locked',
      '--root',
      installRoot,
      '--config',
      'net.git-fetch-with-cli=true'
    ],
    {
      stdio: 'inherit',
      env: { ...process.env, CARGO_NET_GIT_FETCH_WITH_CLI: 'true' }
    }
  );

  if (install.status !== 0 || !existsSync(installedBin)) {
    console.error('mdv: install failed');
    process.exit(1);
  }
}

run(installedBin, args);

function ensureCargo() {
  const probe = spawnSync('cargo', ['--version'], { stdio: 'ignore' });
  if (probe.status === 0) return;

  console.error('mdv: rust toolchain not found. install rustup/cargo first.');
  process.exit(1);
}

function run(bin, binArgs) {
  const res = spawnSync(bin, binArgs, { stdio: 'inherit' });
  process.exit(res.status ?? 1);
}
