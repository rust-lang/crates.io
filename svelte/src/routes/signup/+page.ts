import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

/** Loads the details for an unfinished signup. */
export async function load({ fetch }) {
  let client = createClient({ fetch });

  let { signup } = await loadApi(client.GET('/api/private/session/signup'), {
    message: 'Failed to load signup details',
  });

  return { signup };
}
