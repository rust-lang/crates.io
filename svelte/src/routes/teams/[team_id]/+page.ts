import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let team = await loadTeam(client, params.team_id);

  let cratesResponse = await loadCrates(client, params.team_id, {
    team_id: team.id,
    page,
    per_page: perPage,
    sort,
    include_yanked: 'n',
  });

  return { team, cratesResponse, page, perPage, sort };
}

function loadTeamError(login: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${login}: Team not found` });
  } else {
    error(status, { message: `${login}: Failed to load team data`, tryAgain: true });
  }
}

async function loadTeam(client: ReturnType<typeof createClient>, login: string) {
  let response;
  try {
    response = await client.GET('/api/v1/teams/{team}', { params: { path: { team: login } } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadTeamError(login, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadTeamError(login, status);
  }

  return response.data.team;
}

async function loadCrates(
  client: ReturnType<typeof createClient>,
  login: string,
  query: paths['/api/v1/crates']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadTeamError(login, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadTeamError(login, status);
  }

  return response.data;
}
