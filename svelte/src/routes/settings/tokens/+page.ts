import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch }) {
  let client = createClient({ fetch });

  let { api_tokens: tokens } = await loadApi(
    client.GET('/api/v1/me/tokens', { params: { query: { expired_days: 30 } } }),
    { message: 'Failed to load API tokens' },
  );

  return { tokens };
}
