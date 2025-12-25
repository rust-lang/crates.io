import type { operations } from '@crates-io/api-client';

import { browser } from '$app/environment';
import { createClient } from '@crates-io/api-client';

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
  const client = createClient({ fetch });

  return { summary: fetchSummary(client) };
}

async function fetchSummary(client: ReturnType<typeof createClient>): Promise<SummaryResponse> {
  if (browser && cachedSummary) {
    return cachedSummary;
  }

  const response = await client.GET('/api/v1/summary');
  if (response.error) {
    throw new Error('Failed to fetch summary data');
  }

  if (browser) {
    cachedSummary = response.data;
  }

  return response.data;
}
