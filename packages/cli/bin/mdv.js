#!/usr/bin/env node
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);

const envBin = process.env.MDV_BIN;
if (envBin) run(envBin, args);

const installRoot = process.env.MDV_INSTALL_ROOT || join(homedir(), '.mdv');
const binName = process.platform === 'win32' ? 'mdv.exe' : 'mdv';
const installedBin = join(installRoot, 'bin', binName);

if (!existsSync(installedBin)) {
  const here = fileURLToPath(new URL('.', import.meta.url));
  const installer = join(here, 'install.js');
  const res = spawnSync(process.execPath, [installer], { stdio: 'inherit', env: process.env });
  if (res.status !== 0 || !existsSync(installedBin)) {
    console.error('mdv: install missing. try reinstall: npm i -g @dhruv2mars/mdv');
    process.exit(1);
  }
}

run(installedBin, args);

function run(bin, binArgs) {
  const res = spawnSync(bin, binArgs, { stdio: 'inherit' });
  process.exit(res.status ?? 1);
}
