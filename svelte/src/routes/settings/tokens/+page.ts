import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
  let client = createClient({ fetch });

  let response;
  try {
    response = await client.GET('/api/v1/me/tokens', {
      params: { query: { expired_days: 30 } },
    });
  } catch {
    loadError(504);
  }

  let status = response.response.status;
  if (!response.data) {
    loadError(status);
  }

  return { tokens: response.data.api_tokens };
}

function loadError(status: number): never {
  error(status, { message: 'Failed to load API tokens', tryAgain: true });
}
