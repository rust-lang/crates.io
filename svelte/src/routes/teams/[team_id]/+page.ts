import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let login = params.team_id;
  let errorMessages = { message: `${login}: Failed to load team data`, notFound: `${login}: Team not found` };
  let { team } = await loadApi(
    client.GET('/api/v1/teams/{team}', { params: { path: { team: login } } }),
    errorMessages,
  );

  let cratesResponse = await loadApi(
    client.GET('/api/v1/crates', {
      params: { query: { team_id: team.id, page, per_page: perPage, sort, include_yanked: 'n' } },
    }),
    errorMessages,
  );

  return { team, cratesResponse, page, perPage, sort };
}
