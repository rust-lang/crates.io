import { createClient } from '@crates-io/api-client';

import { loadReadme } from '$lib/utils/readme';

export async function load({ fetch, params, parent }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let downloadsPromise = loadDownloads(client, crateName);

  let { crate, defaultVersion } = await parent();
  let readmePromise = loadReadme(fetch, crate.name, defaultVersion.num);

  return { readmePromise, downloadsPromise };
}

/**
 * Loads download data for a crate (all versions).
 *
 * This loads the per-day downloads for the last 90 days for the latest 5
 * versions plus the sum of the rest ("Other").
 */
async function loadDownloads(client: ReturnType<typeof createClient>, name: string) {
  let response = await client.GET('/api/v1/crates/{name}/downloads', {
    params: { path: { name }, query: { include: 'versions' } },
  });

  if (response.error) {
    throw new Error('Failed to load download data');
  }

  let { version_downloads, versions, meta } = response.data;

  return {
    versionDownloads: version_downloads,
    extraDownloads: meta.extra_downloads,
    versions: versions ?? [],
  };
}
