import { createClient } from '@crates-io/api-client';

export async function load({ fetch }) {
  let client = createClient({ fetch });

  let response = await client.GET('/api/v1/category_slugs');

  if (response.error) {
    throw new Error('Failed to fetch category slugs');
  }

  return {
    categorySlugs: response.data.category_slugs,
  };
}
