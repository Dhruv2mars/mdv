#!/usr/bin/env node
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const testsDir = fileURLToPath(new URL('.', import.meta.url));
const unit = spawnSync(
  process.execPath,
  [
    '--test',
    join(testsDir, 'install.test.js'),
    join(testsDir, 'release-contract.test.js')
  ],
  {
  stdio: 'inherit',
  env: process.env
}
);
if (unit.status !== 0) process.exit(unit.status ?? 1);

const here = testsDir;
const cliDir = join(here, '..');
const repo = join(cliDir, '..', '..');

const bin = join(
  repo,
  'target',
  'debug',
  process.platform === 'win32' ? 'mdv-cli.exe' : 'mdv-cli'
);

if (!existsSync(bin)) {
  console.error(`mdv cli test: missing rust bin at ${bin}`);
  console.error('run: bun run build');
  process.exit(1);
}

const launcher = join(cliDir, 'bin', 'mdv.js');
const env = { ...process.env, MDV_BIN: bin, MDV_SKIP_DOWNLOAD: '1' };
const res = spawnSync(process.execPath, [launcher, '--help'], { stdio: 'inherit', env });
process.exit(res.status ?? 1);
