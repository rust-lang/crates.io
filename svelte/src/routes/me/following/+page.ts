import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, parent, url }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, { message: 'This page requires authentication', loginNeeded: true });
  }

  let client = createClient({ fetch });

  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let cratesResponse = await loadCrates(client, { page, per_page: perPage, sort, following: '1' });

  return { cratesResponse, page, perPage, sort };
}

function loadCratesError(status: number): never {
  error(status, { message: 'Failed to load followed crates', tryAgain: true });
}

async function loadCrates(
  client: ReturnType<typeof createClient>,
  query: paths['/api/v1/crates']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
  } catch (_error) {
    loadCratesError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadCratesError(status);
  }

  return response.data;
}
