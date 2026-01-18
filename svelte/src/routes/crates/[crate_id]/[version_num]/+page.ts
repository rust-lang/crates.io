import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadReadme } from '$lib/utils/readme';

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let versionNum = params.version_num;

  // Start all requests in parallel
  let versionPromise = loadVersion(client, crateName, versionNum);
  let readmePromise = loadReadme(fetch, crateName, versionNum);
  let downloadsPromise = loadDownloads(fetch, crateName, versionNum);

  return {
    requestedVersion: versionNum,
    version: await versionPromise,
    readmePromise,
    downloadsPromise,
  };
}

async function loadVersion(client: ReturnType<typeof createClient>, name: string, version: string) {
  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/{version}', { params: { path: { name, version } } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadVersionError(name, version, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadVersionError(name, version, status);
  }

  return response.data.version;
}

function loadVersionError(name: string, version: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${name}: Version ${version} not found` });
  } else {
    error(status, { message: `${name}: Failed to load version data`, tryAgain: true });
  }
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
