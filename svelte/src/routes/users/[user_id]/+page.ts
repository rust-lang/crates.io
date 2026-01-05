import { createClient } from '@crates-io/api-client';

export async function load({ fetch, params, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let userResponse = await client.GET('/api/v1/users/{user}', {
    params: { path: { user: params.user_id } },
  });

  // TODO: implement error handling
  if (userResponse.error) {
    throw new Error('Failed to fetch user');
  }

  let user = userResponse.data.user;

  // TODO: check if user is current user and conditionally include yanked crates
  let cratesResponse = await client.GET('/api/v1/crates', {
    params: {
      query: {
        user_id: user.id,
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
    user,
    crates: cratesResponse.data,
    page,
    perPage,
    sort,
  };
}
