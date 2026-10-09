import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { isRetryableStatus } from '#lib/utils/api.ts';

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
    let details = response.error?.errors?.[0]?.detail;
    loadError(status, details);
  }

  return { tokens: response.data.api_tokens };
}

function loadError(status: number, details?: string): never {
  error(status, 'Failed to load API tokens', { details, tryAgain: isRetryableStatus(status) });
}
