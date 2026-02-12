import { homedir } from 'node:os';
import { join } from 'node:path';

const PACKAGE_NAME = '@dhruv2mars/mdv@latest';

export function binNameForPlatform(platform = process.platform) {
  return platform === 'win32' ? 'mdv.exe' : 'mdv';
}

export function resolveInstallRoot(env = process.env, home = homedir()) {
  return env.MDV_INSTALL_ROOT || join(home, '.mdv');
}

export function resolveInstalledBin(env = process.env, platform = process.platform, home = homedir()) {
  const installRoot = resolveInstallRoot(env, home);
  const binName = binNameForPlatform(platform);
  return join(installRoot, 'bin', binName);
}

export function shouldRunUpdateCommand(args) {
  return Array.isArray(args) && args.length > 0 && args[0] === 'update';
}

export function resolveUpdateCommand(env = process.env) {
  const npmExecPath = env.npm_execpath;
  if (typeof npmExecPath === 'string' && npmExecPath.endsWith('.js')) {
    return {
      command: process.execPath,
      args: [npmExecPath, 'install', '-g', PACKAGE_NAME]
    };
  }

  return {
    command: 'npm',
    args: ['install', '-g', PACKAGE_NAME]
  };
}
