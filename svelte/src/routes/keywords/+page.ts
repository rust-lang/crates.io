import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'crates';

  let keywords = await loadApi(
    client.GET('/api/v1/keywords', { params: { query: { page, per_page: perPage, sort } } }),
    { message: 'Failed to load keywords' },
  );

  return { keywords, page, perPage, sort };
}
