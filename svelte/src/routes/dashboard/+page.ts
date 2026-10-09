import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { loadApi } from '#lib/utils/api.ts';

export async function load({ fetch, parent }) {
  let { userPromise } = await parent();
  let user = await userPromise;

  if (!user) {
    error(401, 'This page requires authentication', { loginNeeded: true });
  }

  let client = createClient({ fetch });

  let message = 'Failed to load dashboard data';
  let [{ crates: myCrates }, { crates: myFollowing }, stats, updates] = await Promise.all([
    loadApi(client.GET('/api/v1/crates', { params: { query: { user_id: user.id } } }), { message }),
    loadApi(client.GET('/api/v1/crates', { params: { query: { following: '1' } } }), { message }),
    loadApi(client.GET('/api/v1/users/{id}/stats', { params: { path: { id: user.id } } }), { message }),
    loadApi(client.GET('/api/v1/me/updates'), { message }),
  ]);

  return {
    user,
    myCrates,
    myFollowing,
    totalDownloads: stats.total_downloads,
    updates,
  };
}
