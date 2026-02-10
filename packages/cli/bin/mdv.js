#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);

const envBin = process.env.MDV_BIN;
if (envBin) {
  run(envBin, args);
}

const here = dirname(fileURLToPath(import.meta.url));
const localDevBin = resolve(here, '../../../target/debug/mdv-cli');
if (existsSync(localDevBin)) {
  run(localDevBin, args);
}

const fallback = spawnSync('cargo', ['run', '-p', 'mdv-cli', '--', ...args], {
  stdio: 'inherit'
});
process.exit(fallback.status ?? 1);

function run(bin, binArgs) {
  const res = spawnSync(bin, binArgs, { stdio: 'inherit' });
  process.exit(res.status ?? 1);
}
