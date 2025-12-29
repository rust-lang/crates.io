import { createClient } from '@crates-io/api-client';

const MAX_PAGES = 20;

export async function load({ fetch, url, params }) {
  let client = createClient({ fetch });

  let keyword = params.keyword_id;
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';

  let response = await client.GET('/api/v1/crates', {
    params: {
      query: {
        keyword,
        page,
        per_page: perPage,
        sort,
      },
    },
  });

  if (response.error) {
    throw new Error(`Failed to fetch crates for keyword "${keyword}"`);
  }

  return {
    keyword,
    crates: response.data,
    page,
    perPage,
    sort,
    maxPages: MAX_PAGES,
  };
}
