import type { operations } from '@crates-io/api-client';

import { browser } from '$app/environment';
import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

type SummaryResponse = operations['get_summary']['responses']['200']['content']['application/json'];

let cachedSummary: SummaryResponse | undefined;

/**
 * Load function to fetch summary data for the page.
 *
 * The summary data is cached on the client side to avoid redundant network requests
 * and is streamed to the page instead of waiting for the entire data to be fetched
 * before rendering.
 */
export async function load({ fetch }) {
  let client = createClient({ fetch });

  return { summary: loadSummary(client) };
}

function loadSummaryError(status: number): never {
  error(status, { message: 'Failed to load summary data', tryAgain: true });
}

async function loadSummary(client: ReturnType<typeof createClient>): Promise<SummaryResponse> {
  if (browser && cachedSummary) {
    return cachedSummary;
  }

  let response;
  try {
    response = await client.GET('/api/v1/summary');
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadSummaryError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadSummaryError(status);
  }

  if (browser) {
    cachedSummary = response.data;
  }

  return response.data;
}
