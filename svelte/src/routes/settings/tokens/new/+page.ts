import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { isRetryableStatus } from '#lib/utils/api.ts';

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
    let details = response.error?.errors?.[0]?.detail;
    if (status === 404) {
      error(404, 'Token not found');
    }
    loadError(status, details);
  }

  return { existingToken: response.data.api_token };
}

function loadError(status: number, details?: string): never {
  error(status, 'Failed to load token data', { details, tryAgain: isRetryableStatus(status) });
}
