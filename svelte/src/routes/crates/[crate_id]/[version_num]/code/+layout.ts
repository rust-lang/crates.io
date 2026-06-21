import { createClient } from '@crates-io/api-client';

import { cdnBase } from '$lib/utils/cdn';
import { loadSiteMetadata } from '$lib/utils/site-metadata';
import { loadManifest } from '$lib/utils/zip-archive';

export async function load({ fetch, params }) {
  let { data } = await loadSiteMetadata(createClient({ fetch }));
  if (!data) {
    throw new Error('Failed to load site metadata');
  }

  let base = cdnBase(data);
  let manifest = await loadManifest(fetch, base, params.crate_id, params.version_num);

  return { cdnBase: base, manifest };
}
