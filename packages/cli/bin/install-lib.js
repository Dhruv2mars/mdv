export function assetNameFor(platform = process.platform, arch = process.arch) {
  const ext = platform === 'win32' ? '.exe' : '';
  return `mdv-${platform}-${arch}${ext}`;
}

export function findAssetUrl(release, asset) {
  if (!release || !Array.isArray(release.assets)) return null;
  const match = release.assets.find((item) => item?.name === asset);
  if (!match) return null;
  return match.browser_download_url || null;
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
