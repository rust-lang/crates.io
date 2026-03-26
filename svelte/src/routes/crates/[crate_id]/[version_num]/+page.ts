import { createClient } from '@crates-io/api-client';

import { loadDocsRsStatus } from '$lib/utils/docs-rs';
import { loadReadme } from '$lib/utils/readme';

export async function load({ fetch, params }) {
  let crateName = params.crate_id;
  let versionNum = params.version_num;

  let readmePromise = loadReadme(fetch, crateName, versionNum);
  let downloadsPromise = loadDownloads(fetch, crateName, versionNum);
  let docsRsStatusPromise = loadDocsRsStatus(fetch, crateName, versionNum);

  return { readmePromise, downloadsPromise, docsRsStatusPromise };
}

/**
 * Loads download data for a specific crate version.
 *
 * This loads the per-day downloads for the last 90 days for that version only.
 */
async function loadDownloads(fetch: typeof globalThis.fetch, name: string, version: string) {
  let client = createClient({ fetch });

  let response = await client.GET('/api/v1/crates/{name}/{version}/downloads', {
    params: { path: { name, version } },
  });

  if (response.error) {
    throw new Error('Failed to load download data');
  }

  return {
    versionDownloads: response.data.version_downloads,
    extraDownloads: [],
  };
}
