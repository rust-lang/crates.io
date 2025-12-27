import { createClient } from '@crates-io/api-client';

export async function load({ fetch, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'crates';

  let response = await client.GET('/api/v1/keywords', {
    params: {
      query: {
        page,
        per_page: perPage,
        sort,
      },
    },
  });

  if (response.error) {
    throw new Error('Failed to fetch keywords');
  }

  return {
    keywords: response.data,
    page,
    perPage,
    sort,
  };
}
