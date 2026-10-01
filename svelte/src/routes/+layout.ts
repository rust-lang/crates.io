import { createClient } from '@crates-io/api-client';

import { loadPlaygroundCrates } from '#lib/utils/playground.ts';
import { loadUser } from '#lib/utils/session.svelte.ts';
import { loadSiteMetadata } from '#lib/utils/site-metadata.ts';

export const ssr = false;

export async function load({ fetch }) {
  let client = createClient({ fetch });

  return {
    playgroundCratesPromise: loadPlaygroundCrates(fetch),
    siteMetadataPromise: loadSiteMetadata(client),
    userPromise: loadUser(client),
  };
}
