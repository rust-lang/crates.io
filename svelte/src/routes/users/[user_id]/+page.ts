import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { isLoggedIn } from '$lib/utils/session.svelte';

export async function load({ fetch, params, parent, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let user = await loadUser(client, params.user_id);

  // Check if the logged-in user is viewing their own profile, so that
  // yanked crates are included in the results. We gate the `parent()`
  // call behind `isLoggedIn()` to avoid the overhead of waiting for
  // the `/api/v1/me` request for unauthenticated users.
  //
  // NOTE: `isLoggedIn()` reads from localStorage, which is only
  // available in the browser. This will need to be revisited if SSR
  // is implemented in the future.
  let isOwnProfile = false;
  if (isLoggedIn()) {
    let { userPromise } = await parent();
    let currentUser = await userPromise;
    isOwnProfile = currentUser?.login === params.user_id;
  }

  let cratesResponse = await loadCrates(client, params.user_id, {
    user_id: user.id,
    page,
    per_page: perPage,
    sort,
    include_yanked: isOwnProfile ? 'yes' : 'n',
  });

  return { user, cratesResponse, page, perPage, sort };
}

function loadUserError(login: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${login}: User not found` });
  } else {
    error(status, { message: `${login}: Failed to load user data`, tryAgain: true });
  }
}

async function loadUser(client: ReturnType<typeof createClient>, login: string) {
  let response;
  try {
    response = await client.GET('/api/v1/users/{user}', { params: { path: { user: login } } });
  } catch (_error) {
    // Network errors are treated as `504 Gateway Timeout`
    loadUserError(login, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadUserError(login, status);
  }

  return response.data.user;
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
    loadUserError(login, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadUserError(login, status);
  }

  return response.data;
}
