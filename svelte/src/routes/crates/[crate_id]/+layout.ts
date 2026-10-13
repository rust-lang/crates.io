import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadApi } from '#lib/utils/api.ts';

// Match Ember.js behavior of accepting trailing slashes on crate URLs.
// This can likely be removed once the Ember.js app is fully replaced.
export const trailingSlash = 'ignore';

export async function load({ fetch, params }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;

  let cratePromise = loadCrate(client, crateName);
  let ownersPromise = loadOwners(client, crateName);

  let { crate, categories, keywords, defaultVersion } = await cratePromise;

  return { crate, categories, keywords, defaultVersion, ownersPromise };
}

/** Returns the error page headings for the crate and owners requests. */
function crateErrorMessages(name: string) {
  return { message: 'Failed to load crate data', notFound: `Crate "${name}" not found` };
}

async function loadOwners(client: ReturnType<typeof createClient>, name: string) {
  let { users } = await loadApi(
    client.GET('/api/v1/crates/{name}/owners', { params: { path: { name } } }),
    crateErrorMessages(name),
  );

  return users;
}

async function loadCrate(client: ReturnType<typeof createClient>, name: string) {
  let { crate, versions, keywords, categories } = await loadApi(
    client.GET('/api/v1/crates/{name}', {
      params: {
        path: { name },
        query: { include: 'keywords,categories,downloads,default_version' },
      },
    }),
    crateErrorMessages(name),
  );

  if (versions?.length !== 1 || versions[0].num !== crate.default_version) {
    // Unexpected response structure is treated as `500 Internal Server Error`
    error(500, 'Failed to load crate data', { tryAgain: true });
  }
  return {
    crate: crate,
    categories: categories ?? [],
    keywords: keywords ?? [],
    defaultVersion: versions[0],
  };
}
