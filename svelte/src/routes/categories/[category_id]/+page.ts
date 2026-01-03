import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

const MAX_PAGES = 20;

export async function load({ fetch, url, params }) {
  let client = createClient({ fetch });

  let categorySlug = params.category_id;
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'recent-downloads';

  let categoryResponse = await client.GET('/api/v1/categories/{category}', {
    params: {
      path: { category: categorySlug },
    },
  });

  if (categoryResponse.error) {
    error(404, { message: `Category "${categorySlug}" not found` });
  }

  let cratesResponse = await client.GET('/api/v1/crates', {
    params: {
      query: {
        category: categorySlug,
        page,
        per_page: perPage,
        sort,
      },
    },
  });

  if (cratesResponse.error) {
    throw new Error(`Failed to fetch crates for category "${categorySlug}"`);
  }

  return {
    category: categoryResponse.data.category,
    crates: cratesResponse.data,
    page,
    perPage,
    sort,
    maxPages: MAX_PAGES,
  };
}
