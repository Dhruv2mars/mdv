import { existsSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { packageManagerHintFromEnv, shouldInstallBinary } from './install-lib.js';

const PACKAGE_NAME = '@dhruv2mars/mdv@latest';
const SUPPORTED_PMS = new Set(['bun', 'pnpm', 'yarn', 'npm']);

export { shouldInstallBinary };

export function binNameForPlatform(platform = process.platform) {
  return platform === 'win32' ? 'mdv.exe' : 'mdv';
}

export function resolveInstallRoot(env = process.env, home = homedir()) {
  return env.MDV_INSTALL_ROOT || join(home, '.mdv');
}

export function resolveInstallMetaPath(env = process.env, home = homedir()) {
  return join(resolveInstallRoot(env, home), 'install-meta.json');
}

export function resolveInstalledBin(env = process.env, platform = process.platform, home = homedir()) {
  const installRoot = resolveInstallRoot(env, home);
  const binName = binNameForPlatform(platform);
  return join(installRoot, 'bin', binName);
}

export function shouldRunUpdateCommand(args) {
  return Array.isArray(args) && args.length > 0 && args[0] === 'update';
}

function updateArgsFor(pm) {
  if (pm === 'bun') return ['add', '-g', PACKAGE_NAME];
  if (pm === 'pnpm') return ['add', '-g', PACKAGE_NAME];
  if (pm === 'yarn') return ['global', 'add', PACKAGE_NAME];
  return ['install', '-g', PACKAGE_NAME];
}

export function readInstallMeta(env = process.env, home = homedir()) {
  const path = resolveInstallMetaPath(env, home);
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch {
    return null;
  }
}

export function resolveInstalledVersion(env = process.env, home = homedir()) {
  const version = readInstallMeta(env, home)?.version;
  return typeof version === 'string' && version.length > 0 ? version : null;
}

function isSupportedPm(pm) {
  return typeof pm === 'string' && SUPPORTED_PMS.has(pm);
}

function defaultProbe(command) {
  const args = command === 'bun'
    ? ['pm', 'ls', '-g']
    : command === 'pnpm'
      ? ['list', '-g', '--depth=0']
      : command === 'yarn'
        ? ['global', 'list', '--depth=0']
        : ['list', '-g', '--depth=0'];

  try {
    const res = spawnSync(command, args, { encoding: 'utf8', stdio: 'pipe' });
    return {
      status: res.status ?? 1,
      stdout: String(res.stdout || '')
    };
  } catch {
    return { status: 1, stdout: '' };
  }
}

function pmSearchOrder(preferred) {
  const base = ['bun', 'pnpm', 'yarn', 'npm'];
  if (!isSupportedPm(preferred)) return base;
  return [preferred, ...base.filter((x) => x !== preferred)];
}

export function detectInstalledPackageManager(probe = defaultProbe, preferred = null) {
  for (const command of pmSearchOrder(preferred)) {
    const out = probe(command);
    if ((out?.status ?? 1) !== 0) continue;
    if (String(out?.stdout || '').includes('@dhruv2mars/mdv')) return command;
  }
  return null;
}

export function resolveUpdateCommand(env = process.env) {
  const metaPm = readInstallMeta(env)?.packageManager;
  const envPm = packageManagerHintFromEnv(env);
  const hintPm = isSupportedPm(metaPm) ? metaPm : (isSupportedPm(envPm) ? envPm : null);
  const detectedPm = env === process.env && !hintPm
    ? detectInstalledPackageManager(defaultProbe, null)
    : null;
  const manager = hintPm || detectedPm || 'npm';

  if (manager === 'npm') {
    const npmExecPath = env.npm_execpath;
    if (typeof npmExecPath === 'string' && npmExecPath.endsWith('.js')) {
      return {
        command: process.execPath,
        args: [npmExecPath, ...updateArgsFor('npm')]
      };
    }
  }

  if (isSupportedPm(manager)) {
    return {
      command: manager,
      args: updateArgsFor(manager)
    };
  }

  return {
    command: 'npm',
    args: updateArgsFor('npm')
  };
}
