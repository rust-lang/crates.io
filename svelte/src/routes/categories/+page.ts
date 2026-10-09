import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 100;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let categories = await loadApi(
    client.GET('/api/v1/categories', { params: { query: { page, per_page: perPage, sort } } }),
    { message: 'Failed to load categories' },
  );

  return { categories, page, perPage, sort };
}
