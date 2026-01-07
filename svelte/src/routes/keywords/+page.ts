import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'crates';

  let keywords = await loadKeywords(client, { page, per_page: perPage, sort });

  return { keywords, page, perPage, sort };
}

function loadKeywordsError(status: number): never {
  error(status, { message: 'Failed to load keywords', tryAgain: true });
}

async function loadKeywords(
  client: ReturnType<typeof createClient>,
  query: paths['/api/v1/keywords']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/keywords', { params: { query } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadKeywordsError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadKeywordsError(status);
  }

  return response.data;
}
