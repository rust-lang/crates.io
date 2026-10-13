import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch }) {
  let client = createClient({ fetch });

  let { category_slugs: categorySlugs } = await loadApi(client.GET('/api/v1/category_slugs'), {
    message: 'Failed to load category slugs',
  });

  return { categorySlugs };
}
