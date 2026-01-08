import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { processSearchQuery } from '$lib/utils/search';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let q = url.searchParams.get('q') ?? '';
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'relevance';
  let allKeywords = url.searchParams.get('all_keywords');

  let query = q.trim();

  let searchParams = allKeywords
    ? { page, per_page: perPage, sort, q: query, all_keywords: allKeywords }
    : { page, per_page: perPage, sort, ...processSearchQuery(query) };

  let cratesResponse = await loadCrates(client, searchParams);

  return { q, cratesResponse, page, perPage, sort };
}

function loadCratesError(status: number): never {
  error(status, { message: 'Failed to load search results', tryAgain: true });
}

async function loadCrates(
  client: ReturnType<typeof createClient>,
  query: paths['/api/v1/crates']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCratesError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCratesError(status);
  }

  return response.data;
}
