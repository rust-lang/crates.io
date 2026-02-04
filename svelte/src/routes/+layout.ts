import { createClient } from '@crates-io/api-client';

import { loadPlaygroundCrates } from '$lib/utils/playground';
import { loadUser } from '$lib/utils/session.svelte';

export const ssr = false;

export async function load({ fetch }) {
  let client = createClient({ fetch });

  return {
    playgroundCratesPromise: loadPlaygroundCrates(fetch),
    userPromise: loadUser(client),
  };
}
