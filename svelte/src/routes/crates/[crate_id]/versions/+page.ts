import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

const DEFAULT_PER_PAGE = 100;

export async function load({ fetch, params, url, depends }) {
  depends('versions:data');

  let client = createClient({ fetch });

  let crateName = params.crate_id;
  let sort = url.searchParams.get('sort') ?? 'date';
  let perPage = Number(url.searchParams.get('per_page')) || DEFAULT_PER_PAGE;

  let { versions, meta } = await loadApi(
    client.GET('/api/v1/crates/{name}/versions', {
      params: {
        path: { name: crateName },
        query: { sort, per_page: perPage, include: 'release_tracks' },
      },
    }),
    { message: 'Failed to load versions' },
  );
  let releaseTracks = meta.release_tracks ?? {};

  return {
    sort,
    versions,
    releaseTracks,
    nextPage: meta.next_page ?? null,
  };
}
