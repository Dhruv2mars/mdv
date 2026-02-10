#!/usr/bin/env node
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const root = mkdtempSync(join(tmpdir(), 'mdv-install-'));
const env = { ...process.env, MDV_INSTALL_ROOT: root };

const res = spawnSync(process.execPath, [new URL('./install.js', import.meta.url).pathname], {
  stdio: 'inherit',
  env
});

rmSync(root, { recursive: true, force: true });
process.exit(res.status ?? 1);
