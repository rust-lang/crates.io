import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { isRetryableStatus } from '#lib/utils/api.ts';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 100;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let categories = await loadCategories(client, { page, per_page: perPage, sort });

  return { categories, page, perPage, sort };
}

function loadCategoriesError(status: number, details?: string): never {
  error(status, 'Failed to load categories', { details, tryAgain: isRetryableStatus(status) });
}

async function loadCategories(
  client: ReturnType<typeof createClient>,
  query: paths['/api/v1/categories']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/categories', { params: { query } });
  } catch {
    // Network errors are treated as `504 Gateway Timeout`
    loadCategoriesError(504);
  }

  let status = response.response.status;
  if (response.error) {
    let details = response.error?.errors?.[0]?.detail;
    loadCategoriesError(status, details);
  }

  return response.data;
}
