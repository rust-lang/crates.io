import { loadCdnBase } from '#lib/utils/cdn.ts';
import { loadManifest } from '#lib/utils/zip-archive.ts';

export async function load({ fetch, params }) {
  let base = await loadCdnBase(fetch);
  let manifest = await loadManifest(fetch, base, params.crate_id, params.version_num);

  return { cdnBase: base, manifest };
}
