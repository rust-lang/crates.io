import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

const MAX_PAGES = 20;

export async function load({ fetch, url, params }) {
  let client = createClient({ fetch });

  let categorySlug = params.category_id;
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';

  let category = await loadCategory(client, categorySlug);

  let cratesResponse = await loadCrates(client, categorySlug, {
    category: categorySlug,
    page,
    per_page: perPage,
    sort,
  });

  return { category, cratesResponse, page, perPage, sort, maxPages: MAX_PAGES };
}

function loadCategoryError(slug: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${slug}: Category not found` });
  } else {
    error(status, { message: `${slug}: Failed to load category data`, tryAgain: true });
  }
}

async function loadCategory(client: ReturnType<typeof createClient>, slug: string) {
  let response;
  try {
    response = await client.GET('/api/v1/categories/{category}', { params: { path: { category: slug } } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCategoryError(slug, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCategoryError(slug, status);
  }

  return response.data.category;
}

async function loadCrates(
  client: ReturnType<typeof createClient>,
  slug: string,
  query: paths['/api/v1/crates']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCategoryError(slug, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCategoryError(slug, status);
  }

  return response.data;
}
