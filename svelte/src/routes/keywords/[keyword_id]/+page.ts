import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

const MAX_PAGES = 20;

export async function load({ fetch, url, params }) {
  let client = createClient({ fetch });

  let keyword = params.keyword_id;
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';

  let cratesResponse = await loadCrates(client, keyword, { keyword, page, per_page: perPage, sort });

  return { keyword, cratesResponse, page, perPage, sort, maxPages: MAX_PAGES };
}

function loadCratesError(keyword: string, status: number): never {
  error(status, { message: `${keyword}: Failed to load crates`, tryAgain: true });
}

async function loadCrates(
  client: ReturnType<typeof createClient>,
  keyword: string,
  query: paths['/api/v1/crates']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCratesError(keyword, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCratesError(keyword, status);
  }

  return response.data;
}
