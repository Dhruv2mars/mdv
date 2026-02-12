import { join } from 'node:path';

export function assetNameFor(platform = process.platform, arch = process.arch) {
  const ext = platform === 'win32' ? '.exe' : '';
  return `mdv-${platform}-${arch}${ext}`;
}

export function checksumsAssetNameFor(platform = process.platform, arch = process.arch) {
  return `checksums-${platform}-${arch}.txt`;
}

export function checksumsAssetNameFromBinaryAsset(asset) {
  if (typeof asset !== 'string') return null;
  const m = asset.match(/^mdv-([a-z0-9]+)-([a-z0-9_]+)(?:\.exe)?$/i);
  if (!m) return null;
  return checksumsAssetNameFor(m[1], m[2]);
}

export function packageManagerHintFromEnv(env = process.env) {
  const execPath = String(env.npm_execpath || '').toLowerCase();
  if (execPath.includes('bun')) return 'bun';
  if (execPath.includes('pnpm')) return 'pnpm';
  if (execPath.includes('yarn')) return 'yarn';
  if (execPath.includes('npm')) return 'npm';

  const ua = String(env.npm_config_user_agent || '').toLowerCase();
  if (ua.startsWith('bun/')) return 'bun';
  if (ua.startsWith('pnpm/')) return 'pnpm';
  if (ua.startsWith('yarn/')) return 'yarn';
  if (ua.startsWith('npm/')) return 'npm';

  return null;
}

function parseIntEnv(value, fallback, min, max) {
  const parsed = Number.parseInt(String(value ?? ''), 10);
  if (!Number.isFinite(parsed)) return fallback;
  if (parsed < min) return min;
  if (parsed > max) return max;
  return parsed;
}

export function installTuningFromEnv(env = process.env) {
  return {
    retryAttempts: parseIntEnv(env.MDV_INSTALL_RETRY_ATTEMPTS, 3, 1, 10),
    timeoutMs: parseIntEnv(env.MDV_INSTALL_TIMEOUT_MS, 15000, 1000, 120000),
    backoffMs: parseIntEnv(env.MDV_INSTALL_BACKOFF_MS, 250, 50, 5000),
    backoffJitterMs: parseIntEnv(env.MDV_INSTALL_BACKOFF_JITTER_MS, 100, 0, 2000)
  };
}

export function computeBackoffDelay(attempt, backoffMs, backoffJitterMs, rand = Math.random) {
  const scaled = Math.max(1, attempt) * Math.max(0, backoffMs);
  if (backoffJitterMs <= 0) return scaled;
  const jitter = Math.floor(Math.max(0, rand()) * (backoffJitterMs + 1));
  return scaled + jitter;
}

export function cachePathsFor(installRoot, version, asset, checksumsAsset) {
  const root = join(installRoot, 'cache', `v${version}`);
  return {
    cacheDir: root,
    cacheBinary: join(root, asset),
    cacheChecksums: join(root, checksumsAsset)
  };
}

export function buildChecksumMismatchHelp({ asset, expected, actual, cachePath }) {
  const shortExpected = String(expected || '').slice(0, 12);
  const shortActual = String(actual || '').slice(0, 12);
  return [
    `checksum mismatch for ${asset}`,
    `expected=${shortExpected}... actual=${shortActual}...`,
    `clear cache and retry: rm -rf ${cachePath}`
  ].join('; ');
}

export function findAssetUrl(release, asset) {
  if (!release || !Array.isArray(release.assets)) return null;
  for (const item of release.assets) {
    if (item?.name !== asset) continue;
    if (typeof item.browser_download_url === 'string' && item.browser_download_url.length > 0) {
      return item.browser_download_url;
    }
  }
  return null;
}

export function shouldUseFallbackUrl(primaryUrl, fallbackUrl) {
  if (typeof fallbackUrl !== 'string' || fallbackUrl.length === 0) return false;
  if (typeof primaryUrl !== 'string' || primaryUrl.length === 0) return true;
  return fallbackUrl !== primaryUrl;
}

export function parseChecksumForAsset(text, asset) {
  if (typeof text !== 'string' || typeof asset !== 'string' || asset.length === 0) return null;
  for (const line of text.split(/\r?\n/)) {
    const m = line.trim().match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (!m) continue;
    if (m[2].trim() !== asset) continue;
    return m[1].toLowerCase();
  }
  return null;
}

export async function resolveReleaseAssetBundle({ version, asset, checksumsAsset, getRelease }) {
  const tagged = `tags/v${version}`;
  const taggedBundle = await resolveReleaseAssetBundleFromKind({
    kind: tagged,
    asset,
    checksumsAsset,
    getRelease
  });
  if (taggedBundle) return taggedBundle;

  const latestBundle = await resolveReleaseAssetBundleFromKind({
    kind: 'latest',
    asset,
    checksumsAsset,
    getRelease
  });
  if (latestBundle) return latestBundle;

  return null;
}

async function resolveReleaseAssetBundleFromKind({ kind, asset, checksumsAsset, getRelease }) {
  try {
    const release = await getRelease(kind);
    const binaryUrl = findAssetUrl(release, asset);
    const checksumsUrl = findAssetUrl(release, checksumsAsset);
    if (!binaryUrl || !checksumsUrl) return null;
    return { binaryUrl, checksumsUrl };
  } catch {
    return null;
  }
}

export async function resolveReleaseAssetUrl({ version, asset, getRelease }) {
  const checksumsAsset = checksumsAssetNameFromBinaryAsset(asset) || checksumsAssetNameFor();
  const bundle = await resolveReleaseAssetBundle({ version, asset, checksumsAsset, getRelease });
  return bundle?.binaryUrl || null;
}
