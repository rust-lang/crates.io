import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

const MAX_PAGES = 20;

export async function load({ fetch, url, params }) {
  let client = createClient({ fetch });

  let categorySlug = params.category_id;
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';

  let errorMessages = {
    message: `${categorySlug}: Failed to load category data`,
    notFound: `${categorySlug}: Category not found`,
  };
  let { category } = await loadApi(
    client.GET('/api/v1/categories/{category}', { params: { path: { category: categorySlug } } }),
    errorMessages,
  );

  let cratesResponse = await loadApi(
    client.GET('/api/v1/crates', { params: { query: { category: categorySlug, page, per_page: perPage, sort } } }),
    errorMessages,
  );

  return { category, cratesResponse, page, perPage, sort, maxPages: MAX_PAGES };
}
