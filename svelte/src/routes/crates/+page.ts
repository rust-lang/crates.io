import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 50;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';
  let letter = url.searchParams.get('letter') ?? undefined;

  let cratesResponse = await loadApi(
    client.GET('/api/v1/crates', { params: { query: { page, per_page: perPage, sort, letter } } }),
    { message: 'Failed to load crate list' },
  );

  return { cratesResponse, page, perPage, sort, letter };
}
