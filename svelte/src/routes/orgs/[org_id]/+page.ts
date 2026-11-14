import type { operations } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';

type CratesResponse = operations['list_crates']['responses']['200']['content']['application/json'];

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let login = params.org_id;
  let errorMessages = { message: `${login}: Failed to load org data`, notFound: `${login}: Org not found` };
  let org = await loadApi(
    client.GET('/api/v1/org/{org}', { params: { path: { org: login } } }),
    errorMessages,
  );

  let teamIds = org.teams.map(t => t.id);
  let cratesResponse: CratesResponse | undefined = undefined;

  if (teamIds?.length > 0) {
    cratesResponse = await loadApi(
      client.GET('/api/v1/crates', {
        params: { query: { team_id: teamIds, page, per_page: perPage, sort, include_yanked: 'n' } },
      }),
      errorMessages,
    );
  } else {
    // If there are no teams, then there are no crates owned by those teams. No need to search.
    cratesResponse = {
      crates: [],
      meta: {
        next_page: null,
        prev_page: null,
        total: 0,
      }
    };
  }

  return { org, login, cratesResponse, page, perPage, sort };
}
