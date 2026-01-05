import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 50;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';
  let letter = url.searchParams.get('letter') ?? undefined;

  let cratesResponse = await loadCrates(client, { page, per_page: perPage, sort, letter });

  return { cratesResponse, page, perPage, sort, letter };
}

function loadCratesError(status: number): never {
  error(status, { message: 'Failed to load crate list', tryAgain: true });
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
