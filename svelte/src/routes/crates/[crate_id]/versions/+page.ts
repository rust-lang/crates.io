import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

const PER_PAGE = 100;

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let sort = url.searchParams.get('sort') ?? 'date';

  let response;
  try {
    response = await client.GET('/api/v1/crates/{name}/versions', {
      params: {
        path: { name: crateName },
        query: { sort, per_page: PER_PAGE, include: 'release_tracks' },
      },
    });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadVersionsError(504);
  }

  let status = response.response.status;
  if (response.error) {
    loadVersionsError(status);
  }

  let { versions, meta } = response.data;
  let releaseTracks: Record<string, { highest: string }> = meta.release_tracks ?? {};

  return {
    sort,
    versions,
    releaseTracks,
    nextPage: meta.next_page ?? null,
  };
}

function loadVersionsError(status: number): never {
  error(status, { message: 'Failed to load versions', tryAgain: true });
}
