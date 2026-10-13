import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, url }) {
  let tokenId = url.searchParams.get('from');
  if (!tokenId) return { existingToken: null };

  let client = createClient({ fetch });

  let { api_token: existingToken } = await loadApi(
    client.GET('/api/v1/me/tokens/{id}', { params: { path: { id: Number(tokenId) } } }),
    { message: 'Failed to load token data', notFound: 'Token not found' },
  );

  return { existingToken };
}
