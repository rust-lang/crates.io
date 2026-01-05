import { createClient } from '@crates-io/api-client';

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let teamResponse = await client.GET('/api/v1/teams/{team}', {
    params: { path: { team: params.team_id } },
  });

  // TODO: implement error handling
  if (teamResponse.error) {
    throw new Error('Failed to fetch team');
  }

  let team = teamResponse.data.team;

  let cratesResponse = await client.GET('/api/v1/crates', {
    params: {
      query: {
        team_id: team.id,
        page,
        per_page: perPage,
        sort,
        include_yanked: 'n',
      },
    },
  });

  // TODO: implement error handling
  if (cratesResponse.error) {
    throw new Error('Failed to fetch crates');
  }

  return {
    team,
    crates: cratesResponse.data,
    page,
    perPage,
    sort,
  };
}
