import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;

  let cratePromise = loadCrate(client, crateName);
  let ownersPromise = loadOwners(client, crateName);

  let { crate, categories, keywords, defaultVersion } = await cratePromise;

  return { crate, categories, keywords, defaultVersion, ownersPromise };
}

async function loadOwners(client: ReturnType<typeof createClient>, name: string) {
  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/owners', {
      params: { path: { name } },
    });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCrateError(name, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCrateError(name, status);
  }

  return response.data.users;
}

function loadCrateError(name: string, status: number): never {
  if (status === 404) {
    error(404, { message: `Crate "${name}" not found` });
  } else {
    error(status, { message: `Failed to load crate data`, tryAgain: true });
  }
}

async function loadCrate(client: ReturnType<typeof createClient>, name: string) {
  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}', {
      params: {
        path: { name },
        query: { include: 'keywords,categories,downloads,default_version' },
      },
    });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCrateError(name, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCrateError(name, status);
  }

  let { crate, versions, keywords, categories } = response.data;

  if (versions?.length !== 1 || versions[0].num !== crate.default_version) {
    // Unexpected response structure is treated as `500 Internal Server Error`
    loadCrateError(name, 500);
  }
  return {
    crate: crate,
    categories: categories ?? [],
    keywords: keywords ?? [],
    defaultVersion: versions[0],
  };
}
