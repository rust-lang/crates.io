import type { operations } from '@crates-io/api-client';

import { browser } from '$app/env';
import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

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

async function loadSummary(client: ReturnType<typeof createClient>): Promise<SummaryResponse> {
  if (browser && cachedSummary) {
    return cachedSummary;
  }

  let summary = await loadApi(client.GET('/api/v1/summary'), { message: 'Failed to load summary data' });

  if (browser) {
    cachedSummary = summary;
  }

  return summary;
}
