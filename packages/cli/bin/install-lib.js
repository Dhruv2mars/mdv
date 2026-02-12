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
