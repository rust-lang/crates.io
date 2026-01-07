import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
  let client = createClient({ fetch });

  let categorySlugs = await loadCategorySlugs(client);

  return { categorySlugs };
}

function loadCategorySlugsError(status: number): never {
  error(status, { message: 'Failed to load category slugs', tryAgain: true });
}

async function loadCategorySlugs(client: ReturnType<typeof createClient>) {
  let response;
  try {
    response = await client.GET('/api/v1/category_slugs');
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadCategorySlugsError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCategorySlugsError(status);
  }

  return response.data.category_slugs;
}
