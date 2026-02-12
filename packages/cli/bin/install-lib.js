export function assetNameFor(platform = process.platform, arch = process.arch) {
  const ext = platform === 'win32' ? '.exe' : '';
  return `mdv-${platform}-${arch}${ext}`;
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

export async function resolveReleaseAssetUrl({ version, asset, getRelease }) {
  const tag = `tags/v${version}`;
  try {
    const tagged = await getRelease(tag);
    const taggedUrl = findAssetUrl(tagged, asset);
    if (taggedUrl) return taggedUrl;
  } catch {}

  try {
    const latest = await getRelease('latest');
    const latestUrl = findAssetUrl(latest, asset);
    if (latestUrl) return latestUrl;
  } catch {}

  return null;
}
