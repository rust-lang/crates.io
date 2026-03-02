import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, url }) {
  let tokenId = url.searchParams.get('from');
  if (!tokenId) return { existingToken: null };

  let client = createClient({ fetch });

  let response;
  try {
    response = await client.GET('/api/v1/me/tokens/{id}', {
      params: { path: { id: Number(tokenId) } },
    });
  } catch {
    loadError(504);
  }

  let status = response.response.status;
  if (!response.data) {
    if (status === 404) {
      error(404, { message: 'Token not found' });
    }
    loadError(status);
  }

  return { existingToken: response.data.api_token };
}

function loadError(status: number): never {
  error(status, { message: 'Failed to load token data', tryAgain: true });
}
