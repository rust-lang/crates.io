import { createClient } from '@crates-io/api-client';

import { loadApi } from '#lib/utils/api.ts';
import { isLoggedIn } from '#lib/utils/session.svelte.ts';

export async function load({ fetch, params, parent, url }) {
  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  let login = params.user_id;
  let errorMessages = { message: `${login}: Failed to load user data`, notFound: `${login}: User not found` };
  let { user, linked_accounts: linkedAccounts } = await loadApi(
    client.GET('/api/v1/users/{user}', {
      params: { path: { user: login }, query: { include: 'linked_accounts' } },
    }),
    errorMessages,
  );

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
    isOwnProfile = currentUser?.id === user.id;
  }

  let cratesResponse = await loadApi(
    client.GET('/api/v1/crates', {
      params: {
        query: { user_id: user.id, page, per_page: perPage, sort, include_yanked: isOwnProfile ? 'yes' : 'n' },
      },
    }),
    errorMessages,
  );

  return { user, linkedAccounts, cratesResponse, page, perPage, sort };
}
