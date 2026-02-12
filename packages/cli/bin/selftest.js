#!/usr/bin/env node
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const root = mkdtempSync(join(tmpdir(), 'mdv-install-'));
const env = { ...process.env, MDV_INSTALL_ROOT: root };
const binName = process.platform === 'win32' ? 'mdv.exe' : 'mdv';
const installedBin = join(root, 'bin', binName);

const res = spawnSync(process.execPath, [new URL('./install.js', import.meta.url).pathname], {
  stdio: 'pipe',
  encoding: 'utf8',
  env
});

if ((res.status ?? 1) !== 0) {
  printDiag('installer exited non-zero', { root, status: res.status, stderr: res.stderr });
  rmSync(root, { recursive: true, force: true });
  process.exit(res.status ?? 1);
}

if (!existsSync(installedBin)) {
  printDiag('installed binary missing after installer success', { root, expected: installedBin });
  rmSync(root, { recursive: true, force: true });
  process.exit(1);
}

const smoke = spawnSync(installedBin, ['--help'], {
  stdio: 'pipe',
  encoding: 'utf8',
  env
});
if ((smoke.status ?? 1) !== 0) {
  printDiag('installed binary failed --help', {
    root,
    expected: installedBin,
    status: smoke.status,
    stderr: smoke.stderr
  });
  rmSync(root, { recursive: true, force: true });
  process.exit(smoke.status ?? 1);
}

rmSync(root, { recursive: true, force: true });
process.exit(0);

function printDiag(message, data) {
  console.error(`mdv selftest: ${message}`);
  for (const [k, v] of Object.entries(data)) {
    if (!v) continue;
    console.error(`mdv selftest: ${k}=${String(v).trim()}`);
  }
}
