#!/usr/bin/env node
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  resolveInstalledBin,
  resolveUpdateCommand,
  shouldRunUpdateCommand
} from './mdv-lib.js';

const args = process.argv.slice(2);

if (shouldRunUpdateCommand(args)) {
  const update = resolveUpdateCommand(process.env);
  const res = spawnSync(update.command, update.args, { stdio: 'inherit', env: process.env });
  process.exit(res.status ?? 1);
}

const envBin = process.env.MDV_BIN;
if (envBin) run(envBin, args);

const installedBin = resolveInstalledBin(process.env, process.platform);

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
