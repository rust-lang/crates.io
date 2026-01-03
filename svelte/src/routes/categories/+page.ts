import { createClient } from '@crates-io/api-client';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 100;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let response = await client.GET('/api/v1/categories', {
    params: {
      query: {
        page,
        per_page: perPage,
        sort,
      },
    },
  });

  if (response.error) {
    throw new Error('Failed to fetch categories');
  }

  return {
    categories: response.data,
    page,
    perPage,
    sort,
  };
}
