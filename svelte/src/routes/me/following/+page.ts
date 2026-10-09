import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, parent, url }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, 'This page requires authentication', { loginNeeded: true });
  }

  let client = createClient({ fetch });

  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let cratesResponse = await loadApi(
    client.GET('/api/v1/crates', { params: { query: { page, per_page: perPage, sort, following: '1' } } }),
    { message: 'Failed to load followed crates' },
  );

  return { cratesResponse, page, perPage, sort };
}
